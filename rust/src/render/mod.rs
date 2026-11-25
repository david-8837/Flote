//! GPU render core.
//!
//! Immediate-mode style: the app records draw ops into a [`DrawList`] and the
//! [`Renderer`] flushes them through two instanced wgpu pipelines (fill +
//! stroke) with SDF anti-aliasing, gradients and a rounded-rect clip stack.

pub mod path;
pub mod shaders;
pub mod shapes;

use std::borrow::Cow;

use bytemuck::{cast_slice, Zeroable};
use wgpu::{
    AlphaMode, BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout,
    BindGroupLayoutDescriptor, BindGroupLayoutEntry, BindingResource, BindingType, BlendComponent,
    BlendFactor, BlendOperation, BlendState, Buffer, BufferBindingType, BufferDescriptor,
    BufferUsages, Color, ColorTargetState, ColorWrites, CommandEncoderDescriptor, Device, Features,
    Instance, InstanceDescriptor, MultisampleState, Operations, Backends, PipelineLayout,
    PipelineLayoutDescriptor,
    PowerPreference, PresentMode, Queue, PrimitiveState, RenderPassColorAttachment,
    RenderPassDescriptor, RenderPassLoadOp, RenderPassStoreOp, RenderPipeline,
    RenderPipelineDescriptor, ShaderModuleDescriptor, ShaderSource, ShaderStages, Surface,
    SurfaceConfiguration, TextureFormat, TextureUsages, TextureViewDescriptor, VertexAttribute,
    VertexBufferLayout, VertexFormat, VertexState, VertexStepMode,
};

/// Maximum number of nested rounded-rect clip regions.
pub const MAX_CLIPS: usize = 4;
/// Maximum fill instances per frame.
pub const MAX_FILL_INSTANCES: usize = 4096;
/// Maximum stroke instances per frame.
pub const MAX_STROKE_INSTANCES: usize = 4096;

/// Background color (#050507, same as the web app).
pub const BACKGROUND: Color = Color {
    r: 0.0196,
    g: 0.0196,
    b: 0.0275,
    a: 1.0,
};

// ---------------------------------------------------------------------------
// Draw ops
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, PartialEq)]
pub enum Shape {
    Rect,
    Circle,
    Triangle,
}

#[derive(Clone, Copy, PartialEq)]
pub enum Grad {
    Solid,
    Linear,
    Radial,
}

/// A rounded-rect clip region (stage space).
#[derive(Clone, Copy)]
pub struct ClipRect {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
    pub radius: f32,
}

#[derive(Clone, Copy)]
pub struct FillOp {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
    pub radius: f32,
    pub color: [f32; 4],
    pub color2: [f32; 4],
    pub shape: Shape,
    pub grad: Grad,
    pub gx: f32,
    pub gy: f32,
    /// Triangle edges as (nx, ny, h); only used for Shape::Triangle.
    pub edges: [[f32; 3]; 3],
    pub clip_depth: u8,
}

#[derive(Clone, Copy)]
pub struct StrokeOp {
    pub x0: f32,
    pub y0: f32,
    pub x1: f32,
    pub y1: f32,
    pub thickness: f32,
    pub color: [f32; 4],
    pub clip_depth: u8,
}

#[derive(Clone, Copy)]
pub enum Op {
    Fill(FillOp),
    Stroke(StrokeOp),
    ClipPush(ClipRect),
    ClipPop,
}

/// Recording list of draw ops for one frame.
pub struct DrawList {
    ops: Vec<Op>,
    clip_stack: Vec<ClipRect>,
}

impl DrawList {
    pub fn new() -> Self {
        Self {
            ops: Vec::with_capacity(512),
            clip_stack: Vec::with_capacity(4),
        }
    }

    pub fn clip_depth(&self) -> u8 {
        self.clip_stack.len() as u8
    }

