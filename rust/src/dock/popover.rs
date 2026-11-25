//! Side dock popover: an expanded usage detail panel.

use crate::render::shapes::rgba;
use crate::render::{Align, DrawList};
use crate::state::DockState;

/// Draw the popover panel. `progress` is the open/close animation (0..1).
pub fn draw_popover(
    list: &mut DrawList,
    x: f32,
    y: f32,
    w: f32,
    progress: f32,
    dock: &DockState,
) {
    if progress <= 0.001 {
        return;
    }
    let h = 180.0;
    // Slide up + fade in.
    let offset = (1.0 - progress) * 12.0;
    let alpha = progress;

    let panel = rgba(10, 12, 20, 0.97 * alpha);
    let border = rgba(255, 255, 255, 0.10 * alpha);

    list.shadow_rounded_rect(x, y + offset, w, h, 14.0, 0.7 * alpha);
    list.fill_rounded_rect(x, y + offset, w, h, 14.0, panel);
    list.stroke_rounded_rect(x, y + offset, w, h, 14.0, 1.0, border, 12);

    // Header.
    let header_color = rgba(240, 244, 255, alpha);
    list.text(x + 16.0, y + offset + 18.0, 15.0, header_color, Align::Left, "AI Usage");

    // Three usage rows with mini bars.
    let rows = [
        ("Claude", dock.claude, rgba(200, 160, 255, alpha)),
        ("ChatGPT", dock.chatgpt, rgba(120, 200, 255, alpha)),
        ("Gemini", dock.gemini, rgba(150, 220, 180, alpha)),
    ];
    let row_y = y + offset + 44.0;
    for (i, (name, val, col)) in rows.iter().enumerate() {
        let ry = row_y + i as f32 * 42.0;
        let name_color = rgba(210, 215, 235, alpha);
        list.text(x + 16.0, ry, 13.0, name_color, Align::Left, name);
        let pct = (*val / 100.0).clamp(0.0, 1.0);
        let bar_x = x + 16.0;
        let bar_y = ry + 16.0;
        let bar_w = w - 32.0;
        let bar_h = 6.0;
        list.fill_rounded_rect(bar_x, bar_y, bar_w, bar_h, 3.0, rgba(255, 255, 255, 0.10 * alpha));
        list.fill_rounded_rect(bar_x, bar_y, bar_w * pct, bar_h, 3.0, col);
        let pct_color = rgba(230, 235, 255, alpha);
        list.text(x + w - 16.0, ry, 13.0, pct_color, Align::Right, &format!("{:.0}%", val));
    }
}
