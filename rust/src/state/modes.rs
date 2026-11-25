//! The 17 notch modes. Each mode maps to a shell size (design units) and a
//! radius, per the reference spec.

/// Shell size category.
#[derive(Clone, Copy, PartialEq)]
pub enum SizeCategory {
    Small,
    Medium,
    Large,
}

/// A notch mode. The 17 modes of the app.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NotchMode {
    // Small (154x32, r13) — companion-face states
    Resting,
    Waving,
    Sleeping,
    Vibing,
    Surprised,
    Thinking,
    Speaking,
    Loading,
    // Medium (332x68, r24)
    Notification,
    Charging,
    Headphones,
    Recording,
    Alert,
    Music,
    Focused,
    // Large (416x138 / 454x138, r34)
    Expanded,
    Lyrics,
}

impl NotchMode {
    pub const ALL: [NotchMode; 17] = [
        NotchMode::Resting,
        NotchMode::Waving,
        NotchMode::Sleeping,
        NotchMode::Vibing,
        NotchMode::Surprised,
        NotchMode::Thinking,
        NotchMode::Speaking,
        NotchMode::Loading,
        NotchMode::Notification,
        NotchMode::Charging,
        NotchMode::Headphones,
        NotchMode::Recording,
        NotchMode::Alert,
        NotchMode::Music,
        NotchMode::Focused,
        NotchMode::Expanded,
        NotchMode::Lyrics,
    ];

    pub fn tag(&self) -> &'static str {
        match self {
            NotchMode::Resting => "Resting",
            NotchMode::Waving => "Waving",
            NotchMode::Sleeping => "Sleeping",
            NotchMode::Vibing => "Vibing",
            NotchMode::Surprised => "Surprised",
            NotchMode::Thinking => "Thinking",
            NotchMode::Speaking => "Speaking",
            NotchMode::Loading => "Loading",
            NotchMode::Notification => "Notification",
            NotchMode::Charging => "Charging",
            NotchMode::Headphones => "Headphones",
            NotchMode::Recording => "Recording",
            NotchMode::Alert => "Alert",
            NotchMode::Music => "Music",
            NotchMode::Focused => "Focused",
            NotchMode::Expanded => "Expanded",
            NotchMode::Lyrics => "Lyrics",
        }
    }

    pub fn from_tag(tag: &str) -> NotchMode {
        NotchMode::ALL
            .into_iter()
            .find(|m| m.tag().eq_ignore_ascii_case(tag))
            .unwrap_or(NotchMode::Resting)
    }

    pub fn category(&self) -> SizeCategory {
        match self {
            NotchMode::Notification
            | NotchMode::Charging
            | NotchMode::Headphones
            | NotchMode::Recording
            | NotchMode::Alert
            | NotchMode::Music
            | NotchMode::Focused => SizeCategory::Medium,
            NotchMode::Expanded | NotchMode::Lyrics => SizeCategory::Large,
            _ => SizeCategory::Small,
        }
    }

    /// (width, height, radius) in design units for this mode.
    pub fn size(&self) -> (f32, f32, f32) {
        match self.category() {
            SizeCategory::Small => (154.0, 32.0, 13.0),
            SizeCategory::Medium => (332.0, 68.0, 24.0),
            SizeCategory::Large => {
                if matches!(self, NotchMode::Lyrics) {
                    (454.0, 138.0, 34.0)
                } else {
                    (416.0, 138.0, 34.0)
                }
            }
        }
    }
}