    /// Push a rounded-rect clip region. Ignored (with a warning) past [`MAX_CLIPS`].
    pub fn clip_push(&mut self, x: f32, y: f32, w: f32, h: f32, radius: f32) {
        if self.clip_stack.len() >= MAX_CLIPS {
            eprintln!("warning: clip stack full ({MAX_CLIPS}), ignoring extra clip");
            return;
        }
        let c = ClipRect { x, y, w, h, radius };
        self.clip_stack.push(c);
        self.ops.push(Op::ClipPush(c));
    }

    pub fn clip_pop(&mut self) {
        if self.clip_stack.is_empty() {
            eprintln!("warning: clip_pop with empty clip stack");
            return;
        }
        self.clip_stack.pop();
        self.ops.push(Op::ClipPop);
    }

    pub(crate) fn push_fill(&mut self, op: FillOp) {
        self.ops.push(Op::Fill(op));
    }

    pub(crate) fn push_stroke(&mut self, op: StrokeOp) {
        self.ops.push(Op::Stroke(op));
    }
}

impl Default for DrawList {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Uniforms (must match the WGSL layout exactly)
// ---------------------------------------------------------------------------

#[repr(C)]
#[derive(Clone, Copy, Zeroable)]
struct ClipRectU {
    rect: [f32; 4],
    radius: f32,
    enabled: f32,
    _pad: [f32; 2],
}

#[repr(C)]
#[derive(Clone, Copy, Zeroable)]
struct Uniforms {
    clips: [ClipRectU; MAX_CLIPS],
    view: [f32; 4],
}

impl Default for Uniforms {
    fn default() -> Self {
        Self {
            clips: [ClipRectU {
                rect: [0.0; 4],
                radius: 0.0,
                enabled: 0.0,
                _pad: [0.0; 2],
            }; MAX_CLIPS],
            view: [1.0, 1.0, 0.0, 0.0],
        }
    }
}

// ---------------------------------------------------------------------------
// Renderer
// ---------------------------------------------------------------------------

pub struct Renderer {
    surface: Surface,
    device: Device,
    queue: Queue,
    config: SurfaceConfiguration,
    fill_pipeline: RenderPipeline,
    stroke_pipeline: RenderPipeline,
    uniform_buf: Buffer,
    uniform_bind_group: BindGroup,
    corner_buf: Buffer,
    fill_inst_buf: Buffer,
    stroke_inst_buf: Buffer,
    /// (scale_x, scale_y, origin_x, origin_y) in physical pixels.
    view: [f32; 4],
}

impl Renderer {
    pub async fn new(window: &winit::window::Window) -> Result<Self, Box<dyn std::error::Error>> {
        let instance = Instance::new(&InstanceDescriptor {
            backends: Backends::all(),
            dx12_shader_compiler: wgpu::Dx12Compiler::default(),
        });

        let surface = instance.create_surface(window)?;

        let adapter = instance
            .request_adapter_async(&wgpu::RequestAdapterOptions {
                power_preference: PowerPreference::default(),
                force_fallback_adapter: false,
                compatible_surface: Some(&surface),
            })
            .await
            .ok_or("no suitable GPU adapter found")?;

        let (device, queue) = adapter
            .request_device_async(&wgpu::DeviceDescriptor {
                label: None,
                required_features: Features::empty(),
                required_limits: wgpu::Limits::default(),
                trace_path: None,
            })
            .await?;

        let format = surface
            .get_capabilities(&adapter)
            .formats
            .iter()
            .find(|f| f.is_srgb())
            .copied()
            .unwrap_or(TextureFormat::Bgra8UnormSrgb);

        let size = window.inner_size();
        let config = SurfaceConfiguration {
            usage: TextureUsages::RENDER_ATTACHMENT,
            format,
            alpha_mode: AlphaMode::PREMULTIPLIED,
            present_mode: PresentMode::AutoVsync,
            width: size.width.max(1),
            height: size.height.max(1),
        };
        surface.configure(&device, &config);

        // --- bind group layout / uniform buffer ---
        let bind_group_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: None,
            entries: &[BindGroupLayoutEntry {
                binding: 0,
                ty: BindingType::Buffer {
                    ty: BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                visibility: ShaderStages::VERTEX | ShaderStages::FRAGMENT,
                count: None,
            }],
        });
        let pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[bind_group_layout],
            push_constant_ranges: &[],
        });

        let uniform_size = std::mem::size_of::<Uniforms>() as u64;
        let uniform_buf = device.create_buffer(&BufferDescriptor {
            label: None,
            size: uniform_size,
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let uniform_bind_group = device.create_bind_group(&BindGroupDescriptor {
            label: None,
            layout: &bind_group_layout,
            entries: &[BindGroupEntry {
                binding: 0,
                resource: BindingResource::Buffer(uniform_buf.slice(..)),
            }],
        });

        // --- shaders & pipelines ---
        let fill_shader = device.create_shader_module(ShaderModuleDescriptor {
            label: None,
            source: ShaderSource::Wgsl(Cow::Borrowed(shaders::FILL_SHADER)),
        });
        let stroke_shader = device.create_shader_module(ShaderModuleDescriptor {
            label: None,
            source: ShaderSource::Wgsl(Cow::Borrowed(shaders::STROKE_SHADER)),
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
        let color_target = ColorTargetState {
            format,
            blend: Some(blend),
            write_mask: ColorWrites::ALL,
        };

        // Fill pipeline: corner triangle + 9-vec4 instance (144 B stride).
        let fill_pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
            label: None,
            layout: Some(&pipeline_layout),
            vertex: VertexState {
                module: &fill_shader,
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
                        array_stride: 144,
                        step_mode: VertexStepMode::Instance,
                        attributes: &[
                            VertexAttribute { shader_location: 1, offset: 0, format: VertexFormat::Float32x2 },
                            VertexAttribute { shader_location: 2, offset: 16, format: VertexFormat::Float32x2 },
                            VertexAttribute { shader_location: 3, offset: 32, format: VertexFormat::Float32x4 },
                            VertexAttribute { shader_location: 4, offset: 48, format: VertexFormat::Float32x4 },
                            VertexAttribute { shader_location: 5, offset: 64, format: VertexFormat::Float32x4 },
                            VertexAttribute { shader_location: 6, offset: 80, format: VertexFormat::Float32x1 },
                            VertexAttribute { shader_location: 7, offset: 96, format: VertexFormat::Float32x4 },
                            VertexAttribute { shader_location: 8, offset: 112, format: VertexFormat::Float32x4 },
                            VertexAttribute { shader_location: 9, offset: 128, format: VertexFormat::Float32x4 },
                        ],
                    },
                ],
            },
            primitive: PrimitiveState::default(),
            depth_stencil: None,
            multisample: MultisampleState::default(),
            fragment: Some(wgpu::FragmentState {
                module: &fill_shader,
                entry_point: "fs",
                targets: &[color_target.clone()],
            }),
        });

        // Stroke pipeline: corner triangle + 4-vec4 instance (64 B stride).
        let stroke_pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
            label: None,
            layout: Some(&pipeline_layout),
            vertex: VertexState {
                module: &stroke_shader,
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
                        array_stride: 64,
                        step_mode: VertexStepMode::Instance,
                        attributes: &[
                            VertexAttribute { shader_location: 1, offset: 0, format: VertexFormat::Float32x2 },
                            VertexAttribute { shader_location: 2, offset: 16, format: VertexFormat::Float32x2 },
                            VertexAttribute { shader_location: 3, offset: 32, format: VertexFormat::Float32x4 },
                            VertexAttribute { shader_location: 4, offset: 48, format: VertexFormat::Float32x4 },
                        ],
                    },
                ],
            },
            primitive: PrimitiveState::default(),
            depth_stencil: None,
            multisample: MultisampleState::default(),
            fragment: Some(wgpu::FragmentState {
                module: &stroke_shader,
                entry_point: "fs",
                targets: &[color_target],
            }),
        });

        // --- vertex buffers ---
        // Fullscreen triangle corners: (0,0) (1,0) (0,1).
        let corner_data: [f32; 6] = [0.0, 0.0, 1.0, 0.0, 0.0, 1.0];
        let corner_buf = device.create_buffer(&BufferDescriptor {
            label: None,
            size: std::mem::size_of_val(&corner_data) as u64,
            usage: BufferUsages::VERTEX,
            mapped_at_creation: false,
        });
        queue.write_buffer(&corner_buf, 0, cast_slice(&corner_data));

        let fill_inst_buf = device.create_buffer(&BufferDescriptor {
            label: None,
            size: (MAX_FILL_INSTANCES * 144) as u64,
            usage: BufferUsages::VERTEX | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let stroke_inst_buf = device.create_buffer(&BufferDescriptor {
            label: None,
            size: (MAX_STROKE_INSTANCES * 64) as u64,
            usage: BufferUsages::VERTEX | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        Ok(Self {
            surface,
            device,
            queue,
            config,
            fill_pipeline,
            stroke_pipeline,
            uniform_buf,
            uniform_bind_group,
            corner_buf,
            fill_inst_buf,
            stroke_inst_buf,
            view: [1.0, 1.0, 0.0, 0.0],
        })
    }

    /// Reconfigure the surface for a new window size.
    pub fn resize(&mut self, window: &winit::window::Window) {
        let size = window.inner_size();
        self.config.width = size.width.max(1);
        self.config.height = size.height.max(1);
        self.surface.configure(&self.device, &self.config);
    }

    /// Set the stage view transform: the 1964x1124 stage is drawn at `scale`
    /// with its top-left at `(origin_x, origin_y)` in physical pixels.
    pub fn set_view(&mut self, origin_x: f32, origin_y: f32, scale: f32) {
        self.view = [scale, scale, origin_x, origin_y];
    }

    fn flush(
        &mut self,
        fills: &[[f32; 36]],
        strokes: &[[f32; 16]],
        clips: &[ClipRect],
    ) {
        if fills.is_empty() && strokes.is_empty() {
            return;
        }

        // Uniforms for this batch (clip stack snapshot).
        let mut uni = Uniforms::default();
        for (i, c) in clips.iter().enumerate().take(MAX_CLIPS) {
            uni.clips[i] = ClipRectU {
                rect: [c.x, c.y, c.w, c.h],
                radius: c.radius,
                enabled: 1.0,
                _pad: [0.0; 2],
            };
        }
        uni.view = self.view;
        self.queue
            .write_buffer(&self.uniform_buf, 0, cast_slice(&uni));

        let texture = match self.surface.get_current_texture() {
            Ok(t) => t,
            Err(_) => {
                self.surface.configure(&self.device, &self.config);
                self.surface
                    .get_current_texture()
                    .expect("surface texture unavailable after reconfigure")
            }
        };
        let view = texture
            .texture
            .create_view(&TextureViewDescriptor::default());

        if !fills.is_empty() {
            self.queue
                .write_buffer(&self.fill_inst_buf, 0, cast_slice(fills));
        }
        if !strokes.is_empty() {
            self.queue
                .write_buffer(&self.stroke_inst_buf, 0, cast_slice(strokes));
        }

        let mut encoder = self
            .device
            .create_command_encoder(&CommandEncoderDescriptor { label: None });
        {
            let mut pass = encoder.create_render_pass(&RenderPassDescriptor {
                label: None,
                color_attachments: &[RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: Operations {
                        load: RenderPassLoadOp::Clear(BACKGROUND),
                        store: RenderPassStoreOp::Store,
                    },
                }],
            });

            if !fills.is_empty() {
                pass.set_pipeline(&self.fill_pipeline);
                pass.set_vertex_buffer(0, self.corner_buf.slice(..));
                pass.set_vertex_buffer(1, self.fill_inst_buf.slice(..));
                pass.set_bind_group(0, &self.uniform_bind_group, &[]);
                pass.draw(0..3, 0..fills.len() as u32);
            }
            if !strokes.is_empty() {
                pass.set_pipeline(&self.stroke_pipeline);
                pass.set_vertex_buffer(0, self.corner_buf.slice(..));
                pass.set_vertex_buffer(1, self.stroke_inst_buf.slice(..));
                pass.set_bind_group(0, &self.uniform_bind_group, &[]);
                pass.draw(0..3, 0..strokes.len() as u32);
            }
        }
        self.queue.submit(std::iter::once(encoder.finish()));
    }

    /// Render one frame from a recorded draw list.
    pub fn render(&mut self, list: &DrawList) {
        let mut fill_batch: Vec<[f32; 36]> = Vec::with_capacity(256);
        let mut stroke_batch: Vec<[f32; 16]> = Vec::with_capacity(256);
        // Live clip stack (advanced by ClipPush/ClipPop) and the snapshot of
        // the stack the current batch was built under.
        let mut clip_stack: Vec<ClipRect> = Vec::with_capacity(4);
        let mut batch_clip: Vec<ClipRect> = Vec::with_capacity(4);
        let mut batch_depth: u8 = 0;

        for op in list.ops.iter() {
            match *op {
                Op::ClipPush(c) => {
                    clip_stack.push(c);
                }
                Op::ClipPop => {
                    clip_stack.pop();
                }
                Op::Fill(f) => {
                    if f.clip_depth != batch_depth {
                        self.flush(&fill_batch, &stroke_batch, &batch_clip);
                        fill_batch.clear();
                        stroke_batch.clear();
                        batch_clip.clear();
                        batch_clip.extend_from_slice(&clip_stack);
                        batch_depth = f.clip_depth;
                    }
                    fill_batch.push(fill_instance(&f));
                }
                Op::Stroke(s) => {
                    if s.clip_depth != batch_depth {
                        self.flush(&fill_batch, &stroke_batch, &batch_clip);
                        fill_batch.clear();
                        stroke_batch.clear();
                        batch_clip.clear();
                        batch_clip.extend_from_slice(&clip_stack);
                        batch_depth = s.clip_depth;
                    }
                    stroke_batch.push(stroke_instance(&s));
                }
            }
        }
        self.flush(&fill_batch, &stroke_batch, &batch_clip);
    }
}

// ---------------------------------------------------------------------------
// Instance packing (must match the vertex buffer layouts & shaders)
// ---------------------------------------------------------------------------

fn fill_instance(op: &FillOp) -> [f32; 36] {
    let mut i = [0.0f32; 36];
    // inst[0] pos
    i[0] = op.x;
    i[1] = op.y;
    // inst[1] size
    i[4] = op.w;
    i[5] = op.h;
    // inst[2] color
    i[8..12].copy_from_slice(&op.color);
    // inst[3] color2
    i[12..16].copy_from_slice(&op.color2);
    // inst[4] grad: (shape, grad_type, gx, gy)
    let shape = match op.shape {
        Shape::Rect => 0.0,
        Shape::Circle => 1.0,
        Shape::Triangle => 2.0,
    };
    let gtype = match op.grad {
        Grad::Solid => 0.0,
        Grad::Linear => 1.0,
        Grad::Radial => 2.0,
    };
    i[16] = shape;
    i[17] = gtype;
    i[18] = op.gx;
    i[19] = op.gy;
    // inst[5] radius
    i[20] = op.radius;
    // inst[6..9] triangle edges (nx, ny, h)
    for (k, e) in op.edges.iter().enumerate() {
        i[24 + k * 4] = e[0];
        i[25 + k * 4] = e[1];
        i[26 + k * 4] = e[2];
    }
    i
}

fn stroke_instance(op: &StrokeOp) -> [f32; 16] {
    let mut i = [0.0f32; 16];
    // inst[0] start
    i[0] = op.x0;
    i[1] = op.y0;
    // inst[1] end
    i[4] = op.x1;
    i[5] = op.y1;
    // inst[2] (thickness, r, g, b)
    i[8] = op.thickness;
    i[9] = op.color[0];
    i[10] = op.color[1];
    i[11] = op.color[2];
    // inst[3] (a, 0, 0, 0)
    i[12] = op.color[3];
    i
}
