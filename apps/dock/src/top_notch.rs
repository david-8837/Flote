//! Independent top notch widget anchored at the top center of the screen.
//!
//! Handles:
//! - Fluid spring-driven UI expansion into music player (Paper Sky / Lumen Coast style)
//! - Lyrics mode widening with glowing line indicators
//! - Charging confirmation banner when charger plugged in
//! - Mascot idle eyes blinking & glancing when nothing is playing

use anyhow::Result;
use muelle_shell::system::Battery;
use std::{
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};
use windows::{
    Foundation::TimeSpan,
    UI::{
        Color,
        Composition::{
            CompositionBrush, CompositionSurfaceBrush, Compositor,
            ContainerVisual, Desktop::DesktopWindowTarget, SpriteVisual, VisualCollection,
        },
    },
    Win32::{
        Foundation::{HWND, LPARAM, LRESULT, POINT, RECT, WPARAM},
        System::{
            LibraryLoader::GetModuleHandleW, SystemInformation::GetLocalTime,
            WinRT::Composition::ICompositorDesktopInterop,
        },
        UI::Input::KeyboardAndMouse::{TME_LEAVE, TRACKMOUSEEVENT, TrackMouseEvent},
        UI::WindowsAndMessaging::{
            CreateWindowExW, DefWindowProcW, IDC_ARROW, KillTimer,
            LoadCursorW, RegisterClassW, SWP_HIDEWINDOW, SWP_NOACTIVATE,
            SWP_SHOWWINDOW, SetTimer, SetWindowPos, WM_APP, WM_TIMER, WNDCLASSW, WS_EX_NOACTIVATE,
            WS_EX_NOREDIRECTIONBITMAP, WS_EX_TOOLWINDOW, WS_EX_TOPMOST, WS_POPUP,
            WM_LBUTTONDOWN, WM_MOUSEMOVE, WM_NCHITTEST,
        },
    },
    core::{Interface, w},
};
use windows_numerics::{Vector2, Vector3};

use crate::{
    gfx::{Gfx, Ink},
    media::{self, Control as MediaControl, Track},
};

const CLASS_NAME: windows::core::PCWSTR = w!("MuelleTopNotch");
const HTCLIENT: LRESULT = LRESULT(1);
const HTTRANSPARENT: LRESULT = LRESULT(-1);
const WM_MOUSELEAVE: u32 = 0x02A3;
const MEDIA_CHANGED: u32 = WM_APP + 28;

const CHARGING_TIMER_ID: usize = 101;
const IDLE_EYES_TIMER_ID: usize = 102;

fn rgba(r: u8, g: u8, b: u8, a: u8) -> Color {
    Color { A: a, R: r, G: g, B: b }
}

#[derive(Clone, Copy, PartialEq)]
enum TopNotchAction {
    LyricsToggle,
    PlayPause,
    Previous,
    Next,
    Seek(f32),
}

struct HitZone {
    bounds: (f32, f32, f32, f32),
    action: TopNotchAction,
}

pub struct TopNotch {
    hwnd: HWND,
    compositor: Compositor,
    target: DesktopWindowTarget,
    surface: SpriteVisual,
    content_container: ContainerVisual,
    shell: SpriteVisual,
    scale: f32,
    screen: RECT,

    pub enabled: bool,
    pub music_enabled: bool,
    pub lyrics_enabled: bool,
    pub idle_eyes_enabled: bool,
    pub charging_enabled: bool,

    media: Option<media::Watch>,
    track: Option<Track>,
    cover: Option<(Option<Arc<muelle_shell::icons::IconImage>>, CompositionSurfaceBrush)>,
    lyrics_open: bool,
    expanded: bool,
    hovering: bool,
    charging: bool,
    charging_percent: u8,
    battery: Option<Battery>,

    visible_bounds: (f32, f32, f32, f32),
    hit_zones: Vec<HitZone>,
    lyrics_lines: Vec<String>,
}

static mut CURRENT_NOTCH: Option<*mut TopNotch> = None;

