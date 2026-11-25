//! Text rendering: a self-contained glyph atlas + instanced textured-quad
//! pipeline, using `ab_glyph` for font loading, shaping and rasterization.
//!
//! Glyphs are rasterized to a monochrome (R8) coverage atlas and drawn with a
//! color tint. No wgpu-version coupling — ab_glyph is independent of wgpu.

use std::collections::HashMap;
use std::path::Path;

use ab_glyph::{Font, FontRef, GlyphId, Point as AbPoint, ScaleFont};
use bytemuck::{as_bytes, cast_slice, Pod, Zeroable};
use wgpu::{
    BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout, BindGroupLayoutDescriptor,
    BindGroupLayoutEntry, BindingResource, BindingType, BlendComponent, BlendFactor, BlendOperation,
    BlendState, Buffer, BufferBindingType, BufferDescriptor, BufferUsages, ColorTargetState,
    ColorWrites, Device, FragmentState, MultisampleState, PipelineLayout, PipelineLayoutDescriptor,
    PrimitiveState, Queue, RenderPipeline, RenderPipelineDescriptor, ShaderModuleDescriptor,
    ShaderSource, ShaderStages, TextureFormat, TextureView, TextureViewDescriptor, VertexAttribute,
    VertexBufferLayout, VertexFormat, VertexState, VertexStepMode,
};

use super::{ClipRect, MAX_CLIPS};

const ATLAS_SIZE: u32 = 1024;
const GLYPH_CELL: u32 = 96;
const CELLS_PER_ROW: u32 = ATLAS_SIZE / GLYPH_CELL;

const TEXT_SHADER: &str = r#"
struct ClipRect { rect: vec4f, radius: f32, enabled: f32, _pad: vec2f }
struct Uniforms { clips: array<ClipRect, 4>, view: vec4f }

@group(0) @binding(0) var t_atlas: texture_2d<f32>;
@group(0) @binding(1) var s_atlas: sampler;
@group(0) @binding(2) var<uniform> u: Uniforms;

struct VSOut {
    @builtin(position) pos: vec4f,
    @location(0) uv: vec2f,
    @location(1) world: vec2f,
    @location(2) color: vec4f,
}

fn sd_rounded_rect(p: vec2f, half: vec2f, r: f32) -> f32 {
    let q = abs(p) - half + r;
    return length(max(q, vec2f(0.0))) + min(max(q.x, q.y), 0.0) - r;
}

@vertex
fn vs(
    @location(0) corner: vec2f,
    @location(1) pos: vec2f,
    @location(2) size: vec2f,
    @location(3) uv_min: vec2f,
    @location(4) uv_max: vec2f,
    @location(5) color: vec4f,
) -> VSOut {
    let world = pos + corner * size;
    let uv = uv_min + corner * (uv_max - uv_min);
    return VSOut { pos: vec4f(world, 0.0, 1.0), uv: uv, world: world, color: color };
}

@fragment
fn fs(in: VSOut) -> @location(0) vec4f {
    let coverage = textureSample(t_atlas, s_atlas, in.uv).r;
    var alpha = coverage * in.color.a;
    let aa_scale = max(u.view.x, u.view.y);
    for (var i = 0; i < 4; i++) {
        let c = u.clips[i];
        if (c.enabled > 0.5) {
            let cp = in.world - c.rect.xy;
            let cd = sd_rounded_rect(cp - c.rect.zw * 0.5, c.rect.zw * 0.5, c.radius);
            let caa = fwidth(cd) * aa_scale;
            alpha *= 1.0 - smoothstep(-caa, caa, cd);
        }
    }
    if (alpha < 0.004) { discard; }
    return vec4f(in.color.rgb, alpha);
}
"#;

#[repr(C)]
#[derive(Clone, Copy, Zeroable, Pod)]
struct TextClipU {
    rect: [f32; 4],
    radius: f32,
    enabled: f32,
    _pad: [f32; 2],
}

#[repr(C)]
#[derive(Clone, Copy, Zeroable, Pod)]
struct TextUniforms {
    clips: [TextClipU; MAX_CLIPS],
    view: [f32; 4],
}

impl Default for TextUniforms {
    fn default() -> Self {
        Self {
            clips: [TextClipU {
                rect: [0.0; 4],
                radius: 0.0,
                enabled: 0.0,
                _pad: [0.0; 2],
            }; MAX_CLIPS],
            view: [1.0, 1.0, 0.0, 0.0],
        }
    }
}

