//! Circular usage gauges for the side dock.

use crate::render::shapes::rgba;
use crate::render::{Align, DrawList};

/// Draw a circular progress gauge.
/// `progress` is 0..100; `r` is the ring radius.
pub fn draw_gauge(
    list: &mut DrawList,
    cx: f32,
    cy: f32,
    r: f32,
    progress: f32,
    color: [f32; 4],
    label: &str,
) {
    let thickness = (r * 0.28).max(2.0);
    let pct = progress.clamp(0.0, 100.0) / 100.0;

    // Track (full faint ring).
    let track = rgba(255, 255, 255, 0.12);
    list.stroke_arc(cx, cy, r, -90.0, 360.0, thickness, track, 48);

    // Progress arc (from top, clockwise).
    if pct > 0.0 {
        let sweep = pct * 360.0;
        list.stroke_arc(cx, cy, r, -90.0, -90.0 + sweep, thickness, color, 48);
    }

    // Center label.
    let text_color = rgba(230, 235, 255, 1.0);
    list.text(cx, cy - 2.0, (r * 0.6).max(8.0), text_color, Align::Center, label);
}