impl TopNotch {
    pub fn create(gfx: &Gfx) -> Result<Self> {
        let instance = unsafe { GetModuleHandleW(None)? };
        let class = WNDCLASSW {
            lpfnWndProc: Some(top_notch_wndproc),
            hInstance: instance.into(),
            lpszClassName: CLASS_NAME,
            hCursor: unsafe { LoadCursorW(None, IDC_ARROW)? },
            ..Default::default()
        };
        unsafe {
            let _ = RegisterClassW(&class);
        }

        let hwnd = unsafe {
            CreateWindowExW(
                WS_EX_NOREDIRECTIONBITMAP | WS_EX_TOPMOST | WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE,
                CLASS_NAME,
                w!("Flote Top Notch"),
                WS_POPUP,
                0,
                0,
                600,
                200,
                None,
                None,
                Some(instance.into()),
                None,
            )?
        };

        let compositor = gfx.compositor.clone();
        let interop: ICompositorDesktopInterop = compositor.cast()?;
        let target = unsafe { interop.CreateDesktopWindowTarget(hwnd, false)? };
        let root = compositor.CreateContainerVisual()?;
        target.SetRoot(&root)?;

        let surface = compositor.CreateSpriteVisual()?;
        root.Children()?.InsertAtTop(&surface)?;

        let shell = compositor.CreateSpriteVisual()?;
        surface.Children()?.InsertAtTop(&shell)?;

        let content_container = compositor.CreateContainerVisual()?;
        surface.Children()?.InsertAtTop(&content_container)?;

        let watch = media::Watch::start(hwnd, MEDIA_CHANGED);

        let notch = Self {
            hwnd,
            compositor,
            target,
            surface,
            content_container,
            shell,
            scale: 1.0,
            screen: RECT::default(),
            enabled: true,
            music_enabled: true,
            lyrics_enabled: true,
            idle_eyes_enabled: true,
            charging_enabled: true,
            media: watch,
            track: None,
            cover: None,
            lyrics_open: false,
            expanded: false,
            hovering: false,
            charging: false,
            charging_percent: 0,
            battery: None,
            visible_bounds: (0.0, 0.0, 0.0, 0.0),
            hit_zones: Vec::new(),
            lyrics_lines: vec![
                "sky".to_owned(),
                "to find you by the sea".to_owned(),
                "every wing a little song".to_owned(),
                "that carries us along".to_owned(),
            ],
        };

        unsafe {
            SetTimer(Some(hwnd), IDLE_EYES_TIMER_ID, 50, None);
        }

        Ok(notch)
    }

    pub fn set_screen(&mut self, screen: RECT, scale: f32) {
        self.screen = screen;
        self.scale = scale;
    }

    pub fn media_changed(&mut self, gfx: &Gfx) -> Result<()> {
        let new_track = self.media.as_ref().and_then(media::Watch::snapshot);
        self.set_track(gfx, new_track)
    }

    pub fn set_track(&mut self, gfx: &Gfx, track: Option<Track>) -> Result<()> {
        let changed = self.track.as_ref().map(|t| &t.title) != track.as_ref().map(|t| &t.title);
        self.track = track;
        if changed {
            self.cover = None;
        }
        self.draw(gfx)
    }

    pub fn set_battery(&mut self, gfx: &Gfx, battery: Option<Battery>) -> Result<()> {
        if let (Some(new_bat), Some(old_bat)) = (battery, self.battery) {
            if !old_bat.charging && new_bat.charging && self.charging_enabled {
                self.charging = true;
                self.charging_percent = new_bat.percent;
                unsafe {
                    SetTimer(Some(self.hwnd), CHARGING_TIMER_ID, 4000, None);
                }
            }
        }
        self.battery = battery;
        self.draw(gfx)
    }

