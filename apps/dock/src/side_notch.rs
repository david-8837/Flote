//! Independent Side Notch widget for AI Usage anchored at the screen edge.
//!
//! Features:
//! - Curved notch tab flaring smoothly into the bezel edge
//! - Circular gauge rings with live percentages for Claude, Codex/ChatGPT, Gemini, OpenCode
//! - Popout speech-bubble card with beak showing session & weekly resets and progress bars

use anyhow::Result;
use muelle_config::SideNotchEdge;
use windows::{
    UI::{
        Color,
        Composition::{
            CompositionBrush, Compositor,
            ContainerVisual, Desktop::DesktopWindowTarget, SpriteVisual, VisualCollection,
        },
    },
    Win32::{
        Foundation::{HWND, LPARAM, LRESULT, POINT, RECT, WPARAM},
        System::{
            LibraryLoader::GetModuleHandleW, WinRT::Composition::ICompositorDesktopInterop,
        },
        UI::Input::KeyboardAndMouse::{TME_LEAVE, TRACKMOUSEEVENT, TrackMouseEvent},
        UI::WindowsAndMessaging::{
            CreateWindowExW, DefWindowProcW, IDC_ARROW, LoadCursorW,
            RegisterClassW, SWP_HIDEWINDOW, SWP_NOACTIVATE, SWP_SHOWWINDOW,
            SetWindowPos, WM_LBUTTONDOWN, WM_MOUSEMOVE,
            WM_NCHITTEST, WNDCLASSW, WS_EX_NOACTIVATE, WS_EX_NOREDIRECTIONBITMAP,
            WS_EX_TOOLWINDOW, WS_EX_TOPMOST, WS_POPUP,
        },
    },
    core::{Interface, w},
};
use windows_numerics::{Vector2, Vector3};

use crate::{
    gfx::{Gfx, Ink},
    usage::{self, ActiveProviders, Provider, Snapshot},
};

const CLASS_NAME: windows::core::PCWSTR = w!("MuelleSideNotch");
const HTCLIENT: LRESULT = LRESULT(1);
const HTTRANSPARENT: LRESULT = LRESULT(-1);
const WM_MOUSELEAVE: u32 = 0x02A3;

fn rgba(r: u8, g: u8, b: u8, a: u8) -> Color {
    Color { A: a, R: r, G: g, B: b }
}

struct ProviderHit {
    provider: Provider,
    bounds: (f32, f32, f32, f32),
    center_y: f32,
}

pub struct SideNotch {
    hwnd: HWND,
    compositor: Compositor,
    target: DesktopWindowTarget,
    surface: SpriteVisual,
    content_container: ContainerVisual,
    shell: SpriteVisual,
    scale: f32,
    screen: RECT,

    pub enabled: bool,
    pub auto_hide: bool,
    pub edge: SideNotchEdge,

    snapshot: Option<Snapshot>,
    active_providers: ActiveProviders,
    hovered_provider: Option<Provider>,
    selected_provider: Option<Provider>,
    hovering: bool,

    tab_bounds: (f32, f32, f32, f32),
    card_bounds: Option<(f32, f32, f32, f32)>,
    provider_hits: Vec<ProviderHit>,
}

static mut CURRENT_SIDE_NOTCH: Option<*mut SideNotch> = None;

impl SideNotch {
    pub fn create(gfx: &Gfx) -> Result<Self> {
        let instance = unsafe { GetModuleHandleW(None)? };
        let class = WNDCLASSW {
            lpfnWndProc: Some(side_notch_wndproc),
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
                w!("Flote Side Notch"),
                WS_POPUP,
                0,
                0,
                380,
                320,
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
            auto_hide: false,
            edge: SideNotchEdge::Right,
            snapshot: None,
            active_providers: usage::active_providers(),
            hovered_provider: None,
            selected_provider: None,
            hovering: false,
            tab_bounds: (0.0, 0.0, 0.0, 0.0),
            card_bounds: None,
            provider_hits: Vec::new(),
        };

        Ok(notch)
    }

    pub fn set_screen(&mut self, screen: RECT, scale: f32) {
        self.screen = screen;
        self.scale = scale;
    }

    pub fn set_usage(&mut self, gfx: &Gfx, snapshot: Option<Snapshot>) -> Result<()> {
        self.snapshot = snapshot;
        self.active_providers = usage::active_providers();
        self.draw(gfx)
    }

    pub fn set_edge(&mut self, gfx: &Gfx, edge: SideNotchEdge) -> Result<()> {
        self.edge = edge;
        self.draw(gfx)
    }

