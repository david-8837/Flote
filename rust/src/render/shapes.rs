//! Draw-op helpers: solid/gradient fills, rounded-rects, shadows, lines,
//! beziers, arcs and triangles. All coordinates are in stage space (the same
//! 1964x1124 design resolution as the web app).

use crate::render::DrawList;
use crate::render::{FillOp, Grad, Op, Shape, StrokeOp};

/// Convert 0-255 RGBA to normalized color, premultiplying nothing (the shader
/// multiplies alpha on the blend factor).
pub fn rgba(r: u8, g: u8, b: u8, a: f32) -> [f32; 4] {
    [r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0, a]
}

impl DrawList {
    // ----------------------------------------------------------- fills ---

    pub fn fill_rect(
        &mut self,
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        radius: f32,
        color: [f32; 4],
    ) {
        self.push_fill(FillOp {
            x,
            y,
            w,
            h,
            radius,
            color,
            color2: color,
            shape: Shape::Rect,
            grad: Grad::Solid,
            gx: 0.0,
            gy: 0.0,
            edges: [[0.0; 3]; 3],
            clip_depth: self.clip_depth(),
        });
    }

    /// Linear-gradient rounded-rect (colors at the top and bottom of the shape).
    pub fn fill_rect_lin(
        &mut self,
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        color: [f32; 4],
        color2: [f32; 4],
        gx: f32,
        gy: f32,
    ) {
        self.push_fill(FillOp {
            x,
            y,
            w,
            h,
            radius: 0.0,
            color,
            color2,
            shape: Shape::Rect,
            grad: Grad::Linear,
            gx,
            gy,
            edges: [[0.0; 3]; 3],
            clip_depth: self.clip_depth(),
        });
    }

    pub fn fill_rounded_rect(
        &mut self,
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        radius: f32,
        color: [f32; 4],
    ) {
        self.fill_rect(x, y, w, h, radius, color);
    }

    /// Soft shadow: stacked translucent rounded rects with increasing blur
    /// falloff (drawn underneath the shape it casts for).
    pub fn shadow_rounded_rect(
        &mut self,
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        radius: f32,
        alpha: f32,
    ) {
        let layers = 6;
        for i in 0..layers {
            let t = i as f32 / (layers - 1);
            let grow = 2.0 + t * 14.0;
            let a = alpha * (1.0 - t) * 0.13;
            self.fill_rounded_rect(x - grow, y - grow, w + grow * 2.0, h + grow * 2.0, radius + grow, [0.0, 0.0, 0.0, a]);
        }
    }

    pub fn fill_circle(&mut self, x: f32, y: f32, r: f32, color: [f32; 4]) {
        let d = r * 2.0;
        self.push_fill(FillOp {
            x: x - r,
            y: y - r,
            w: d,
            h: d,
            radius: r,
            color,
            color2: color,
            shape: Shape::Circle,
            grad: Grad::Solid,
            gx: 0.0,
            gy: 0.0,
            edges: [[0.0; 3]; 3],
            clip_depth: self.clip_depth(),
        });
    }

    pub fn fill_triangle(&mut self, a: (f32, f32), b: (f32, f32), c: (f32, f32), color: [f32; 4]) {
        let xs = [a.0, b.0, c.0];
        let ys = [a.1, b.1, c.1];
        let min_x = xs[0].min(xs[1]).min(xs[2]);
        let min_y = ys[0].min(ys[1]).min(ys[2]);
        let max_x = xs[0].max(xs[1]).max(xs[2]);
        let max_y = ys[0].max(ys[1]).max(ys[2]);
        let w = (max_x - min_x).max(0.0);
        let h = (max_y - min_y).max(0.0);
        let edges = [
            tri_edge(b, c, (min_x, min_y)),
            tri_edge(c, a, (min_x, min_y)),
            tri_edge(a, b, (min_x, min_y)),
        ];
        self.push_fill(FillOp {
            x: min_x,
            y: min_y,
            w,
            h,
            radius: 0.0,
            color,
            color2: color,
            shape: Shape::Triangle,
            grad: Grad::Solid,
            gx: 0.0,
            gy: 0.0,
            edges,
            clip_depth: self.clip_depth(),
        });
    }