    pub fn draw(&mut self, gfx: &Gfx) -> Result<()> {
        if !self.enabled {
            unsafe {
                let _ = SetWindowPos(
                    self.hwnd,
                    None,
                    0,
                    0,
                    0,
                    0,
                    SWP_HIDEWINDOW | SWP_NOACTIVATE,
                );
            }
            return Ok(());
        }

        unsafe {
            CURRENT_NOTCH = Some(self as *mut TopNotch);
        }

        let px = |logical: f32| (logical * self.scale).round();
        let c = &self.compositor;

        let has_music = self.music_enabled && self.track.is_some();
        let show_expanded_music = has_music && (self.expanded || self.hovering);

        let (notch_w, notch_h) = if self.charging {
            (px(330.0), px(52.0))
        } else if show_expanded_music {
            let w = if self.lyrics_open && self.lyrics_enabled { 560.0 } else { 420.0 };
            (px(w), px(148.0))
        } else {
            // Resting notch pill
            (px(214.0), px(28.0))
        };

        let win_w = px(580.0);
        let win_h = px(170.0);
        let win_x = self.screen.left + (self.screen.right - self.screen.left - win_w as i32) / 2;
        let win_y = self.screen.top;

        unsafe {
            let _ = SetWindowPos(
                self.hwnd,
                None,
                win_x,
                win_y,
                win_w as i32,
                win_h as i32,
                SWP_SHOWWINDOW | SWP_NOACTIVATE,
            );
        }

        let notch_x = (win_w - notch_w) / 2.0;
        let notch_y = 0.0;
        self.visible_bounds = (notch_x, notch_y, notch_x + notch_w, notch_y + notch_h);
        self.hit_zones.clear();

        // 1. Surface Shell Shape
        let mask = gfx.island_mask(notch_w, notch_h, self.scale)?;
        let mask_brush = c.CreateMaskBrush()?;
        let black_brush = c.CreateColorBrushWithColor(rgba(0, 0, 0, 255))?;
        mask_brush.SetSource(&black_brush)?;
        mask_brush.SetMask(&mask)?;

        self.shell.SetSize(Vector2 { X: notch_w, Y: notch_h })?;
        self.shell.SetOffset(Vector3 { X: notch_x, Y: notch_y, Z: 0.0 })?;
        self.shell.SetBrush(&mask_brush)?;

        // Spring animation if newly expanded
        let duration = TimeSpan { Duration: 340 * 10_000 };
        let ease_spring = c.CreateCubicBezierEasingFunction(
            Vector2::new(0.16, 1.0),
            Vector2::new(0.3, 1.0),
        )?;

        self.content_container.SetSize(Vector2 { X: notch_w, Y: notch_h })?;
        self.content_container.SetOffset(Vector3 { X: notch_x, Y: notch_y, Z: 0.0 })?;

        let contents = self.content_container.Children()?;
        contents.RemoveAll()?;

        if self.charging {
            self.draw_charging_content(gfx, &contents, notch_w, notch_h)?;
        } else if show_expanded_music {
            let track = self.track.clone().unwrap();
            self.draw_player_content(gfx, &contents, notch_w, notch_h, &track)?;
        } else if self.idle_eyes_enabled && self.track.is_none() {
            self.draw_idle_eyes_content(gfx, &contents, notch_w, notch_h)?;
        } else {
            self.draw_resting_clock_content(gfx, &contents, notch_w, notch_h)?;
        }

        Ok(())
    }

