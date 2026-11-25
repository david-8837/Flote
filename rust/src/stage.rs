//! Stage geometry: the 1964x1124 design stage scaled to the viewport.

use winit::dpi::PhysicalSize;

/// Design resolution of the laptop screen (15.6" stage).
pub const STAGE_W: f32 = 1964.0;
pub const STAGE_H: f32 = 1124.0;
/// Minimum scale of the stage (same clamp as the web recreation).
pub const MIN_SCALE: f32 = 0.2;
/// Height reserved below the stage (web app uses 190px).
pub const RESERVED_H: f32 = 190.0;

/// Same fit math as the web app: scale the 1964x1124 stage to the viewport
/// (96% width, RESERVED_H reserved below), clamped to [0.2, 1].
pub fn fit_scale(size: PhysicalSize<u32>) -> f32 {
    let w = (size.width as f32 * 0.96) / STAGE_W;
    let h = (size.height as f32 - RESERVED_H) / STAGE_H;
    w.min(h).min(1.0).max(MIN_SCALE)
}

/// The stage rect (physical pixels) for a given scale and window size.
pub fn stage_rect(size: PhysicalSize<u32>, scale: f32) -> (f32, f32, f32, f32) {
    let stage_w = STAGE_W * scale;
    let stage_h = STAGE_H * scale;
    let origin_x = (size.width as f32 - stage_w) * 0.5;
    let origin_y = (size.height as f32 - stage_h) * 0.5;
    (origin_x, origin_y, stage_w, stage_h)
}

/// Center X of the stage in stage coordinates (for the centered notch).
pub fn center_x() -> f32 {
    STAGE_W * 0.5
}
