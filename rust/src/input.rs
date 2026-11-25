//! Mouse/keyboard input handling and hit-testing (stage coordinates).

use crate::settings;
use crate::state::modes::NotchMode;
use crate::state::AppState;
use crate::stage::center_x;

/// Convert a physical-pixel cursor position to stage coordinates using the
/// current view transform (scale, origin).
pub fn to_stage(phys: (f32, f32), view: [f32; 4]) -> (f32, f32) {
    ((phys.0 - view[2]) / view[0], (phys.1 - view[3]) / view[1])
}

/// Handle a mouse click/press at a physical position.
pub fn handle_click(state: &mut AppState, phys: (f32, f32), view: [f32; 4]) {
    let (sx, sy) = to_stage(phys, view);

    if state.settings_open {
        // Click inside the settings panel → apply to the matching row.
        let panel_x = (crate::stage::STAGE_W - settings::PANEL_W) * 0.5;
        let panel_y = 60.0;
        if sx >= panel_x && sx <= panel_x + settings::PANEL_W && sy >= panel_y && sy <= panel_y + settings::PANEL_H {
            let rows = settings::layout(&state.settings);
            if settings::apply_click(&mut state.settings, &rows, sx, sy) {
                state.settings.save();
                state.mark_dirty();
            }
        } else {
            // Click outside → close settings.
            state.settings_open = false;
            state.mark_dirty();
        }
        return;
    }

    // Notch click → cycle to the next mode.
    let (w, h, _r) = state.notch.size;
    let nx = center_x() - w * 0.5;
    if sx >= nx && sx <= nx + w && sy >= 0.0 && sy <= h {
        let idx = NotchMode::ALL
            .iter()
            .position(|m| *m == state.notch.mode)
            .unwrap_or(0);
        let next = NotchMode::ALL[(idx + 1) % NotchMode::ALL.len()];
        state.notch.transition_to(next);
        match next {
            NotchMode::Resting => state.notch.companion.set_expression(crate::state::companion::Expression::Idle),
            NotchMode::Waving => state.notch.companion.set_expression(crate::state::companion::Expression::Happy),
            NotchMode::Sleeping => state.notch.companion.set_expression(crate::state::companion::Expression::Sleeping),
            NotchMode::Vibing => state.notch.companion.set_expression(crate::state::companion::Expression::Vibing),
            NotchMode::Surprised => state.notch.companion.set_expression(crate::state::companion::Expression::Surprised),
            NotchMode::Thinking => state.notch.companion.set_expression(crate::state::companion::Expression::Thinking),
            NotchMode::Speaking => state.notch.companion.set_expression(crate::state::companion::Expression::Speaking),
            NotchMode::Loading => state.notch.companion.set_expression(crate::state::companion::Expression::Listening),
            _ => state.notch.companion.set_expression(crate::state::companion::Expression::Idle),
        }
        state.mark_dirty();
        return;
    }

    // Dock click → toggle popover.
    if state.settings.dock_enabled {
        if sx >= 0.0 && sx <= 144.0 && sy >= 0.0 && sy <= 56.0 {
            state.dock.open = !state.dock.open;
            state.mark_dirty();
        }
    }
}

/// Handle a mouse move (hover states).
pub fn handle_move(state: &mut AppState, phys: (f32, f32), view: [f32; 4]) {
    let (sx, sy) = to_stage(phys, view);

    let over_dock = state.settings.dock_enabled
        && sx >= 0.0
        && sx <= 144.0
        && sy >= 0.0
        && sy <= 56.0;
    state.dock.hover = over_dock;

    // Hovering the dock auto-opens the popover (if not explicitly toggled closed).
    if over_dock && !state.dock.open {
        // Keep closed unless clicked; do not force-open to avoid surprise.
    }
}

/// Handle a key press.
pub fn handle_key(state: &mut AppState, key: &str) {
    match key {
        "Escape" => {
            if state.settings_open {
                state.settings_open = false;
                state.mark_dirty();
            } else if state.dock.open {
                state.dock.open = false;
                state.mark_dirty();
            }
        }
        "OpenSettings" => {
            state.settings_open = true;
            state.mark_dirty();
        }
        _ => {}
    }
}