    fn draw_charging_content(
        &mut self,
        gfx: &Gfx,
        layers: &VisualCollection,
        w: f32,
        h: f32,
    ) -> Result<()> {
        let c = &self.compositor;
        let px = |v: f32| (v * self.scale).round();

        let ring_size = px(28.0);
        let ring_x = px(22.0);
        let ring_y = (h - ring_size) / 2.0;

        let green = rgba(34, 197, 94, 255);
        add_box(c, layers, ring_x, ring_y, ring_size, ring_size, ring_size / 2.0, green)?;
        let hole = ring_size - px(5.0);
        add_box(c, layers, ring_x + px(2.5), ring_y + px(2.5), hole, hole, hole / 2.0, rgba(0, 0, 0, 255))?;

        let bolt = gfx.glyph_brush(&[('\u{EBAE}', 1.0)], self.scale, Ink::Light)?;
        add_vis(c, layers, &bolt.brush.cast()?, ring_x + (ring_size - bolt.width) / 2.0, ring_y + (ring_size - bolt.height) / 2.0, bolt.width, bolt.height)?;

        let title = gfx.text_brush_sized("Charging", self.scale, 13.5, true, Ink::Light)?;
        add_vis(c, layers, &title.brush.cast()?, px(62.0), px(9.0), title.width, title.height)?;

        let sub = gfx.text_brush_sized("Charger connected", self.scale, 10.5, false, Ink::Light)?;
        let sub_vis = add_vis(c, layers, &sub.brush.cast()?, px(62.0), px(28.0), sub.width, sub.height)?;
        sub_vis.SetOpacity(0.70)?;

        let pct_str = format!("{}%", self.charging_percent);
        let pct = gfx.text_brush_sized(&pct_str, self.scale, 20.0, false, Ink::Light)?;
        add_vis(c, layers, &pct.brush.cast()?, w - px(24.0) - pct.width, (h - pct.height) / 2.0, pct.width, pct.height)?;

        Ok(())
    }

    fn draw_idle_eyes_content(
        &self,
        gfx: &Gfx,
        layers: &VisualCollection,
        w: f32,
        h: f32,
    ) -> Result<()> {
        let c = &self.compositor;
        let px = |v: f32| (v * self.scale).round();
        let center_x = w / 2.0;
        let center_y = h / 2.0;

        let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_millis();
        let cycle = (now % 4500) as f32;
        let is_blinking = cycle >= 3500.0 && cycle < 3700.0;

        let glance_x = if cycle >= 1200.0 && cycle < 2200.0 {
            px(3.0)
        } else if cycle >= 2400.0 && cycle < 3300.0 {
            -px(2.5)
        } else {
            0.0
        };

        let eye_w = px(10.5);
        let eye_h = if is_blinking { px(2.5) } else { px(16.5) };
        let eye_radius = if is_blinking { px(1.2) } else { px(5.2) };
        let gap = px(9.0);

        let left_eye_x = center_x - gap / 2.0 - eye_w + glance_x;
        let right_eye_x = center_x + gap / 2.0 + glance_x;
        let eye_y = center_y - eye_h / 2.0;

        let eye_color = rgba(255, 255, 255, 255);
        add_box(c, layers, left_eye_x, eye_y, eye_w, eye_h, eye_radius, eye_color)?;
        add_box(c, layers, right_eye_x, eye_y, eye_w, eye_h, eye_radius, eye_color)?;

        Ok(())
    }

    fn draw_resting_clock_content(
        &mut self,
        gfx: &Gfx,
        layers: &VisualCollection,
        w: f32,
        h: f32,
    ) -> Result<()> {
        let c = &self.compositor;
        let px = |v: f32| (v * self.scale).round();

        let ring_size = px(12.0);
        let ring_x = px(16.0);
        let ring_y = (h - ring_size) / 2.0;
        let green = rgba(34, 197, 94, 255);
        add_box(c, layers, ring_x, ring_y, ring_size, ring_size, ring_size / 2.0, green)?;
        add_box(c, layers, ring_x + px(1.5), ring_y + px(1.5), ring_size - px(3.0), ring_size - px(3.0), (ring_size - px(3.0)) / 2.0, rgba(0, 0, 0, 255))?;

        let time_str = {
            let st = unsafe { GetLocalTime() };
            format!("{:02}:{:02}", st.wHour, st.wMinute)
        };
        let clock = gfx.text_brush_sized(&time_str, self.scale, 11.0, true, Ink::Light)?;
        add_vis(c, layers, &clock.brush.cast()?, ring_x + ring_size + px(8.0), (h - clock.height) / 2.0, clock.width, clock.height)?;

        if let Some(track) = &self.track {
            let wave = gfx.glyph_brush(&[('', 0.8)], self.scale, Ink::Light)?;
            add_vis(c, layers, &wave.brush.cast()?, w - px(20.0) - wave.width, (h - wave.height) / 2.0, wave.width, wave.height)?;
        }

        Ok(())
    }