/// A cached glyph's atlas UV rect, pixel size, and pen→top-left offset.
#[derive(Clone, Copy)]
struct GlyphEntry {
    uv_min: [f32; 2],
    uv_max: [f32; 2],
    w: f32,
    h: f32,
    off_x: f32,
    off_y: f32,
}

/// A queued text draw.
#[derive(Clone, Copy)]
pub struct TextOp {
    pub x: f32,
    pub y: f32,
    pub size: f32,
    pub color: [f32; 4],
    pub align: Align,
    pub text: String,
    pub clip_depth: u8,
    pub max_width: f32,
}

#[derive(Clone, Copy, PartialEq)]
pub enum Align {
    Left,
    Center,
    Right,
}

/// Renders text through a glyph atlas.
pub struct TextRenderer {
    device: Device,
    queue: Queue,
    font: FontRef<'static>,
    atlas_texture: wgpu::Texture,
    atlas_view: TextureView,
    sampler: wgpu::Sampler,
    pipeline: RenderPipeline,
    bind_group: BindGroup,
    uniform_buf: Buffer,
    corner_buf: Buffer,
    inst_buf: Buffer,
    cache: HashMap<(GlyphId, u32), GlyphEntry>,
    next_cell: u32,
}

impl TextRenderer {
    /// Load a font. Tries a list of common Windows fonts; falls back to the
    /// first available. `font_bytes` (if given) takes priority. `format` is the
    /// surface color format the pipeline must target.
    pub fn new(
        device: &Device,
        queue: &Queue,
        font_bytes: Option<&[u8]>,
        format: TextureFormat,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let font: FontRef<'static> = match font_bytes {
            Some(bytes) => FontRef::try_from_slice(bytes)
                .map_err(|e| format!("font load failed: {e}"))?,
            None => load_system_font().ok_or("no system font found on this system")?,
        };

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: None,
            address_mode_x: wgpu::AddressMode::ClampToEdge,
            address_mode_y: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        let atlas_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: None,
            size: wgpu::Extent3d {
                width: ATLAS_SIZE,
                height: ATLAS_SIZE,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: TextureFormat::R8Unorm,
            usage: BufferUsages::TEXTURE_BINDING | BufferUsages::COPY_DST,
            view_formats: Vec::new(),
        });
        let atlas_view = atlas_texture.create_view(&TextureViewDescriptor::default());

        let layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: None,
            entries: &[
                BindGroupLayoutEntry {
                    binding: 0,
                    ty: BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    visibility: ShaderStages::FRAGMENT,
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 1,
                    ty: BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    visibility: ShaderStages::FRAGMENT,
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 2,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    visibility: ShaderStages::VERTEX | ShaderStages::FRAGMENT,
                    count: None,
                },
            ],
        });
        let pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[layout],
            push_constant_ranges: &[],
        });

        let shader = device.create_shader_module(ShaderModuleDescriptor {
            label: None,
            source: ShaderSource::Wgsl(std::borrow::Cow::Borrowed(TEXT_SHADER)),
        });

        let blend = BlendState {
            color: BlendComponent {
                src_factor: BlendFactor::SrcAlpha,
                dst_factor: BlendFactor::OneMinusSrcAlpha,
                operation: BlendOperation::Add,
            },
            alpha: BlendComponent {
                src_factor: BlendFactor::One,
                dst_factor: BlendFactor::OneMinusSrcAlpha,
                operation: BlendOperation::Add,
            },
        };

        let pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
            label: None,
            layout: Some(&pipeline_layout),
            vertex: VertexState {
                module: &shader,
                entry_point: "vs",
                buffers: &[
                    VertexBufferLayout {
                        array_stride: 8,
                        step_mode: VertexStepMode::Vertex,
                        attributes: &[VertexAttribute {
                            shader_location: 0,
                            offset: 0,
                            format: VertexFormat::Float32x2,
                        }],
                    },
                    VertexBufferLayout {
                        array_stride: 48,
                        step_mode: VertexStepMode::Instance,
                        attributes: &[
                            VertexAttribute { shader_location: 1, offset: 0, format: VertexFormat::Float32x2 },
                            VertexAttribute { shader_location: 2, offset: 8, format: VertexFormat::Float32x2 },
                            VertexAttribute { shader_location: 3, offset: 16, format: VertexFormat::Float32x2 },
                            VertexAttribute { shader_location: 4, offset: 24, format: VertexFormat::Float32x2 },
                            VertexAttribute { shader_location: 5, offset: 32, format: VertexFormat::Float32x4 },
                        ],
                    },
                ],
            },
            primitive: PrimitiveState::default(),
            depth_stencil: None,
            multisample: MultisampleState::default(),
            fragment: Some(FragmentState {
                module: &shader,
                entry_point: "fs",
                targets: &[ColorTargetState {
                    format,
                    blend: Some(blend),
                    write_mask: ColorWrites::ALL,
                }],
            }),
        });

        let uniform_buf = device.create_buffer(&BufferDescriptor {
            label: None,
            size: std::mem::size_of::<TextUniforms>() as u64,
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let bind_group = device.create_bind_group(&BindGroupDescriptor {
            label: None,
            layout: &layout,
            entries: &[
                BindGroupEntry { binding: 0, resource: BindingResource::TextureView(&atlas_view) },
                BindGroupEntry { binding: 1, resource: BindingResource::Sampler(&sampler) },
                BindGroupEntry { binding: 2, resource: BindingResource::Buffer(uniform_buf.slice(..)) },
            ],
        });

        let corner_data: [f32; 6] = [0.0, 0.0, 1.0, 0.0, 0.0, 1.0];
        let corner_buf = device.create_buffer(&BufferDescriptor {
            label: None,
            size: std::mem::size_of_val(&corner_data) as u64,
            usage: BufferUsages::VERTEX,
            mapped_at_creation: false,
        });
        queue.write_buffer(&corner_buf, 0, cast_slice(&corner_data));

        let inst_buf = device.create_buffer(&BufferDescriptor {
            label: None,
            size: (2048 * 48) as u64,
            usage: BufferUsages::VERTEX | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // Blank the atlas (zero coverage).
        let blank = vec![0u8; (ATLAS_SIZE * ATLAS_SIZE) as usize];
        queue.write_texture(
            wgpu::ImageCopyTexture {
                texture: &atlas_texture,
                mip_level: 0,
                array_layer: 0,
                origin: wgpu::Origin3d::ZERO,
            },
            &blank,
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(ATLAS_SIZE),
                rows_per_image: Some(ATLAS_SIZE),
            },
        );

        Ok(Self {
            device,
            queue,
            font,
            atlas_texture,
            atlas_view,
            sampler,
            pipeline,
            bind_group,
            uniform_buf,
            corner_buf,
            inst_buf,
            cache: HashMap::new(),
            next_cell: 0,
        })
    }

    /// Ensure a glyph is in the atlas (rasterized at position (0,0)); returns
    /// its entry with the pen→top-left offset and pixel size.
    fn ensure_glyph(&mut self, gid: GlyphId, size: f32) -> Option<GlyphEntry> {
        let size_key = (size * 4.0).round() as u32;
        if let Some(e) = self.cache.get(&(gid, size_key)) {
            return Some(*e);
        }
        let glyph = gid.with_scale_and_position(size, AbPoint { x: 0.0, y: 0.0 });
        let outlined = match self.font.outline_glyph(glyph) {
            Some(o) => o,
            None => return None,
        };
        let pb = outlined.px_bounds();
        let w = pb.width().round().max(1.0) as u32;
        let h = pb.height().round().max(1.0) as u32;

        let col = self.next_cell % CELLS_PER_ROW;
        let row = self.next_cell / CELLS_PER_ROW;
        if row >= CELLS_PER_ROW {
            eprintln!("warning: text glyph atlas full");
            return None;
        }
        self.next_cell += 1;

        let mut buf = vec![0u8; (w * h) as usize];
        outlined.draw(|x, y, coverage| {
            if x < w && y < h {
                let idx = (y * w + x) as usize;
                let v = (coverage * 255.0).clamp(0.0, 255.0) as u8;
                buf[idx] = buf[idx].max(v);
            }
        });

        let origin = wgpu::Origin3d {
            x: col * GLYPH_CELL,
            y: row * GLYPH_CELL,
            z: 0,
        };
        self.queue.write_texture(
            wgpu::ImageCopyTexture {
                texture: &self.atlas_texture,
                mip_level: 0,
                array_layer: 0,
                origin,
            },
            &buf,
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(w),
                rows_per_image: Some(h),
            },
        );

        let entry = GlyphEntry {
            uv_min: [
                (col * GLYPH_CELL) as f32 / ATLAS_SIZE as f32,
                (row * GLYPH_CELL) as f32 / ATLAS_SIZE as f32,
            ],
            uv_max: [
                ((col * GLYPH_CELL + w) as f32) / ATLAS_SIZE as f32,
                ((row * GLYPH_CELL + h) as f32) / ATLAS_SIZE as f32,
            ],
            w: w as f32,
            h: h as f32,
            off_x: pb.min.x,
            off_y: pb.min.y,
        };
        self.cache.insert((gid, size_key), entry);
        Some(entry)
    }

    /// Render queued text into the given render pass.
    pub fn render(
        &mut self,
        pass: &mut wgpu::RenderPass<'_>,
        ops: &[TextOp],
        clips: &[ClipRect],
        view: [f32; 4],
    ) {
        if ops.is_empty() {
            return;
        }

        let mut uni = TextUniforms::default();
        for (i, c) in clips.iter().enumerate().take(MAX_CLIPS) {
            uni.clips[i] = TextClipU {
                rect: [c.x, c.y, c.w, c.h],
                radius: c.radius,
                enabled: 1.0,
                _pad: [0.0; 2],
            };
        }
        uni.view = view;
        self.queue
            .write_buffer(&self.uniform_buf, 0, as_bytes(&uni));

        let mut insts: Vec<[f32; 12]> = Vec::new();
        for op in ops {
            let size = op.size;
            // ascent is a plain f32 (the temporary scaled font is dropped).
            let ascent = self.font.as_scaled(size).ascent();

            let chars: Vec<char> = op.text.chars().collect();
            // Total advance for alignment (each scaled font is a temporary).
            let mut total_advance = 0.0f32;
            for &ch in &chars {
                let gid = self.font.glyph_id(ch);
                total_advance += self.font.as_scaled(size).h_advance(gid);
            }
            let start_x = match op.align {
                Align::Left => op.x,
                Align::Center => op.x - total_advance * 0.5,
                Align::Right => op.x - total_advance,
            };

            let baseline_y = op.y + ascent;
            let mut pen_x = start_x;

            for &ch in &chars {
                let gid = self.font.glyph_id(ch);
                let h_adv = self.font.as_scaled(size).h_advance(gid);
                let entry = match self.ensure_glyph(gid, size) {
                    Some(e) => e,
                    None => {
                        pen_x += h_adv;
                        continue;
                    }
                };
                // Quad top-left = pen + (pen→top-left offset).
                let px = pen_x + entry.off_x;
                let py = baseline_y + entry.off_y;
                let mut i = [0.0f32; 12];
                i[0] = px;
                i[1] = py;
                i[2] = entry.w;
                i[3] = entry.h;
                i[4] = entry.uv_min[0];
                i[5] = entry.uv_min[1];
                i[6] = entry.uv_max[0];
                i[7] = entry.uv_max[1];
                i[8..12].copy_from_slice(&op.color);
                insts.push(i);
                pen_x += h_adv;
            }
        }

        if insts.is_empty() {
            return;
        }
        self.queue
            .write_buffer(&self.inst_buf, 0, cast_slice(&insts[..]));
        pass.set_pipeline(&self.pipeline);
        pass.set_vertex_buffer(0, self.corner_buf.slice(..));
        pass.set_vertex_buffer(1, self.inst_buf.slice(..));
        pass.set_bind_group(0, &self.bind_group, &[]);
        pass.draw(0..3, 0..insts.len() as u32);
    }
}

/// Load a system font from common Windows font paths.
fn load_system_font() -> Option<FontRef<'static>> {
    let candidates = [
        "C:\\Windows\\Fonts\\segoeui.ttf",
        "C:\\Windows\\Fonts\\arial.ttf",
        "C:\\Windows\\Fonts\\calibri.ttf",
        "C:\\Windows\\Fonts\\segoeuib.ttf",
    ];
    for path in candidates {
        if let Ok(bytes) = std::fs::read(path) {
            if FontRef::try_from_slice(&bytes).is_ok() {
                let boxed: &'static [u8] = Box::leak(bytes.into_boxed_slice());
                return FontRef::try_from_slice(boxed).ok();
            }
        }
    }
    let dir = Path::new("C:\\Windows\\Fonts");
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) == Some("ttf") {
                if let Ok(bytes) = std::fs::read(&path) {
                    if FontRef::try_from_slice(&bytes).is_ok() {
                        let boxed: &'static [u8] = Box::leak(bytes.into_boxed_slice());
                        return FontRef::try_from_slice(boxed).ok();
                    }
                }
            }
        }
    }
    None
}
