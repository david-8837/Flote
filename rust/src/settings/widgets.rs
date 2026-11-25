//! Settings UI widget primitives: headers, labels, toggles, sliders, selectors.

use crate::render::shapes::rgba;
use crate::render::{Align, DrawList};

/// Draw a section header.
pub fn draw_header(list: &mut DrawList, x: f32, y: f32, text: &str, accent: [u8; 3]) {
    let color = rgba(accent[0], accent[1], accent[2], 1.0);
    // Accent bar + label.
    list.fill_rounded_rect(x, y, 3.0, 16.0, 1.5, color);
    let label = rgba(240, 244, 255, 1.0);
    list.text(x + 12.0, y - 2.0, 15.0, label, Align::Left, text);
}

/// Draw a row label + value text.
pub fn draw_label(list: &mut DrawList, x: f32, y: f32, text: &str, value: Option<&str>) {
    let label = rgba(215, 220, 240, 1.0);
    list.text(x, y, 14.0, label, Align::Left, text);
    if let Some(v) = value {
        let val_color = rgba(170, 180, 210, 1.0);
        list.text(x + 220.0, y, 13.0, val_color, Align::Right, v);
    }
}

/// Draw a toggle switch. `on` = state, `accent` = on color.
pub fn draw_toggle(list: &mut DrawList, x: f32, y: f32, on: bool, accent: [u8; 3]) {
    let w = 44.0;
    let h = 24.0;
    let track = rgba(255, 255, 255, if on { 0.35 } else { 0.14 });
    list.fill_rounded_rect(x, y, w, h, h * 0.5, track);
    let knob_color = if on {
        rgba(accent[0], accent[1], accent[2], 1.0)
    } else {
        rgba(230, 235, 255, 1.0)
    };
    let knob_r = (h * 0.5) - 2.0;
    let knob_x = if on { x + w - knob_r - 2.0 } else { x + knob_r + 2.0 };
    let knob_y = y + h * 0.5;
    list.fill_circle(knob_x, knob_y, knob_r, knob_color);
}

/// Draw a slider. `value` is 0..1.
pub fn draw_slider(list: &mut DrawList, x: f32, y: f32, w: f32, value: f32, accent: [u8; 3]) {
    let h = 6.0;
    let v = value.clamp(0.0, 1.0);
    let track = rgba(255, 255, 255, 0.14);
    list.fill_rounded_rect(x, y, w, h, h * 0.5, track);
    let fill = rgba(accent[0], accent[1], accent[2], 1.0);
    list.fill_rounded_rect(x, y, w * v, h, h * 0.5, fill);
    let knob_r = h * 0.9;
    list.fill_circle(x + w * v, y + h * 0.5, knob_r, rgba(240, 244, 255, 1.0));
}

/// Draw a selector pill (click cycles options; current shown).
pub fn draw_selector(list: &mut DrawList, x: f32, y: f32, w: f32, current: &str, accent: [u8; 3]) {
    let h = 28.0;
    let border = rgba(accent[0], accent[1], accent[2], 0.5);
    list.fill_rounded_rect(x, y, w, h, h * 0.4, rgba(255, 255, 255, 0.06));
    list.stroke_rounded_rect(x, y, w, h, h * 0.4, 1.0, border, 10);
    let color = rgba(240, 244, 255, 1.0);
    list.text(x + w * 0.5, y + 6.0, 13.0, color, Align::Center, current);
}