    fn draw_player_content(
        &mut self,
        gfx: &Gfx,
        layers: &VisualCollection,
        w: f32,
        h: f32,
        track: &Track,
    ) -> Result<()> {
        let c = &self.compositor;
        let px = |v: f32| (v * self.scale).round();
        let ink = Ink::Light;

        // Album cover
        let cover_size = px(56.0);
        let cover_x = px(20.0);
        let cover_y = px(22.0);

        if self.cover.is_none() {
            self.cover = media::artwork(track)
                .map(|img| {
                    let s = gfx.image_surface(img)?;
                    Ok::<_, anyhow::Error>((
                        track.artwork.clone(),
                        gfx.compositor.CreateSurfaceBrushWithSurface(&s)?,
                    ))
                })
                .transpose()?;
        }

        if let Some((_, cover_brush)) = &self.cover {
            add_vis(c, layers, &cover_brush.cast()?, cover_x, cover_y, cover_size, cover_size)?;
        } else {
            add_box(c, layers, cover_x, cover_y, cover_size, cover_size, px(10.0), rgba(28, 30, 36, 255))?;
            let note = gfx.glyph_brush(&[('', 1.0)], self.scale, ink)?;
            add_vis(c, layers, &note.brush.cast()?, cover_x + (cover_size - note.width) / 2.0, cover_y + (cover_size - note.height) / 2.0, note.width, note.height)?;
        }

        // Title and artist
        let text_x = cover_x + cover_size + px(16.0);
        let max_text_w = if self.lyrics_open { px(160.0) } else { w - text_x - px(48.0) };
        let title_str = if track.title.trim().is_empty() { "Nothing playing" } else { &track.title };
        let title = gfx.text_brush_fitted(title_str, self.scale, 14.5, true, ink, max_text_w)?;
        add_vis(c, layers, &title.brush.cast()?, text_x, px(24.0), title.width, title.height)?;

        let artist_str = if track.artist.trim().is_empty() { "Unknown artist" } else { &track.artist };
        let artist = gfx.text_brush_fitted(artist_str, self.scale, 11.5, false, ink, max_text_w)?;
        let art_vis = add_vis(c, layers, &artist.brush.cast()?, text_x, px(45.0), artist.width, artist.height)?;
        art_vis.SetOpacity(0.68)?;

        // Top right waveform and next chevron
        let wave = gfx.glyph_brush(&[('', 0.9)], self.scale, ink)?;
        add_vis(c, layers, &wave.brush.cast()?, w - px(24.0) - wave.width, px(14.0), wave.width, wave.height)?;

        // Progress bar
        let bar_y = px(88.0);
        let bar_left = px(64.0);
        let bar_right = if self.lyrics_open { px(240.0) } else { w - px(64.0) };
        let bar_w = bar_right - bar_left;
        let bar_h = px(3.5);

        let t_left = gfx.text_brush_sized("01:24", self.scale, 9.5, false, ink)?;
        add_vis(c, layers, &t_left.brush.cast()?, px(24.0), bar_y - px(4.0), t_left.width, t_left.height)?;

        let t_right = gfx.text_brush_sized("-02:40", self.scale, 9.5, false, ink)?;
        add_vis(c, layers, &t_right.brush.cast()?, bar_right + px(8.0), bar_y - px(4.0), t_right.width, t_right.height)?;

        // Track & Progress
        add_box(c, layers, bar_left, bar_y, bar_w, bar_h, bar_h / 2.0, rgba(255, 255, 255, 50))?;
        let fill_w = bar_w * 0.35;
        add_box(c, layers, bar_left, bar_y, fill_w, bar_h, bar_h / 2.0, rgba(255, 255, 255, 255))?;

        // Bottom Controls Row
        let ctrl_y = px(106.0);
        let btn_size = px(26.0);

        // Lyrics toggle button
        let lyrics_x = px(24.0);
        let bubble_glyph = gfx.glyph_brush(&[('', 1.0)], self.scale, ink)?;
        let bubble_vis = add_vis(c, layers, &bubble_glyph.brush.cast()?, lyrics_x + (btn_size - bubble_glyph.width) / 2.0, ctrl_y + (btn_size - bubble_glyph.height) / 2.0, bubble_glyph.width, bubble_glyph.height)?;
        if !self.lyrics_open {
            bubble_vis.SetOpacity(0.65)?;
        }
        self.hit_zones.push(HitZone {
            bounds: (lyrics_x, ctrl_y, lyrics_x + btn_size, ctrl_y + btn_size),
            action: TopNotchAction::LyricsToggle,
        });

        // Prev / PlayPause / Next
        let center_x = if self.lyrics_open { px(150.0) } else { w / 2.0 };
        let prev_x = center_x - px(48.0) - btn_size / 2.0;
        let prev_glyph = gfx.glyph_brush(&[('\u{E892}', 1.0)], self.scale, ink)?;
        add_vis(c, layers, &prev_glyph.brush.cast()?, prev_x + (btn_size - prev_glyph.width) / 2.0, ctrl_y + (btn_size - prev_glyph.height) / 2.0, prev_glyph.width, prev_glyph.height)?;
        self.hit_zones.push(HitZone {
            bounds: (prev_x, ctrl_y, prev_x + btn_size, ctrl_y + btn_size),
            action: TopNotchAction::Previous,
        });

        let play_x = center_x - btn_size / 2.0;
        let play_char = if track.playing { '\u{E769}' } else { '\u{E768}' };
        let play_glyph = gfx.glyph_brush(&[(play_char, 1.1)], self.scale, ink)?;
        add_vis(c, layers, &play_glyph.brush.cast()?, play_x + (btn_size - play_glyph.width) / 2.0, ctrl_y + (btn_size - play_glyph.height) / 2.0, play_glyph.width, play_glyph.height)?;
        self.hit_zones.push(HitZone {
            bounds: (play_x, ctrl_y, play_x + btn_size, ctrl_y + btn_size),
            action: TopNotchAction::PlayPause,
        });

        let next_x = center_x + px(48.0) - btn_size / 2.0;
        let next_glyph = gfx.glyph_brush(&[('\u{E893}', 1.0)], self.scale, ink)?;
        add_vis(c, layers, &next_glyph.brush.cast()?, next_x + (btn_size - next_glyph.width) / 2.0, ctrl_y + (btn_size - next_glyph.height) / 2.0, next_glyph.width, next_glyph.height)?;
        self.hit_zones.push(HitZone {
            bounds: (next_x, ctrl_y, next_x + btn_size, ctrl_y + btn_size),
            action: TopNotchAction::Next,
        });

        // Dots indicator at bottom
        let dot_y = px(138.0);
        let dot_w = px(14.0);
        let dot_h = px(2.5);
        add_box(c, layers, center_x - dot_w / 2.0, dot_y, dot_w, dot_h, dot_h / 2.0, rgba(255, 255, 255, 200))?;

        // If lyrics open: render lyrics pane on the right side
        if self.lyrics_open && self.lyrics_enabled {
            let sep_x = px(270.0);
            add_box(c, layers, sep_x, px(20.0), 1.0, h - px(40.0), 0.5, rgba(255, 255, 255, 30))?;

            let mut lyric_y = px(24.0);
            let lyric_left = sep_x + px(24.0);

            for (idx, line) in self.lyrics_lines.iter().enumerate() {
                let is_active = idx == 2;
                let font_size = if is_active { 14.5 } else { 12.0 };
                let bold = is_active;
                let l_brush = gfx.text_brush_sized(line, self.scale, font_size, bold, ink)?;
                let l_vis = add_vis(c, layers, &l_brush.brush.cast()?, lyric_left, lyric_y, l_brush.width, l_brush.height)?;
                if !is_active {
                    l_vis.SetOpacity(0.40)?;
                }
                lyric_y += l_brush.height + px(7.0);
            }
        }

        Ok(())
    }

