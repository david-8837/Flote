//! Settings UI: a panel rendered with the render core, plus a shared layout
//! used for mouse hit-testing.

pub mod widgets;

use crate::config::Settings;
use crate::render::shapes::rgba;
use crate::render::{Align, DrawList};
use crate::state::modes::NotchMode;

/// Identifies a setting row for hit-testing and mutation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SettingId {
    Header,
    NotchScale,
    RestingGlow,
    AutoHideFullscreen,
    DefaultMode,
    AnimationSpeed,
    CompanionEnabled,
    ExpressionIntensity,
    CompanionName,
    VoiceFeedback,
    DockEnabled,
    GaugeClaude,
    GaugeChatgpt,
    GaugeGemini,
    PopoverDefaultOpen,
    WidgetLyrics,
    WidgetClock,
    ClockFormat,
    AccentColor,
    Transparency,
}

#[derive(Clone)]
pub enum RowKind {
    Toggle(bool),
    Slider { value: f32, min: f32, max: f32 },
    Selector(String),
}

#[derive(Clone)]
pub struct Row {
    pub id: SettingId,
    pub label: String,
    pub kind: RowKind,
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

pub const PANEL_W: f32 = 460.0;
pub const PANEL_H: f32 = 720.0;
const ROW_H: f32 = 42.0;
const HEADER_H: f32 = 30.0;

/// Build the panel layout (rows with absolute rects) for hit-testing + render.
pub fn layout(settings: &Settings) -> Vec<Row> {
    let mut rows = Vec::new();
    let mut y = 70.0; // below the panel title
    let x = 24.0;
    let w = PANEL_W - 48.0;

    let sections: [(&str, Vec<(SettingId, String, RowKind)>); 5] = [
        ("Notch", vec![
            (SettingId::NotchScale, "Notch scale".into(), RowKind::Slider { value: settings.notch_scale, min: 0.8, max: 2.0 }),
            (SettingId::RestingGlow, "Resting glow".into(), RowKind::Toggle(settings.resting_glow)),
            (SettingId::AutoHideFullscreen, "Auto-hide in fullscreen".into(), RowKind::Toggle(settings.auto_hide_fullscreen)),
            (SettingId::DefaultMode, "Default mode".into(), RowKind::Selector(settings.default_mode.clone())),
            (SettingId::AnimationSpeed, "Animation speed".into(), RowKind::Slider { value: settings.animation_speed, min: 0.2, max: 2.0 }),
        ]),
        ("Companion", vec![
            (SettingId::CompanionEnabled, "Companion enabled".into(), RowKind::Toggle(settings.companion_enabled)),
            (SettingId::ExpressionIntensity, "Expression intensity".into(), RowKind::Slider { value: settings.expression_intensity, min: 0.0, max: 1.0 }),
            (SettingId::CompanionName, "Companion name".into(), RowKind::Selector(settings.companion_name.clone())),
            (SettingId::VoiceFeedback, "Voice feedback".into(), RowKind::Toggle(settings.voice_feedback)),
        ]),
        ("Side Dock", vec![
            (SettingId::DockEnabled, "Side dock".into(), RowKind::Toggle(settings.dock_enabled)),
            (SettingId::GaugeClaude, "Claude gauge".into(), RowKind::Toggle(settings.gauge_claude)),
            (SettingId::GaugeChatgpt, "ChatGPT gauge".into(), RowKind::Toggle(settings.gauge_chatgpt)),
            (SettingId::GaugeGemini, "Gemini gauge".into(), RowKind::Toggle(settings.gauge_gemini)),
            (SettingId::PopoverDefaultOpen, "Popover open by default".into(), RowKind::Toggle(settings.popover_default_open)),
        ]),
        ("Widgets", vec![
            (SettingId::WidgetLyrics, "Lyrics widget".into(), RowKind::Toggle(settings.widget_lyrics)),
            (SettingId::WidgetClock, "Clock widget".into(), RowKind::Toggle(settings.widget_clock)),
            (SettingId::ClockFormat, "Clock format".into(), RowKind::Selector(settings.clock_format.clone())),
        ]),
        ("Appearance", vec![
            (SettingId::AccentColor, "Accent color".into(), RowKind::Selector(accent_name(&settings.accent_color))),
            (SettingId::Transparency, "Transparency".into(), RowKind::Slider { value: settings.transparency, min: 0.5, max: 1.0 }),
        ]),
    ];

    for (title, items) in sections.iter() {
        rows.push(Row {
            id: SettingId::Header,
            label: title.to_string(),
            kind: RowKind::Toggle(false),
            x,
            y,
            w,
            h: HEADER_H,
        });
        y += HEADER_H + 8.0;
        for (id, label, kind) in items {
            rows.push(Row {
                id: *id,
                label: label.clone(),
                kind: kind.clone(),
                x,
                y,
                w,
                h: ROW_H,
            });
            y += ROW_H;
        }
        y += 12.0;
    }
    rows
}

fn accent_name(c: &[u8; 3]) -> String {
    match *c {
        [120, 140, 255] => "Blue".into(),
        [255, 120, 160] => "Pink".into(),
        [120, 220, 160] => "Green".into(),
        [255, 180, 90] => "Amber".into(),
        _ => "Blue".into(),
    }
}

fn next_accent(name: &str) -> [u8; 3] {
    match name {
        "Blue" => [255, 120, 160],
        "Pink" => [120, 220, 160],
        "Green" => [255, 180, 90],
        "Amber" => [120, 140, 255],
        _ => [120, 140, 255],
    }
}

fn next_name(name: &str) -> String {
    match name {
        "Notch" => "Companion".into(),
        "Companion" => "Aura".into(),
        _ => "Notch".into(),
    }
}

fn next_mode(name: &str) -> String {
    let idx = NotchMode::ALL
        .iter()
        .position(|m| m.tag().eq_ignore_ascii_case(name))
        .unwrap_or(0);
    NotchMode::ALL[(idx + 1) % NotchMode::ALL.len()].tag().to_string()
}

fn next_clock(name: &str) -> String {
    if name == "12h" { "24h".into() } else { "12h".into() }
}

/// Apply a click at (mx, my) to the settings. Returns true if something changed.
pub fn apply_click(settings: &mut Settings, rows: &[Row], mx: f32, my: f32) -> bool {
    for row in rows {
        // Skip header rows (not interactive).
        if row.id == SettingId::Header {
            continue;
        }
        if (my >= row.y && my <= row.y + row.h) && (mx >= row.x && mx <= row.x + row.w) {
            match row.id {
                SettingId::NotchScale => {
                    let frac = ((mx - row.x) / row.w).clamp(0.0, 1.0);
                    settings.notch_scale = 0.8 + frac * 1.2;
                    return true;
                }
                SettingId::AnimationSpeed => {
                    let frac = ((mx - row.x) / row.w).clamp(0.0, 1.0);
                    settings.animation_speed = 0.2 + frac * 1.8;
                    return true;
                }
                SettingId::ExpressionIntensity => {
                    let frac = ((mx - row.x) / row.w).clamp(0.0, 1.0);
                    settings.expression_intensity = frac;
                    return true;
                }
                SettingId::Transparency => {
                    let frac = ((mx - row.x) / row.w).clamp(0.0, 1.0);
                    settings.transparency = 0.5 + frac * 0.5;
                    return true;
                }
                SettingId::RestingGlow => { settings.resting_glow = !settings.resting_glow; return true; }
                SettingId::AutoHideFullscreen => { settings.auto_hide_fullscreen = !settings.auto_hide_fullscreen; return true; }
                SettingId::CompanionEnabled => { settings.companion_enabled = !settings.companion_enabled; return true; }
                SettingId::VoiceFeedback => { settings.voice_feedback = !settings.voice_feedback; return true; }
                SettingId::DockEnabled => { settings.dock_enabled = !settings.dock_enabled; return true; }
                SettingId::GaugeClaude => { settings.gauge_claude = !settings.gauge_claude; return true; }
                SettingId::GaugeChatgpt => { settings.gauge_chatgpt = !settings.gauge_chatgpt; return true; }
                SettingId::GaugeGemini => { settings.gauge_gemini = !settings.gauge_gemini; return true; }
                SettingId::PopoverDefaultOpen => { settings.popover_default_open = !settings.popover_default_open; return true; }
                SettingId::WidgetLyrics => { settings.widget_lyrics = !settings.widget_lyrics; return true; }
                SettingId::WidgetClock => { settings.widget_clock = !settings.widget_clock; return true; }
                SettingId::DefaultMode => { settings.default_mode = next_mode(&settings.default_mode); return true; }
                SettingId::CompanionName => { settings.companion_name = next_name(&settings.companion_name); return true; }
                SettingId::ClockFormat => { settings.clock_format = next_clock(&settings.clock_format); return true; }
                SettingId::AccentColor => { settings.accent_color = next_accent(&accent_name(&settings.accent_color)); return true; }
            }
        }
    }
    false
}

/// Draw the settings panel (background + title + all rows).
pub fn draw_settings(list: &mut DrawList, settings: &Settings) {
    let px = (crate::stage::STAGE_W - PANEL_W) * 0.5;
    let py = 60.0;

    // Panel background.
    list.shadow_rounded_rect(px, py, PANEL_W, PANEL_H, 18.0, 0.8);
    list.fill_rounded_rect(px, py, PANEL_W, PANEL_H, 18.0, rgba(12, 14, 22, 0.97));
    let border = rgba(255, 255, 255, 0.10);
    list.stroke_rounded_rect(px, py, PANEL_W, PANEL_H, 18.0, 1.0, border, 16);

    // Title + close hint.
    let title_color = rgba(240, 244, 255, 1.0);
    list.text(px + 24.0, py + 28.0, 20.0, title_color, Align::Left, "Settings");
    let hint = rgba(150, 160, 190, 1.0);
    list.text(px + PANEL_W - 24.0, py + 32.0, 13.0, hint, Align::Right, "Esc to close");

    let rows = layout(settings);
    for row in &rows {
        // Header rows: id is NotchScale but label is a section title and h is HEADER_H.
        if row.h == HEADER_H {
            widgets::draw_header(list, row.x, row.y + 4.0, &row.label, settings.accent_color);
            continue;
        }
        let label_y = row.y + 14.0;
        widgets::draw_label(list, row.x, label_y, &row.label, None);
        match &row.kind {
            RowKind::Toggle(on) => {
                widgets::draw_toggle(list, row.x + row.w - 56.0, row.y + 10.0, *on, settings.accent_color);
            }
            RowKind::Slider { value, min, max } => {
                let frac = ((*value - *min) / (*max - *min)).clamp(0.0, 1.0);
                widgets::draw_slider(list, row.x, row.y + 18.0, row.w, frac, settings.accent_color);
            }
            RowKind::Selector(current) => {
                widgets::draw_selector(list, row.x + row.w - 120.0, row.y + 8.0, 120.0, current, settings.accent_color);
            }
        }
    }
}
