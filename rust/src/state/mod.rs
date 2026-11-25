//! Application state: notch state, dock state, and the top-level AppState.

pub mod companion;
pub mod modes;
pub mod shelf;

use crate::anim::Animator;
use crate::config::Settings;
use companion::Companion;
use modes::NotchMode;
use shelf::Shelf;

/// Current animated progress of the notch shell morph (0..1).
#[derive(Clone, Copy)]
pub struct NotchState {
    pub mode: NotchMode,
    /// Target mode when transitioning; None when settled.
    pub target_mode: Option<NotchMode>,
    /// Morph progress 0..1 (interpolated size between current and target).
    pub morph: f32,
    /// Current rendered (w, h, radius) — updated each frame from morph.
    pub size: (f32, f32, f32),
    pub companion: Companion,
    /// Aurora glow phase 0..1 (4.2s cycle).
    pub aurora_phase: f32,
}

impl NotchState {
    pub fn new() -> Self {
        let mode = NotchMode::Resting;
        Self {
            size: mode.size(),
            mode,
            target_mode: None,
            morph: 1.0,
            companion: Companion::new(),
            aurora_phase: 0.0,
        }
    }

    /// Begin transitioning to a new mode.
    pub fn transition_to(&mut self, mode: NotchMode) {
        if mode == self.mode && self.morph >= 1.0 {
            return;
        }
        self.target_mode = Some(mode);
        self.morph = 0.0;
    }

    /// Advance the notch by dt_ms. Updates morph, size, companion, aurora.
    pub fn tick(&mut self, dt_ms: f32, animator: &mut Animator) {
        // Aurora glow: 4.2s cycle.
        self.aurora_phase = (self.aurora_phase + dt_ms / 4200.0) % 1.0;

        // Morph toward target size.
        if self.morph < 1.0 {
            self.morph = (self.morph + dt_ms / 520.0).min(1.0); // 0.52s body morph
            let from = self.mode.size();
            let to = self.target_mode.unwrap_or(self.mode).size();
            let t = crate::easing::body_morph(self.morph);
            self.size = (
                from.0 + (to.0 - from.0) * t,
                from.1 + (to.1 - from.1) * t,
                from.2 + (to.2 - from.2) * t,
            );
            if self.morph >= 1.0 {
                if let Some(m) = self.target_mode {
                    self.mode = m;
                }
                self.target_mode = None;
            }
        }

        // Companion tick.
        self.companion.tick(dt_ms);

        // Keep the animator ticking (drives redraws during transitions).
        animator.advance(std::time::Duration::from_millis(dt_ms as u64));
    }

    pub fn is_animating(&self) -> bool {
        self.morph < 1.0 || self.companion.is_animating()
    }
}

impl Default for NotchState {
    fn default() -> Self {
        Self::new()
    }
}

/// Side dock state.
#[derive(Clone, Copy)]
pub struct DockState {
    pub open: bool,
    pub open_progress: f32, // 0..1
    pub hover: bool,
    /// Usage percentages for the three gauges.
    pub claude: f32,
    pub chatgpt: f32,
    pub gemini: f32,
}

impl DockState {
    pub fn new() -> Self {
        Self {
            open: false,
            open_progress: 0.0,
            hover: false,
            claude: 73.0,
            chatgpt: 21.0,
            gemini: 52.0,
        }
    }

    pub fn tick(&mut self, dt_ms: f32) {
        // Open/close progress.
        let target = if self.open { 1.0 } else { 0.0 };
        let speed = dt_ms / 200.0; // 0.2s
        if self.open_progress < target {
            self.open_progress = (self.open_progress + speed).min(1.0);
        } else if self.open_progress > target {
            self.open_progress = (self.open_progress - speed).max(0.0);
        }
    }

    pub fn is_animating(&self) -> bool {
        self.open_progress != 0.0 && self.open_progress != 1.0
    }
}

impl Default for DockState {
    fn default() -> Self {
        Self::new()
    }
}

/// Top-level application state.
pub struct AppState {
    pub settings: Settings,
    pub notch: NotchState,
    pub dock: DockState,
    pub shelf: Shelf,
    pub animator: Animator,
    /// Redraw trigger: true when the scene needs a new frame.
    pub dirty: bool,
    /// Settings panel open.
    pub settings_open: bool,
    /// Whether a full-screen app is currently active (step-aside).
    pub fullscreen_active: bool,
}

impl AppState {
    pub fn new() -> Self {
        let settings = Settings::load();
        let notch = NotchState::new();
        notch.companion.set_expression(companion::Expression::Idle);
        Self {
            settings_open: false,
            settings,
            notch,
            dock: DockState::new(),
            shelf: Shelf::load(),
            animator: Animator::new(),
            dirty: true,
            fullscreen_active: false,
        }
    }

    pub fn mark_dirty(&mut self) {
        self.dirty = true;
    }

    /// Tick all state. Returns true if anything is still animating.
    pub fn tick(&mut self, dt_ms: f32) {
        self.notch.tick(dt_ms, &mut self.animator);
        self.dock.tick(dt_ms);

        // Anything still animating (or just changed) → redraw. Evaluated after
        // tick so discrete events (e.g. a blink toggle) during the tick fire.
        if self.animator.is_active() || self.notch.is_animating() || self.dock.is_animating() {
            self.mark_dirty();
        }
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}