    pub fn handle_mouse_move(&mut self, gfx: &Gfx, x: f32, y: f32) -> Result<()> {
        let inside = x >= self.visible_bounds.0
            && x <= self.visible_bounds.2
            && y >= self.visible_bounds.1
            && y <= self.visible_bounds.3;

        if inside && !self.hovering {
            self.hovering = true;
            self.expanded = true;
            self.draw(gfx)?;
        }
        Ok(())
    }

    pub fn handle_mouse_leave(&mut self, gfx: &Gfx) -> Result<()> {
        if self.hovering {
            self.hovering = false;
            self.expanded = false;
            self.draw(gfx)?;
        }
        Ok(())
    }

    pub fn handle_click(&mut self, gfx: &Gfx, x: f32, y: f32) -> Result<()> {
        let rel_x = x - self.visible_bounds.0;
        let rel_y = y - self.visible_bounds.1;

        for zone in &self.hit_zones {
            if rel_x >= zone.bounds.0
                && rel_x <= zone.bounds.2
                && rel_y >= zone.bounds.1
                && rel_y <= zone.bounds.3
            {
                match zone.action {
                    TopNotchAction::LyricsToggle => {
                        self.lyrics_open = !self.lyrics_open;
                        self.draw(gfx)?;
                        return Ok(());
                    }
                    TopNotchAction::PlayPause => {
                        if let (Some(watch), Some(track)) = (&self.media, &self.track) {
                            watch.send(track, MediaControl::PlayPause);
                        }
                        return Ok(());
                    }
                    TopNotchAction::Previous => {
                        if let (Some(watch), Some(track)) = (&self.media, &self.track) {
                            watch.send(track, MediaControl::Previous);
                        }
                        return Ok(());
                    }
                    TopNotchAction::Next => {
                        if let (Some(watch), Some(track)) = (&self.media, &self.track) {
                            watch.send(track, MediaControl::Next);
                        }
                        return Ok(());
                    }
                    TopNotchAction::Seek(_) => {}
                }
            }
        }

        if !self.expanded {
            self.expanded = true;
            self.draw(gfx)?;
        }
        Ok(())
    }
}