    /// Radial-gradient circle (radial gradient centered at the circle center).
    pub fn fill_circle_grad(&mut self, cx: f32, cy: f32, r: f32, inner: [f32; 4], outer: [f32; 4]) {
        let d = r * 2.0;
        let half = r;
        // Center of the circle in local (0..1) coords.
        let lgx = (half + half) * 0.5 / r * 0.5 + 0.5;
        let lgy = (half + half) * 0.5 / r * 0.5 + 0.5;
        self.push_fill(FillOp {
            x: cx - r,
            y: cy - r,
            w: d,
            h: d,
            radius: r,
            color: inner,
            color2: outer,
            shape: Shape::Circle,
            grad: Grad::Radial,
            gx: lgx,
            gy: lgy,
            edges: [[0.0; 3]; 3],
            clip_depth: self.clip_depth(),
        });
    }

    // ----------------------------------------------------------- strokes ---

    pub fn stroke_line(
        &mut self,
        x0: f32,
        y0: f32,
        x1: f32,
        y1: f32,
        thickness: f32,
        color: [f32; 4],
    ) {
        self.push_stroke(StrokeOp {
            x0,
            y0,
            x1,
            y1,
            thickness,
            color,
            clip_depth: self.clip_depth(),
        });
    }

    /// Quadratic bezier stroke, tessellated into segments on the CPU.
    pub fn stroke_quad_bezier(
        &mut self,
        p0: (f32, f32),
        p1: (f32, f32),
        p2: (f32, f32),
        thickness: f32,
        color: [f32; 4],
        segments: usize,
    ) {
        for i in 0..segments {
            let t0 = i as f32 / segments as f32;
            let t1 = (i + 1) as f32 / segments as f32;
            let a = quad_point(p0, p1, p2, t0);
            let b = quad_point(p0, p1, p2, t1);
            self.stroke_line(a.0, a.1, b.0, b.1, thickness, color);
        }
    }

    /// Cubic bezier stroke, tessellated on the CPU.
    pub fn stroke_cubic_bezier(
        &mut self,
        p0: (f32, f32),
        p1: (f32, f32),
        p2: (f32, f32),
        p3: (f32, f32),
        thickness: f32,
        color: [f32; 4],
        segments: usize,
    ) {
        for i in 0..segments {
            let t0 = i as f32 / segments as f32;
            let t1 = (i + 1) as f32 / segments as f32;
            let a = cubic_point(p0, p1, p2, p3, t0);
            let b = cubic_point(p0, p1, p2, p3, t1);
            self.stroke_line(a.0, a.1, b.0, b.1, thickness, color);
        }
    }

    /// Arc stroke (center c, radius r, start angle..end angle in degrees).
    /// Angles are measured from +x axis, clockwise positive in screen space.
    pub fn stroke_arc(
        &mut self,
        cx: f32,
        cy: f32,
        r: f32,
        start_deg: f32,
        end_deg: f32,
        thickness: f32,
        color: [f32; 4],
        segments: usize,
    ) {
        let mut sweep = end_deg - start_deg;
        while sweep <= 0.0 {
            sweep += 360.0;
        }
        let segs = (segments.max(8) as f32 * sweep.min(360.0) / 360.0) as usize;
        for i in 0..segs {
            let a0 = start_deg + sweep * (i as f32 / segs as f32);
            let a1 = start_deg + sweep * ((i + 1) as f32 / segs as f32);
            let p0 = (cx + r * a0.to_radians().cos(), cy + r * a0.to_radians().sin());
            let p1 = (cx + r * a1.to_radians().cos(), cy + r * a1.to_radians().sin());
            self.stroke_line(p0.0, p0.1, p1.0, p1.1, thickness, color);
        }
    }

