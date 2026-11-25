//! Settings model + persistence. ~18 settings across 5 sections, saved to
//! %APPDATA%\MacBookNotch\settings.json.

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

/// Default mode tag (matches a NotchMode variant name in state::modes).
const DEFAULT_MODE: &str = "Resting";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    // --- Notch ---
    pub notch_scale: f32,
    pub resting_glow: bool,
    pub auto_hide_fullscreen: bool,
    pub default_mode: String,
    pub animation_speed: f32,

    // --- Companion ---
    pub companion_enabled: bool,
    pub expression_intensity: f32,
    pub companion_name: String,
    pub voice_feedback: bool,

    // --- Side Dock ---
    pub dock_enabled: bool,
    pub gauge_claude: bool,
    pub gauge_chatgpt: bool,
    pub gauge_gemini: bool,
    pub popover_default_open: bool,

    // --- Widgets ---
    pub widget_lyrics: bool,
    pub widget_clock: bool,
    pub clock_format: String,

    // --- Appearance ---
    pub accent_color: [u8; 3],
    pub transparency: f32,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            notch_scale: 1.6,
            resting_glow: true,
            auto_hide_fullscreen: true,
            default_mode: DEFAULT_MODE.to_string(),
            animation_speed: 1.0,

            companion_enabled: true,
            expression_intensity: 1.0,
            companion_name: "Notch".to_string(),
            voice_feedback: false,

            dock_enabled: true,
            gauge_claude: true,
            gauge_chatgpt: true,
            gauge_gemini: true,
            popover_default_open: false,

            widget_lyrics: true,
            widget_clock: true,
            clock_format: "24h".to_string(),

            accent_color: [120, 140, 255],
            transparency: 0.95,
        }
    }
}

impl Settings {
    /// Path to the settings file on Windows.
    pub fn path() -> PathBuf {
        let base = std::env::var("APPDATA").unwrap_or_else(|_| ".".to_string());
        PathBuf::from(base).join("MacBookNotch").join("settings.json")
    }

    /// Load settings, falling back to defaults on any error.
    pub fn load() -> Self {
        match fs::read_to_string(Self::path()) {
            Ok(text) => serde_json::from_str(&text).unwrap_or_else(|e| {
                eprintln!("warning: failed to parse settings ({e}); using defaults");
                Self::default()
            }),
            Err(_) => Self::default(),
        }
    }

    /// Persist settings to disk (best-effort).
    pub fn save(&self) {
        if let Some(parent) = Self::path().parent() {
            if let Err(e) = fs::create_dir_all(parent) {
                eprintln!("warning: could not create settings dir: {e}");
                return;
            }
        }
        match serde_json::to_string_pretty(self) {
            Ok(text) => {
                if let Err(e) = fs::write(Self::path(), text) {
                    eprintln!("warning: could not save settings: {e}");
                }
            }
            Err(e) => eprintln!("warning: could not serialize settings: {e}"),
        }
    }
}
