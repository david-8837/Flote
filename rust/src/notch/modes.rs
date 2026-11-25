//! Per-mode notch content: the companion face and mode-specific elements
//! (icons, labels, lyric lines, expanded panel).

use crate::render::shapes::rgba;
use crate::render::{Align, DrawList};
use crate::state::companion::{Companion, Expression};
use crate::state::modes::NotchMode;

use super::aurora;

/// Draw the companion face (eyes, mouth, blush) centered in the notch.
pub fn draw_companion_face(list: &mut DrawList, cx: f32, cy: f32, s: f32, c: &Companion) {
    let eye_off = s * 0.22;
    let eye_r = (s * 0.075).max(1.5);
    let eye_y = cy - s * 0.05 + c.look_y * eye_r;
    let eye_lx = cx - eye_off + c.look_x * eye_r * 0.6;
    let eye_rx = cx + eye_off + c.look_x * eye_r * 0.6;

    let eye_color = rgba(28, 28, 34, 1.0);
    let highlight = rgba(255, 255, 255, 0.85);

    if c.eyes_closed {
        // Closed eyes: gentle arcs, with a small glint so the blink keeps its sparkle.
        list.stroke_arc(eye_lx, eye_y, eye_r, 180.0, 360.0, eye_r * 0.5, eye_color, 8);
        list.stroke_arc(eye_rx, eye_y, eye_r, 180.0, 360.0, eye_r * 0.5, eye_color, 8);
        let glint_r = eye_r * 0.3;
        let glint_y = eye_y - eye_r * 0.2;
        list.fill_circle(eye_lx - eye_r * 0.3, glint_y, glint_r, highlight);
        list.fill_circle(eye_rx - eye_r * 0.3, glint_y, glint_r, highlight);
    } else {
        list.fill_circle(eye_lx, eye_y, eye_r, eye_color);
        list.fill_circle(eye_rx, eye_y, eye_r, eye_color);
        // Highlight.
        list.fill_circle(eye_lx - eye_r * 0.3, eye_y - eye_r * 0.3, eye_r * 0.35, highlight);
        list.fill_circle(eye_rx - eye_r * 0.3, eye_y - eye_r * 0.3, eye_r * 0.35, highlight);
    }

    // Blush.
    if c.blush > 0.01 {
        let blush_a = c.blush * 0.5;
        let blush_r = s * 0.09;
        list.fill_circle(cx - eye_off * 1.4, cy + s * 0.12, blush_r, rgba(255, 140, 170, blush_a));
        list.fill_circle(cx + eye_off * 1.4, cy + s * 0.12, blush_r, rgba(255, 140, 170, blush_a));
    }

    // Mouth.
    let mouth_y = cy + s * 0.22;
    let mouth_w = s * 0.18;
    let open = c.mouth_open;
    let mouth_color = rgba(28, 28, 34, 1.0);
    if open > 0.05 {
        // Open mouth: an ellipse-ish shape (two arcs).
        let r = (mouth_w * 0.5).max(2.0) * (0.6 + open * 0.8);
        list.fill_circle(cx, mouth_y, r * 0.7, mouth_color);
    } else if matches!(c.expression, Expression::Happy) {
        // Smile arc.
        list.stroke_arc(cx, mouth_y, mouth_w, 20.0, 160.0, (mouth_w * 0.12).max(1.5), mouth_color, 12);
    } else {
        // Neutral/flat line.
        list.stroke_line(cx - mouth_w, mouth_y, cx + mouth_w, mouth_y, (mouth_w * 0.12).max(1.5), mouth_color);
    }
}

/// Draw the mode-specific content for a notch (icon, label, lyric, expanded).
pub fn draw_mode_content(
    list: &mut DrawList,
    mode: NotchMode,
    cx: f32,
    cy: f32,
    w: f32,
    h: f32,
    icon_id: u32,
    label: &str,
    aurora_phase: f32,
) {
    match mode {
        NotchMode::Lyrics => {
            // Lyric line: two lines of text centered in the large notch.
            let line1 = "Midnight, the city's asleep";
            let line2 = "Stars are the only ones awake";
            let size = 22.0;
            let color = rgba(230, 235, 255, 1.0);
            list.text(cx, cy - h * 0.28, size, color, Align::Center, line1);
            list.text(cx, cy + h * 0.12, size, color, Align::Center, line2);
        }
        NotchMode::Expanded => {
            // Expanded panel: icon + title + two rows of text.
            let icon_color = rgba(230, 235, 255, 1.0);
            list.icon(icon_id, cx - w * 0.30, cy - 14.0, 28.0, icon_color);
            let title_color = rgba(240, 244, 255, 1.0);
            list.text(cx, cy - h * 0.28, 18.0, title_color, Align::Center, label);
            let sub = rgba(170, 180, 210, 1.0);
            list.text(cx, cy + h * 0.02, 14.0, sub, Align::Center, "Expanded view");
            list.text(cx, cy + h * 0.24, 14.0, sub, Align::Center, "Click to collapse");
        }
        NotchMode::Recording => {
            // Recording: level bars + label.
            draw_level_bars(list, cx, cy, w, h, aurora_phase);
            let color = rgba(255, 120, 120, 1.0);
            list.text(cx, cy + h * 0.26, 12.0, color, Align::Center, "REC");
        }
        // Medium modes with an icon + label.
        NotchMode::Notification
        | NotchMode::Charging
        | NotchMode::Headphones
        | NotchMode::Music
        | NotchMode::Alert
        | NotchMode::Focused => {
            let icon_color = rgba(230, 235, 255, 1.0);
            list.icon(icon_id, cx - w * 0.28, cy - 14.0, 28.0, icon_color);
            let label_color = rgba(230, 235, 255, 1.0);
            list.text(cx + w * 0.04, cy - 8.0, 14.0, label_color, Align::Left, label);
        }
        // Small companion-face modes: just the face (drawn by the caller).
        NotchMode::Resting
        | NotchMode::Waving
        | NotchMode::Sleeping
        | NotchMode::Vibing
        | NotchMode::Surprised
        | NotchMode::Thinking
        | NotchMode::Speaking
        | NotchMode::Loading => {
            // Face-only; nothing extra here.
        }
    }
}

/// Draw animated recording level bars.
fn draw_level_bars(list: &mut DrawList, cx: f32, cy: f32, w: f32, h: f32, phase: f32) {
    let bars = 7;
    let bar_w = 4.0;
    let gap = 3.0;
    let total = (bars as f32) * bar_w + ((bars - 1) as f32) * gap;
    let start_x = cx - total * 0.5;
    for i in 0..bars {
        let h_frac = 0.4 + 0.6 * ((i as f32 * 1.3 + phase) % 1.0).abs().sin();
        let bh = h * 0.4 * h_frac;
        let x = start_x + i as f32 * (bar_w + gap);
        list.fill_rect(x, cy - bh * 0.5, bar_w, bh, 1.5, rgba(255, 120, 120, 0.9));
    }
}

/// Draw the resting aurora glow (called for resting-family modes).
pub fn draw_resting_glow(list: &mut DrawList, cx: f32, cy: f32, w: f32, h: f32, phase: f32, on: bool) {
    if on {
        aurora::draw_aurora(list, cx, cy, w, h, phase);
    }
}