fn add_vis(
    c: &Compositor,
    layers: &VisualCollection,
    brush: &CompositionBrush,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
) -> Result<SpriteVisual> {
    let sprite = c.CreateSpriteVisual()?;
    sprite.SetBrush(brush)?;
    sprite.SetOffset(Vector3 { X: x, Y: y, Z: 0.0 })?;
    sprite.SetSize(Vector2 { X: w, Y: h })?;
    layers.InsertAtTop(&sprite)?;
    Ok(sprite)
}

fn add_box(
    c: &Compositor,
    layers: &VisualCollection,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    radius: f32,
    color: Color,
) -> Result<SpriteVisual> {
    let brush = c.CreateColorBrushWithColor(color)?;
    let sprite = c.CreateSpriteVisual()?;
    sprite.SetBrush(&brush)?;
    sprite.SetOffset(Vector3 { X: x, Y: y, Z: 0.0 })?;
    sprite.SetSize(Vector2 { X: w, Y: h })?;

    let shape = c.CreateShapeVisual()?;
    shape.SetSize(Vector2 { X: w, Y: h })?;
    let geom = c.CreateRoundedRectangleGeometry()?;
    geom.SetSize(Vector2 { X: w, Y: h })?;
    geom.SetCornerRadius(Vector2 { X: radius, Y: radius })?;
    let sprite_shape = c.CreateSpriteShapeWithGeometry(&geom)?;
    sprite_shape.SetFillBrush(&brush)?;
    shape.Shapes()?.Append(&sprite_shape)?;

    layers.InsertAtTop(&shape)?;
    Ok(sprite)
}