    /// Stroke a rounded-rect outline (thick ring).
    pub fn stroke_rounded_rect(
        &mut self,
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        radius: f32,
        thickness: f32,
        color: [f32; 4],
        segments_per_corner: usize,
    ) {
        // Four straight edges.
        let inner_x = x + radius;
        let inner_x2 = x + w - radius;
        let inner_y = y + radius;
        let inner_y2 = y + h - radius;
        self.stroke_line(inner_x, y, inner_x2, y, thickness, color);
        self.stroke_line(inner_x2, y, inner_x2, inner_y, thickness, color);
        self.stroke_line(inner_x2, inner_y, inner_x2, inner_y2, thickness, color);
        self.stroke_line(inner_x2, inner_y2, inner_x, inner_y2, thickness, color);
        self.stroke_line(inner_x, inner_y2, inner_x, inner_y, thickness, color);

        // Four corner arcs.
        let corners = [
            (inner_x, inner_y, radius, 180.0, 270.0),
            (inner_x2, inner_y, radius, 270.0, 360.0),
            (inner_x2, inner_y2, radius, 0.0, 90.0),
            (inner_x, inner_y2, radius, 90.0, 180.0),
        ];
        for (ccx, ccy, cr, a0, a1) in corners {
            self.stroke_arc(ccx, ccy, cr, a0, a1, thickness, color, segments_per_corner);
        }
    }

    // ----------------------------------------------------------- clip ---

    /// Rounded-rect clip region (all subsequent draws inside it are masked).
    pub fn clip_rounded_rect(&mut self, x: f32, y: f32, w: f32, h: f32, radius: f32) {
        self.clip_push(x, y, w, h, radius);
    }

    /// Expose raw clip depth for the caller's own op construction.
    pub(crate) fn push_raw_fill(&mut self, op: FillOp) {
        self.push_fill(op);
    }

    pub(crate) fn push_raw_stroke(&mut self, op: StrokeOp) {
        self.push_stroke(op);
    }
}

// ---------------------------------------------------------------------------
// Small math helpers
// ---------------------------------------------------------------------------

impl Op {
    pub fn is_fill(&self) -> bool {
        matches!(self, Op::Fill(_))
    }
}

fn tri_edge(a: (f32, f32), b: (f32, f32), origin: (f32, f32)) -> [f32; 3] {
    // Line through a->b in absolute coords: n·p = h, normal pointing inward.
    let dx = b.0 - a.0;
    let dy = b.1 - a.1;
    let len = (dx * dx + dy * dy).sqrt().max(1e-6);
    let nx = -dy / len;
    let ny = dx / len;
    let h = nx * a.0 + ny * a.1;
    // Flip the normal so that the origin is inside (n·origin <= h).
    if nx * origin.0 + ny * origin.1 > h {
        [-nx, -ny, -h]
    } else {
        [nx, ny, h]
    }
}

fn quad_point(p0: (f32, f32), p1: (f32, f32), p2: (f32, f32), t: f32) -> (f32, f32) {
    let u = 1.0 - t;
    (
        u * u * p0.0 + 2.0 * u * t * p1.0 + t * t * p2.0,
        u * u * p0.1 + 2.0 * u * t * p1.1 + t * t * p2.1,
    )
}

fn cubic_point(
    p0: (f32, f32),
    p1: (f32, f32),
    p2: (f32, f32),
    p3: (f32, f32),
    t: f32,
) -> (f32, f32) {
    let u = 1.0 - t;
    let w0 = u * u * u;
    let w1 = 3.0 * u * u * t;
    let w2 = 3.0 * u * t * t;
    let w3 = t * t * t;
    (
        w0 * p0.0 + w1 * p1.0 + w2 * p2.0 + w3 * p3.0,
        w0 * p0.1 + w1 * p1.1 + w2 * p2.1 + w3 * p3.1,
    )
}
