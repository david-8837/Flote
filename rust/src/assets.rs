//! Icon ID constants. Icons are drawn procedurally (see `render::icons`),
//! so there is no SVG asset loading — these IDs select the drawing routine.

pub const ICON_BATTERY: u32 = 0;
pub const ICON_BELL: u32 = 1;
pub const ICON_CHARGING: u32 = 2;
pub const ICON_HEADPHONES: u32 = 3;
pub const ICON_MUSIC: u32 = 4;
pub const ICON_ALERT: u32 = 5;
pub const ICON_MIC: u32 = 6;
pub const ICON_FOCUS: u32 = 7;
pub const ICON_LYRICS: u32 = 8;
pub const ICON_NOTIFICATION: u32 = 9;

/// All icon IDs.
pub const ALL_ICONS: [u32; 10] = [
    ICON_BATTERY,
    ICON_BELL,
    ICON_CHARGING,
    ICON_HEADPHONES,
    ICON_MUSIC,
    ICON_ALERT,
    ICON_MIC,
    ICON_FOCUS,
    ICON_LYRICS,
    ICON_NOTIFICATION,
];