unsafe extern "system" fn top_notch_wndproc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match msg {
        WM_NCHITTEST => {
            let mut pt = POINT {
                x: (lparam.0 & 0xFFFF) as i16 as i32,
                y: ((lparam.0 >> 16) & 0xFFFF) as i16 as i32,
            };
            if let Some(notch_ptr) = unsafe { CURRENT_NOTCH } {
                let notch = unsafe { &*notch_ptr };
                let win_x = notch.screen.left + (notch.screen.right - notch.screen.left - 580) / 2;
                let rel_x = (pt.x - win_x) as f32;
                let rel_y = (pt.y - notch.screen.top) as f32;

                if rel_x >= notch.visible_bounds.0
                    && rel_x <= notch.visible_bounds.2
                    && rel_y >= notch.visible_bounds.1
                    && rel_y <= notch.visible_bounds.3
                {
                    return HTCLIENT;
                }
            }
            HTTRANSPARENT
        }
        WM_MOUSEMOVE => {
            let x = (lparam.0 & 0xFFFF) as i16 as f32;
            let y = ((lparam.0 >> 16) & 0xFFFF) as i16 as f32;
            let mut tme = TRACKMOUSEEVENT {
                cbSize: std::mem::size_of::<TRACKMOUSEEVENT>() as u32,
                dwFlags: TME_LEAVE,
                hwndTrack: hwnd,
                dwHoverTime: 0,
            };
            unsafe {
                let _ = TrackMouseEvent(&mut tme);
            }
            if let Some(notch_ptr) = unsafe { CURRENT_NOTCH } {
                let notch = unsafe { &mut *notch_ptr };
                let gfx = Gfx::new().ok();
                if let Some(g) = gfx {
                    let _ = notch.handle_mouse_move(&g, x, y);
                }
            }
            LRESULT(0)
        }
        WM_MOUSELEAVE => {
            if let Some(notch_ptr) = unsafe { CURRENT_NOTCH } {
                let notch = unsafe { &mut *notch_ptr };
                let gfx = Gfx::new().ok();
                if let Some(g) = gfx {
                    let _ = notch.handle_mouse_leave(&g);
                }
            }
            LRESULT(0)
        }
        WM_LBUTTONDOWN => {
            let x = (lparam.0 & 0xFFFF) as i16 as f32;
            let y = ((lparam.0 >> 16) & 0xFFFF) as i16 as f32;
            if let Some(notch_ptr) = unsafe { CURRENT_NOTCH } {
                let notch = unsafe { &mut *notch_ptr };
                let gfx = Gfx::new().ok();
                if let Some(g) = gfx {
                    let _ = notch.handle_click(&g, x, y);
                }
            }
            LRESULT(0)
        }
        MEDIA_CHANGED => {
            if let Some(notch_ptr) = unsafe { CURRENT_NOTCH } {
                let notch = unsafe { &mut *notch_ptr };
                let gfx = Gfx::new().ok();
                if let Some(g) = gfx {
                    let _ = notch.media_changed(&g);
                }
            }
            LRESULT(0)
        }
        WM_TIMER => {
            if wparam.0 == CHARGING_TIMER_ID {
                unsafe {
                    KillTimer(Some(hwnd), CHARGING_TIMER_ID);
                }
                if let Some(notch_ptr) = unsafe { CURRENT_NOTCH } {
                    let notch = unsafe { &mut *notch_ptr };
                    notch.charging = false;
                    let gfx = Gfx::new().ok();
                    if let Some(g) = gfx {
                        let _ = notch.draw(&g);
                    }
                }
            } else if wparam.0 == IDLE_EYES_TIMER_ID {
                if let Some(notch_ptr) = unsafe { CURRENT_NOTCH } {
                    let notch = unsafe { &mut *notch_ptr };
                    if notch.idle_eyes_enabled && notch.track.is_none() && !notch.expanded && !notch.charging {
                        let gfx = Gfx::new().ok();
                        if let Some(g) = gfx {
                            let _ = notch.draw(&g);
                        }
                    }
                }
            }
            LRESULT(0)
        }
        _ => unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) },
    }
}