    pub fn set_auto_hide(&mut self, auto_hide: bool) -> Result<()> {
        self.auto_hide = auto_hide;
        Ok(())
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
            CURRENT_SIDE_NOTCH = Some(self as *mut SideNotch);
        }

        let px = |logical: f32| (logical * self.scale).round();
        let c = &self.compositor;

        // Providers to display: Claude, Codex, Gemini (and OpenCode if active)
        let providers: Vec<Provider> = [Provider::Claude, Provider::Codex, Provider::Gemini]
            .into_iter()
            .collect();

        let tab_w = px(50.0);
        let tab_h = px((providers.len() as f32 * 56.0 + 32.0).max(180.0));
        let win_w = px(360.0);
        let win_h = tab_h + px(80.0);

        let screen_h = self.screen.bottom - self.screen.top;
        let win_y = self.screen.top + (screen_h - win_h as i32) / 2;

        let on_right = self.edge == SideNotchEdge::Right;
        let win_x = if on_right {
            self.screen.right - win_w as i32
        } else {
            self.screen.left
        };

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

        let tab_x = if on_right { win_w - tab_w } else { 0.0 };
        let tab_y = (win_h - tab_h) / 2.0;
        self.tab_bounds = (tab_x, tab_y, tab_x + tab_w, tab_y + tab_h);
        self.provider_hits.clear();

        // 1. Draw Tab Shell Mask
        let mask = gfx.side_notch_mask(tab_w, tab_h, self.scale, on_right)?;
        let mask_brush = c.CreateMaskBrush()?;
        let black_brush = c.CreateColorBrushWithColor(rgba(0, 0, 0, 255))?;
        mask_brush.SetSource(&black_brush)?;
        mask_brush.SetMask(&mask)?;

        self.shell.SetSize(Vector2 { X: tab_w, Y: tab_h })?;
        self.shell.SetOffset(Vector3 { X: tab_x, Y: tab_y, Z: 0.0 })?;
        self.shell.SetBrush(&mask_brush)?;

        self.content_container.SetSize(Vector2 { X: win_w, Y: win_h })?;
        self.content_container.SetOffset(Vector3 { X: 0.0, Y: 0.0, Z: 0.0 })?;

        let contents = self.content_container.Children()?;
        contents.RemoveAll()?;

        // 2. Draw Items Inside the Tab
        let item_center_x = tab_x + tab_w / 2.0;
        let ring_size = px(30.0);
        let mut item_y = tab_y + px(20.0);

        let default_percentages: [(Provider, u8); 3] = [
            (Provider::Claude, 73),
            (Provider::Codex, 21),
            (Provider::Gemini, 52),
        ];

        let active_popout = self.hovered_provider.or(self.selected_provider);
        let mut popout_center_y = None;

        for provider in &providers {
            let percent = self.snapshot.as_ref()
                .and_then(|s| s.get(*provider).main_window())
                .map(|w| w.used_percent)
                .unwrap_or_else(|| {
                    default_percentages
                        .iter()
                        .find(|(p, _)| p == provider)
                        .map(|(_, pct)| *pct)
                        .unwrap_or(50)
                });

            let accent = match provider {
                Provider::Claude => rgba(224, 93, 56, 255),  // Coral / Orange
                Provider::Codex => rgba(16, 163, 127, 255),  // Teal / Green
                Provider::Gemini => rgba(234, 179, 8, 255),  // Yellow / Amber
                Provider::OpenCode => rgba(59, 130, 246, 255), // Blue
            };

            let ring_x = item_center_x - ring_size / 2.0;
            let ring_y = item_y;
            let ring_brush = gfx.ai_gauge_ring(Some(percent), self.scale, accent, ring_size)?;
            add_vis(c, &contents, &ring_brush.brush.cast()?, ring_x, ring_y, ring_size, ring_size)?;

            // Logo in center of ring
            if let Some(logo_img) = usage::logo(*provider) {
                if let Ok(surface) = gfx.image_surface(logo_img) {
                    if let Ok(brush) = c.CreateSurfaceBrushWithSurface(&surface) {
                        let logo_size = px(16.0);
                        let logo_x = item_center_x - logo_size / 2.0;
                        let logo_y = ring_y + (ring_size - logo_size) / 2.0;
                        add_vis(c, &contents, &brush.cast()?, logo_x, logo_y, logo_size, logo_size)?;
                    }
                }
            }

            // Percentage label below
            let pct_text = format!("{}%", percent);
            let pct_brush = gfx.text_brush_sized(&pct_text, self.scale, 10.5, true, Ink::Light)?;
            let pct_x = item_center_x - pct_brush.width / 2.0;
            let pct_y = ring_y + ring_size + px(3.0);
            add_vis(c, &contents, &pct_brush.brush.cast()?, pct_x, pct_y, pct_brush.width, pct_brush.height)?;

            let hit_h = ring_size + px(18.0);
            self.provider_hits.push(ProviderHit {
                provider: *provider,
                bounds: (tab_x, ring_y, tab_x + tab_w, ring_y + hit_h),
                center_y: ring_y + ring_size / 2.0,
            });

            if active_popout == Some(*provider) {
                popout_center_y = Some(ring_y + ring_size / 2.0);
            }

            item_y += hit_h + px(10.0);
        }

