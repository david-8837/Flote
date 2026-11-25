//! Side dock: the AI usage dock body, gauges, and popover.

pub mod gauges;
pub mod popover;

use crate::config::Settings;
use crate::render::shapes::rgba;
use crate::render::DrawList;
use crate::state::DockState;

/// Draw the side dock (body + gauges + popover).
pub fn draw_dock(list: &mut DrawList, dock: &DockState, settings: &Settings) {
    if !settings.dock_enabled {
        return;
    }

    // Dock body: 56px tall, #050507, radius 22px 0 0 22px (left edge).
    let body_w = 144.0;
    let body_h = 56.0;
    let body_color = rgba(5, 5, 7, settings.transparency);
    let border = rgba(255, 255, 255, 0.08);

    list.shadow_rounded_rect(0.0, 0.0, body_w, body_h, 22.0, 0.6);
    list.fill_rounded_rect(0.0, 0.0, body_w, body_h, 22.0, body_color);
    // Right edge is square; just stroke the top/left rounded outline lightly.
    list.stroke_rounded_rect(0.0, 0.0, body_w, body_h, 22.0, 1.0, border, 12);

    // Three 36px gauges (r=18), centers at y=28.
    let cy = body_h * 0.5;
    let r = 18.0;
    let centers = [36.0, 72.0, 108.0];
    let accent = settings.accent_color;
    let accent_rgba = rgba(accent[0], accent[1], accent[2], 1.0);

    if settings.gauge_claude {
        gauges::draw_gauge(list, centers[0], cy, r, dock.claude, accent_rgba, "C");
    }
    if settings.gauge_chatgpt {
        gauges::draw_gauge(list, centers[1], cy, r, dock.chatgpt, rgba(120, 200, 255, 1.0), "G");
    }
    if settings.gauge_gemini {
        gauges::draw_gauge(list, centers[2], cy, r, dock.gemini, rgba(150, 220, 180, 1.0), "M");
    }

    // Popover (below the dock body).
    popover::draw_popover(list, 0.0, body_h + 8.0, 214.0, dock.open_progress, dock);
}
