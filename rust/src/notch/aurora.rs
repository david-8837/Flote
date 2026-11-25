//! Aurora resting glow: a soft, animated glow around the resting notch.

use crate::render::shapes::rgba;
use crate::render::DrawList;

/// Draw the aurora glow for the resting notch. `phase` is 0..1 (4.2s cycle),
/// `cx`/`cy` the notch center, `w`/`h` the notch size.
pub fn draw_aurora(list: &mut DrawList, cx: f32, cy: f32, w: f32, h: f32, phase: f32) {
    // Pulsing soft radial glow, shifted slightly for a living feel.
    let pulse = 0.5 + 0.5 * (phase * 6.283).sin();
    let r = (w * 0.9 + 30.0) * (0.85 + 0.15 * pulse);
    let alpha = 0.10 + 0.06 * pulse;

    // Two stacked radial gradients for a soft, layered glow.
    list.fill_circle_grad(cx, cy, r, rgba(150, 170, 255, alpha * 1.4), rgba(120, 140, 250, 0.0));
    list.fill_circle_grad(
        cx - w * 0.2,
        cy + h * 0.1,
        r * 0.7,
        rgba(170, 150, 255, alpha),
        rgba(120, 120, 250, 0.0),
    );
}