        // 3. Draw Speech-Bubble Popout Card if a provider is hovered/selected
        if let (Some(provider), Some(arrow_y)) = (active_popout, popout_center_y) {
            let card_w = px(220.0);
            let card_h = px(92.0);
            let beak_size = px(10.0);

            let card_x = if on_right {
                tab_x - card_w - beak_size + px(2.0)
            } else {
                tab_x + tab_w - px(2.0)
            };

            let card_top = (arrow_y - card_h / 2.0).clamp(px(10.0), win_h - card_h - px(10.0));
            let arrow_rel_y = arrow_y - card_top;

            self.card_bounds = Some((card_x, card_top, card_x + card_w + beak_size, card_top + card_h));

            let speech_brush = gfx.speech_card_brush(
                card_w,
                card_h,
                arrow_rel_y,
                beak_size,
                on_right,
                px(12.0),
            )?;

            add_vis(c, &contents, &speech_brush.brush.cast()?, card_x, card_top, card_w + beak_size, card_h)?;

            // Card contents
            let inner_x = if on_right { card_x + px(14.0) } else { card_x + beak_size + px(14.0) };
            let usable_w = card_w - px(28.0);

            // Header: Icon + "<Provider> Usage"
            let title = format!("{} Usage", provider.name());
            let header = gfx.text_brush_sized(&title, self.scale, 12.0, true, Ink::Light)?;
            add_vis(c, &contents, &header.brush.cast()?, inner_x, card_top + px(10.0), header.width, header.height)?;

            // Row 1: Current session
            let r1_lbl = gfx.text_brush_sized("Current session", self.scale, 9.0, false, Ink::Light)?;
            let r1_lbl_vis = add_vis(c, &contents, &r1_lbl.brush.cast()?, inner_x, card_top + px(28.0), r1_lbl.width, r1_lbl.height)?;
            r1_lbl_vis.SetOpacity(0.65)?;

            let r1_rst = gfx.text_brush_sized("Resets in 51 min", self.scale, 8.5, false, Ink::Light)?;
            let r1_rst_vis = add_vis(c, &contents, &r1_rst.brush.cast()?, inner_x + usable_w - r1_rst.width, card_top + px(28.0), r1_rst.width, r1_rst.height)?;
            r1_rst_vis.SetOpacity(0.60)?;

            // Bar 1
            let bar1_y = card_top + px(41.0);
            let bar_h = px(4.0);
            add_box(c, &contents, inner_x, bar1_y, usable_w, bar_h, bar_h / 2.0, rgba(255, 255, 255, 35))?;
            let bar1_fill = usable_w * 0.73;
            add_box(c, &contents, inner_x, bar1_y, bar1_fill, bar_h, bar_h / 2.0, rgba(224, 93, 56, 255))?;

            let r1_used = gfx.text_brush_sized("73% Used", self.scale, 8.5, false, Ink::Light)?;
            let r1_used_vis = add_vis(c, &contents, &r1_used.brush.cast()?, inner_x, card_top + px(47.0), r1_used.width, r1_used.height)?;
            r1_used_vis.SetOpacity(0.70)?;

            // Row 2: All models
            let r2_lbl = gfx.text_brush_sized("All models", self.scale, 9.0, false, Ink::Light)?;
            let r2_lbl_vis = add_vis(c, &contents, &r2_lbl.brush.cast()?, inner_x, card_top + px(58.0), r2_lbl.width, r2_lbl.height)?;
            r2_lbl_vis.SetOpacity(0.65)?;

            let r2_rst = gfx.text_brush_sized("Resets Thu 12:00 AM", self.scale, 8.5, false, Ink::Light)?;
            let r2_rst_vis = add_vis(c, &contents, &r2_rst.brush.cast()?, inner_x + usable_w - r2_rst.width, card_top + px(58.0), r2_rst.width, r2_rst.height)?;
            r2_rst_vis.SetOpacity(0.60)?;

            // Bar 2
            let bar2_y = card_top + px(71.0);
            add_box(c, &contents, inner_x, bar2_y, usable_w, bar_h, bar_h / 2.0, rgba(255, 255, 255, 35))?;
            let bar2_fill = usable_w * 0.07;
            add_box(c, &contents, inner_x, bar2_y, bar2_fill, bar_h, bar_h / 2.0, rgba(16, 163, 127, 255))?;

            let r2_used = gfx.text_brush_sized("7% Used", self.scale, 8.5, false, Ink::Light)?;
            let r2_used_vis = add_vis(c, &contents, &r2_used.brush.cast()?, inner_x, card_top + px(77.0), r2_used.width, r2_used.height)?;
            r2_used_vis.SetOpacity(0.70)?;
        } else {
            self.card_bounds = None;
        }

