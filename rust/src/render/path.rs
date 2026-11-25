//! Geometry helpers: bezier and arc tessellation for the stroke pipeline.
//!
//! All helpers return absolute stage-space points; the stroke shader handles
//! SDF anti-aliasing so the segments join smoothly.

/// Evaluate a quadratic bezier at `t` (0..1).
pub fn quad_at(p0: (f32, f32), p1: (f32, f32), p2: (f32, f32), t: f32) -> (f32, f32) {
    let u = 1.0 - t;
    (
        u * u * p0.0 + 2.0 * u * t * p1.0 + t * t * p2.0,
        u * u * p0.1 + 2.0 * u * t * p1.1 + t * t * p2.1,
    )
}

/// Evaluate a cubic bezier at `t` (0..1).
pub fn cubic_at(
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

/// Tessellate a quadratic bezier into `segments` points (inclusive of both ends).
pub fn quad_tessellate(
    p0: (f32, f32),
    p1: (f32, f32),
    p2: (f32, f32),
    segments: usize,
) -> Vec<(f32, f32)> {
    let n = segments.max(1);
    let mut out = Vec::with_capacity(n + 1);
    for i in 0..=n {
        out.push(quad_at(p0, p1, p2, i as f32 / n as f32));
    }
    out
}

/// Tessellate a cubic bezier into `segments` points (inclusive of both ends).
pub fn cubic_tessellate(
    p0: (f32, f32),
    p1: (f32, f32),
    p2: (f32, f32),
    p3: (f32, f32),
    segments: usize,
) -> Vec<(f32, f32)> {
    let n = segments.max(1);
    let mut out = Vec::with_capacity(n + 1);
    for i in 0..=n {
        out.push(cubic_at(p0, p1, p2, p3, i as f32 / n as f32));
    }
    out
}

/// Tessellate an arc (center, radius, start angle, sweep in degrees) into points.
/// Angles are in degrees measured from +x, increasing clockwise in screen coords.
pub fn arc_tessellate(
    cx: f32,
    cy: f32,
    r: f32,
    start_deg: f32,
    sweep_deg: f32,
    segments: usize,
) -> Vec<(f32, f32)> {
    let sweep = sweep_deg.max(0.0);
    let n = segments.max(1);
    let mut out = Vec::with_capacity(n + 1);
    for i in 0..=n {
        let a = start_deg + sweep * (i as f32 / n as f32);
        let rad = a.to_radians();
        out.push((cx + r * rad.cos(), cy + r * rad.sin()));
    }
    out
}

/// Rounded-rect outline points (a closed polyline approximation). Used for
/// dashed/outline strokes where the SDF stroke can't do a ring in one pass.
pub fn rounded_rect_outline(
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    r: f32,
    corner_segments: usize,
) -> Vec<(f32, f32)> {
    let r = r.min(w * 0.5, h * 0.5);
    let mut out = Vec::with_capacity(corner_segments * 4 + 4);
    let cx1 = x + r;
    let cx2 = x + w - r;
    let cy1 = y + r;
    let cy2 = y + h - r;

    // Top-left corner
    for (i, a) in (0..=corner_segments)
        .map(|i| 180.0 + 90.0 * (i as f32 / corner_segments as f32))
        .enumerate()
    {
        let rad = a.to_radians();
        out.push((cx1 + r * rad.cos(), cy1 + r * rad.sin()));
        let _ = i;
    }
    // Top-right corner
    for a in (0..=corner_segments)
        .map(|i| 270.0 + 90.0 * (i as f32 / corner_segments as f32))
    {
        let rad = a.to_radians();
        out.push((cx2 + r * rad.cos(), cy1 + r * rad.sin()));
    }
    // Bottom-right corner
    for a in (0..=corner_segments)
        .map(|i| 0.0 + 90.0 * (i as f32 / corner_segments as f32))
    {
        let rad = a.to_radians();
        out.push((cx2 + r * rad.cos(), cy2 + r * rad.sin()));
    }
    // Bottom-left corner
    for a in (0..=corner_segments)
        .map(|i| 90.0 + 90.0 * (i as f32 / corner_segments as f32))
    {
        let rad = a.to_radians();
        out.push((cx1 + r * rad.cos(), cy2 + r * rad.sin()));
    }
    out
}
