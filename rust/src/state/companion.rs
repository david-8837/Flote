//! Companion AI state machine — the animated face/expression living in the
//! notch. The "AI" feel is the expression animation, not a real model.

/// Expression of the companion face.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Expression {
    Idle,
    Listening,
    Thinking,
    Speaking,
    Happy,
    Surprised,
    Sleeping,
    Vibing,
}

/// The companion. Holds the current expression and a small FSM that the notch
/// renderer reads to draw eyes/mouth/blush.
pub struct Companion {
    pub expression: Expression,
    /// Blink timer accumulator (ms).
    blink_acc: f32,
    /// Next blink interval (ms).
    blink_next: f32,
    /// Whether eyes are currently closed (mid-blink).
    pub eyes_closed: bool,
    /// Set true on the frame `eyes_closed` toggled (so both the close and open
    /// frames get rendered).
    pub blink_changed: bool,
    /// Mouth openness 0..1 (for speaking cycles).
    pub mouth_open: f32,
    /// Blush intensity 0..1.
    pub blush: f32,
    /// Look offset -1..1 (eyes dart left/right).
    pub look_x: f32,
    pub look_y: f32,
}

impl Companion {
    pub fn new() -> Self {
        Self {
            expression: Expression::Idle,
            blink_acc: 0.0,
            blink_next: 2500.0,
            eyes_closed: false,
            blink_changed: false,
            mouth_open: 0.0,
            blush: 0.0,
            look_x: 0.0,
            look_y: 0.0,
        }
    }

    pub fn set_expression(&mut self, e: Expression) {
        self.expression = e;
    }

    /// Whether the companion is mid-animation (mid-blink or a continuously
    /// animating expression), so the renderer keeps redrawing it.
    pub fn is_animating(&self) -> bool {
        self.eyes_closed
            || self.blink_changed
            || matches!(
                self.expression,
                Expression::Speaking | Expression::Vibing | Expression::Listening
            )
    }

    /// Advance the companion by `dt_ms`. Drives blink, mouth, blush, look.
    pub fn tick(&mut self, dt_ms: f32) {
        let was_closed = self.eyes_closed;
        // Blink cycle (only when not sleeping).
        if !matches!(self.expression, Expression::Sleeping) {
            self.blink_acc += dt_ms;
            if self.blink_acc >= self.blink_next {
                self.blink_acc = 0.0;
                self.blink_next = 2200.0 + (self.blink_next - 2200.0).abs().min(3000.0);
                self.eyes_closed = true;
            }
            if self.eyes_closed {
                // Close for ~120ms then open.
                if self.blink_acc > 120.0 {
                    self.eyes_closed = false;
                }
            }
        }

        // Expression-driven parameters.
        match self.expression {
            Expression::Idle => {
                self.mouth_open = 0.0;
                self.blush = 0.0;
                self.look_x = 0.0;
                self.look_y = 0.0;
            }
            Expression::Listening => {
                self.mouth_open = 0.0;
                self.blush = 0.0;
                // Gentle look around.
                self.look_x = (self.blink_acc * 0.001).sin() * 0.5;
                self.look_y = (self.blink_acc * 0.0013).cos() * 0.3;
            }
            Expression::Thinking => {
                self.mouth_open = 0.0;
                self.blush = 0.0;
                self.look_y = 0.25;
                self.look_x = 0.0;
            }
            Expression::Speaking => {
                // Mouth cycles open/closed.
                self.mouth_open = (self.blink_acc * 0.012).sin().max(0.0) * 0.8;
                self.blush = 0.0;
                self.look_x = 0.0;
                self.look_y = 0.0;
            }
            Expression::Happy => {
                self.mouth_open = 0.2;
                self.blush = 0.5;
                self.look_x = 0.0;
                self.look_y = 0.0;
            }
            Expression::Surprised => {
                self.mouth_open = 0.7;
                self.blush = 0.0;
                self.look_x = 0.0;
                self.look_y = 0.0;
            }
            Expression::Sleeping => {
                self.mouth_open = 0.0;
                self.blush = 0.0;
                self.look_x = 0.0;
                self.look_y = 0.0;
            }
            Expression::Vibing => {
                self.mouth_open = 0.1 + (self.blink_acc * 0.01).sin().max(0.0) * 0.3;
                self.blush = 0.4;
                self.look_x = 0.0;
                self.look_y = 0.0;
            }
        }

        self.blink_changed = self.eyes_closed != was_closed;
    }
}

impl Default for Companion {
    fn default() -> Self {
        Self::new()
    }
}
