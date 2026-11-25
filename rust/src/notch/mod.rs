//! Top notch rendering: shell, aurora glow, companion face, and mode content.

pub mod aurora;
pub mod modes;

use crate::config::Settings;
use crate::render::shapes::rgba;
use crate::render::DrawList;
use crate::state::modes::NotchMode;
use crate::state::NotchState;
use crate::stage::center_x;

use modes::{draw_companion_face, draw_mode_content, draw_resting_glow};

/// Modes that show the companion face (small shell).
fn is_face_mode(mode: NotchMode) -> bool {
    matches!(
        mode,
        NotchMode::Resting
            | NotchMode::Waving
            | NotchMode::Sleeping
            | NotchMode::Vibing
            | NotchMode::Surprised
            | NotchMode::Thinking
            | NotchMode::Speaking
            | NotchMode::Loading
    )
}

/// Pick the icon + label for a mode.
fn mode_icon(mode: NotchMode) -> (u32, &'static str) {
    use crate::assets;
    match mode {
        NotchMode::Notification => (assets::ICON_NOTIFICATION, "Notification"),
        NotchMode::Charging => (assets::ICON_CHARGING, "Charging"),
        NotchMode::Headphones => (assets::ICON_HEADPHONES, "Audio"),
        NotchMode::Music => (assets::ICON_MUSIC, "Now Playing"),
        NotchMode::Alert => (assets::ICON_ALERT, "Alert"),
        NotchMode::Focused => (assets::ICON_FOCUS, "Focus"),
        NotchMode::Recording => (assets::ICON_MIC, "Recording"),
        NotchMode::Expanded => (assets::ICON_BATTERY, "MacBook"),
        NotchMode::Lyrics => (assets::ICON_LYRICS, "Lyrics"),
        _ => (assets::ICON_BATTERY, "MacBook"),
    }
}

/// Draw the full top notch for the current notch state.
pub fn draw_notch(list: &mut DrawList, notch: &NotchState, settings: &Settings) {
    let (w, h, radius) = notch.size;
    let cx = center_x();
    let x = cx - w * 0.5;
    let y = 0.0;
    let cy = y + h * 0.5;

    let mode = notch.mode;

    // Aurora resting glow (for resting-family modes, if enabled).
    if settings.resting_glow && matches!(mode, NotchMode::Resting | NotchMode::Vibing) {
        draw_resting_glow(list, cx, cy, w, h, notch.aurora_phase, true);
    }

    // Shadow + shell.
    let shell_alpha = settings.transparency;
    let shell_color = rgba(8, 8, 10, shell_alpha);
    list.shadow_rounded_rect(x, y, w, h, radius, 0.9);
    list.fill_rounded_rect(x, y, w, h, radius, shell_color);

    // Content inside the notch (clipped to the shell).
    list.clip_rounded_rect(x, y, w, h, radius);
    if is_face_mode(mode) && settings.companion_enabled {
        let face_scale = h.max(w) * 0.5;
        draw_companion_face(list, cx, cy, face_scale, &notch.companion);
    } else {
        let (icon_id, label) = mode_icon(mode);
        draw_mode_content(list, mode, cx, cy, w, h, icon_id, label, notch.aurora_phase);
    }
    list.clip_pop();
}
