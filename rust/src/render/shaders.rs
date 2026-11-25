//! WGSL shaders for the render core.
//!
//! Two instanced pipelines:
//! - `FILL_SHADER`: solid / linear-gradient / radial-gradient fills with SDF
//!   anti-aliasing for rounded rects, circles and triangles.
//! - `STROKE_SHADER`: thick anti-aliased lines (segments, beziers, arcs are
//!   tessellated on the CPU into segments).
//!
//! Both share one uniform block: a stack of rounded-rect clip regions and the
//! stage view transform (scale + origin in physical pixels).

pub const FILL_SHADER: &str = r#"
struct ClipRect {
    rect: vec4f,
    radius: f32,
    enabled: f32,
}

struct Uniforms {
    clips: array<ClipRect, 4>,
    view: vec4f, // (scale_x, scale_y, origin_x, origin_y)
}

@group(0) @binding(0) var<uniform> u: Uniforms;

struct VSOut {
    @builtin(position) pos: vec4f,
    @location(0) local: vec2f,
    @location(1) size: vec2f,
    @location(2) color: vec4f,
    @location(3) color2: vec4f,
    @location(4) grad: vec4f,   // (shape, grad_type, gx, gy)
    @location(5) radius: f32,
    @location(6) edges0: vec4f, // triangle edge: (nx, ny, h, 0)
    @location(7) edges1: vec4f,
    @location(8) edges2: vec4f,
    @location(9) world: vec2f,   // absolute stage position
}

fn sd_rounded_rect(p: vec2f, half: vec2f, r: f32) -> f32 {
    let q = abs(p) - half + r;
    return length(max(q, vec2f(0.0))) + min(max(q.x, q.y), 0.0) - r;
}

fn sd_circle(p: vec2f, r: f32) -> f32 {
    return length(p) - r;
}

fn sd_triangle(p: vec2f, e0: vec4f, e1: vec4f, e2: vec4f) -> f32 {
    // Convex polygon SDF: max of the three edge signed distances.
    let d0 = dot(p, e0.xy) - e0.z;
    let d1 = dot(p, e1.xy) - e1.z;
    let d2 = dot(p, e2.xy) - e2.z;
    return max(max(d0, d1), d2);
}

@vertex
fn vs(@location(0) corner: vec2f, @location(1) inst: array<vec4f, 9>) -> VSOut {
    let size = inst[1].xy;
    let pos = inst[0].xy + corner * size;
    return VSOut {
        pos: vec4f(pos, 0.0, 1.0),
        local: corner,
        size: size,
        color: inst[2],
        color2: inst[3],
        grad: inst[4],
        radius: inst[5].x,
        edges0: inst[6],
        edges1: inst[7],
        edges2: inst[8],
        world: pos.xy,
    };
}

@fragment
fn fs(in: VSOut) -> @location(0) vec4f {
    let shape = in.grad.x;
    let topleft = in.world - in.local * in.size;
    let center = topleft + in.size * 0.5;

    var d: f32;
    if (shape > 1.5) {
        d = sd_triangle(in.world, in.edges0, in.edges1, in.edges2);
    } else if (shape > 0.5) {
        d = sd_circle(in.world - center, in.radius);
    } else {
        d = sd_rounded_rect(in.world - center, in.size * 0.5, in.radius);
    }

    // AA in physical pixels: fwidth is in stage units per raster pixel.
    let view = u.view.xy;
    let aa = fwidth(d) * max(view.x, view.y);
    var alpha = 1.0 - smoothstep(-aa, aa, d);

    // Gradient: 0 = solid, 1 = linear (dir in local coords), 2 = radial
    var color = in.color;
    let gtype = in.grad.y;
    if (gtype > 1.5) {
        let c = vec2f(in.grad.z, in.grad.w);
        let t = clamp(length(in.local - c) * 1.6, 0.0, 1.0);
        color = mix(in.color, in.color2, t);
    } else if (gtype > 0.5) {
        let t = clamp(dot(in.local - vec2f(0.5), vec2f(in.grad.z, in.grad.w)) + 0.5, 0.0, 1.0);
        color = mix(in.color, in.color2, t);
    }

    // Rounded-rect clip stack (stage space, absolute coords).
    for (var i = 0; i < 4; i++) {
        let c = u.clips[i];
        if (c.enabled > 0.5) {
            let cp = in.world - c.rect.xy;
            let cd = sd_rounded_rect(cp - c.rect.zw * 0.5, c.rect.zw * 0.5, c.radius);
            let caa = fwidth(cd) * max(view.x, view.y);
            alpha *= 1.0 - smoothstep(-caa, caa, cd);
        }
    }

    if (alpha < 0.004) { discard; }
    return vec4f(color.rgb, color.a * alpha);
}
"#;

pub const STROKE_SHADER: &str = r#"
struct ClipRect {
    rect: vec4f,
    radius: f32,
    enabled: f32,
}

struct Uniforms {
    clips: array<ClipRect, 4>,
    view: vec4f,
}

@group(0) @binding(0) var<uniform> u: Uniforms;

struct VSOut {
    @builtin(position) pos: vec4f,
    @location(0) world: vec2f,
    @location(1) start: vec2f,
    @location(2) end: vec2f,
    @location(3) color: vec4f,
    @location(4) thickness: f32,
}

fn sd_segment(p: vec2f, a: vec2f, b: vec2f) -> f32 {
    let pa = p - a;
    let ba = b - a;
    let h = clamp(dot(pa, ba) / max(dot(ba, ba), 1e-6), 0.0, 1.0);
    return length(pa - ba * h);
}

fn sd_rounded_rect(p: vec2f, half: vec2f, r: f32) -> f32 {
    let q = abs(p) - half + r;
    return length(max(q, vec2f(0.0))) + min(max(q.x, q.y), 0.0) - r;
}

@vertex
fn vs(@location(0) corner: vec2f, @location(1) inst: array<vec4f, 4>) -> VSOut {
    let start = inst[0].xy;
    let end = inst[1].xy;
    let thickness = inst[2].x;
    let dir = end - start;
    let perp = vec2f(-dir.y, dir.x) / max(length(dir), 1e-6) * (thickness * 0.5);
    let world = start + corner.x * dir + corner.y * perp;
    return VSOut {
        pos: vec4f(world, 0.0, 1.0),
        world: world,
        start: start,
        end: end,
        color: vec4f(inst[2].y, inst[2].z, inst[2].w, inst[3].x),
        thickness: thickness,
    };
}

@fragment
fn fs(in: VSOut) -> @location(0) vec4f {
    let d = sd_segment(in.world, in.start, in.end);
    let view = u.view.xy;
    let aa = fwidth(d) * max(view.x, view.y);
    var alpha = 1.0 - smoothstep(in.thickness * 0.5 - aa, in.thickness * 0.5 + aa, d);

    for (var i = 0; i < 4; i++) {
        let c = u.clips[i];
        if (c.enabled > 0.5) {
            let cp = in.world - c.rect.xy;
            let cd = sd_rounded_rect(cp - c.rect.zw * 0.5, c.rect.zw * 0.5, c.radius);
            let caa = fwidth(cd) * max(view.x, view.y);
            alpha *= 1.0 - smoothstep(-caa, caa, cd);
        }
    }

    if (alpha < 0.004) { discard; }
    return vec4f(in.color.rgb, in.color.a * alpha);
}
"#;