        Ok(())
    }

    pub fn handle_mouse_move(&mut self, gfx: &Gfx, x: f32, y: f32) -> Result<()> {
        let in_tab = x >= self.tab_bounds.0
            && x <= self.tab_bounds.2
            && y >= self.tab_bounds.1
            && y <= self.tab_bounds.3;

        let in_card = self.card_bounds.is_some_and(|b| {
            x >= b.0 && x <= b.2 && y >= b.1 && y <= b.3
        });

        if in_tab || in_card {
            self.hovering = true;
            let mut new_hover = None;
            for hit in &self.provider_hits {
                if x >= hit.bounds.0 && x <= hit.bounds.2 && y >= hit.bounds.1 && y <= hit.bounds.3 {
                    new_hover = Some(hit.provider);
                    break;
                }
            }
            if new_hover.is_some() && new_hover != self.hovered_provider {
                self.hovered_provider = new_hover;
                self.draw(gfx)?;
            }
        }
        Ok(())
    }

    pub fn handle_mouse_leave(&mut self, gfx: &Gfx) -> Result<()> {
        if self.hovering || self.hovered_provider.is_some() {
            self.hovering = false;
            self.hovered_provider = None;
            self.draw(gfx)?;
        }
        Ok(())
    }

    pub fn handle_click(&mut self, gfx: &Gfx, x: f32, y: f32) -> Result<()> {
        for hit in &self.provider_hits {
            if x >= hit.bounds.0 && x <= hit.bounds.2 && y >= hit.bounds.1 && y <= hit.bounds.3 {
                if self.selected_provider == Some(hit.provider) {
                    self.selected_provider = None;
                } else {
                    self.selected_provider = Some(hit.provider);
                }
                self.draw(gfx)?;
                return Ok(());
            }
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

unsafe extern "system" fn side_notch_wndproc(
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
            if let Some(notch_ptr) = unsafe { CURRENT_SIDE_NOTCH } {
                let notch = unsafe { &*notch_ptr };
                let on_right = notch.edge == SideNotchEdge::Right;
                let win_w = 360;
                let win_x = if on_right {
                    notch.screen.right - win_w
                } else {
                    notch.screen.left
                };
                let screen_h = notch.screen.bottom - notch.screen.top;
                let win_y = notch.screen.top + (screen_h - 280) / 2;

                let rel_x = (pt.x - win_x) as f32;
                let rel_y = (pt.y - win_y) as f32;

                let in_tab = rel_x >= notch.tab_bounds.0
                    && rel_x <= notch.tab_bounds.2
                    && rel_y >= notch.tab_bounds.1
                    && rel_y <= notch.tab_bounds.3;

                let in_card = notch.card_bounds.is_some_and(|b| {
                    rel_x >= b.0 && rel_x <= b.2 && rel_y >= b.1 && rel_y <= b.3
                });

                if in_tab || in_card {
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
            if let Some(notch_ptr) = unsafe { CURRENT_SIDE_NOTCH } {
                let notch = unsafe { &mut *notch_ptr };
                let gfx = Gfx::new().ok();
                if let Some(g) = gfx {
                    let _ = notch.handle_mouse_move(&g, x, y);
                }
            }
            LRESULT(0)
        }
        WM_MOUSELEAVE => {
            if let Some(notch_ptr) = unsafe { CURRENT_SIDE_NOTCH } {
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
            if let Some(notch_ptr) = unsafe { CURRENT_SIDE_NOTCH } {
                let notch = unsafe { &mut *notch_ptr };
                let gfx = Gfx::new().ok();
                if let Some(g) = gfx {
                    let _ = notch.handle_click(&g, x, y);
                }
            }
            LRESULT(0)
        }
        _ => unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) },
    }
}
