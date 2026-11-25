//! Animation framework: tweens + an animator that advances by time deltas.
//!
//! The animator owns active tweens, advances them, calls their update closures,
//! and reports whether anything is still animating (so the event loop can decide
//! whether to keep requesting redraws).

use std::collections::VecDeque;
use std::time::Duration;

use crate::easing;

/// An easing selector.
#[derive(Clone, Copy, PartialEq)]
pub enum Ease {
    Linear,
    EaseOut,
    EaseIn,
    EaseInOut,
    Shell,
    BodyMorph,
}

impl Ease {
    pub fn apply(self, t: f32) -> f32 {
        match self {
            Ease::Linear => t,
            Ease::EaseOut => easing::ease_out(t),
            Ease::EaseIn => easing::ease_in(t),
            Ease::EaseInOut => easing::ease_in_out(t),
            Ease::Shell => easing::shell(t),
            Ease::BodyMorph => easing::body_morph(t),
        }
    }
}

/// A single running tween. `update` receives the eased value in [0, 1] and the
/// raw progress; the closure is responsible for applying it to the target.
pub struct Tween {
    pub elapsed: Duration,
    pub duration: Duration,
    pub ease: Ease,
    pub update: Box<dyn FnMut(f32, f32)>,
    pub done: bool,
}

/// Owns active tweens. Tweens are advanced by `advance(delta)`.
pub struct Animator {
    tweens: VecDeque<Tween>,
}

impl Animator {
    pub fn new() -> Self {
        Self {
            tweens: VecDeque::new(),
        }
    }

    /// Spawn a tween. `update` is called every step with (eased_value, raw_t).
    pub fn spawn<F>(&mut self, duration: Duration, ease: Ease, update: F)
    where
        F: FnMut(f32, f32) + 'static,
    {
        self.tweens.push_back(Tween {
            elapsed: Duration::ZERO,
            duration,
            ease,
            update: Box::new(update),
            done: false,
        });
    }

    /// Advance all tweens by `delta`. Returns true if any tween is still active.
    pub fn advance(&mut self, delta: Duration) -> bool {
        let mut still_active = false;
        let mut i = 0;
        while i < self.tweens.len() {
            let tween = &mut self.tweens[i];
            tween.elapsed += delta;
            let raw = if tween.duration.is_zero() {
                1.0
            } else {
                (tween.elapsed.as_secs_f32() / tween.duration.as_secs_f32()).min(1.0)
            };
            let eased = tween.ease.apply(raw);
            (tween.update)(eased, raw);
            if raw >= 1.0 {
                tween.done = true;
                self.tweens.remove(i);
            } else {
                still_active = true;
                i += 1;
            }
        }
        still_active
    }

    pub fn is_active(&self) -> bool {
        !self.tweens.is_empty()
    }

    pub fn clear(&mut self) {
        self.tweens.clear();
    }
}

impl Default for Animator {
    fn default() -> Self {
        Self::new()
    }
}
