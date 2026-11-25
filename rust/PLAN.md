# MacBook Dynamic Notch Recreation — Rust Implementation Plan

**Purpose:** A complete, self-contained plan so an AI coding agent (or a human)
can build the entire app from scratch in Rust. Read this top-to-bottom and
execute the phases in order. Each phase has a goal, concrete tasks, files to
create/modify, and acceptance criteria.

**Target:** A native, GPU-rendered macOS-style dynamic notch for **Windows 10/11**,
built with **winit + wgpu**. No web engine, no background browser. Draws only
when something moves. Event-driven. Sharp at any display scale.

---

## 0. Ground rules (do not violate these)

These are the native-parity rules that the recreation must honor. They are
checked in every phase:

1. **GPU-drawn only.** All rendering goes through wgpu. No webview, no browser
   engine living in the background.
2. **Draw only when something moves.** The event loop waits (`ControlFlow::Wait`)
   when idle. A `dirty` flag + scheduled redraws drive frames only on state
   change or during active animation. No idle redraw loop.
3. **Event-driven.** Wait for events/timers, don't busy-poll. Use
   `ControlFlow::WaitUntil(next_deadline)` for scheduled animation/timer work.
4. **Files stay where they are.** The "Shelf" feature remembers file locations;
   moving/closing windows never loses the user's file context.
5. **Steps aside for full-screen apps.** When a full-screen app is detected, the
   notch hides/moves aside and returns when it's gone.
6. **Sharp at any scale.** Render at physical pixel resolution; the 1964×1124
   design stage is scaled to the viewport with a clamp (see §4.6).
7. **Windows 10 and 11.** Use Win32 window attributes; no macOS-only APIs.

---

## 1. Technology stack (what to use)

| Concern | Crate | Version | Notes |
|---|---|---|---|
| Window + event loop | `winit` | `0.30` | Borderless topmost transparent window |
| GPU rendering | `wgpu` | `22` | Vulkan/Metal/DX12 backend |
| Async init (block_on) | `pollster` | `0.3` | Run the wgpu init future to completion |
| Zero-copy buffers | `bytemuck` | `1` | derive `Pod`/`Zeroable` for instance/uniform structs |
| Polygon triangulation | `earcut` | `0.3` | Icon/text-background fills |
| Text shaping | `rustybuzz` | `0.6` | OpenType/GSUB/GPOS shaping (via glyphon) |
| Text rasterization | `glyphon` | `0.5` | Text on wgpu (uses rustybuzz + wgpu) |
| SVG icon rasterization | `resvg` + `tiny-skia` | `0.40` / `0.12` | Parse SVG → rasterize to a wgpu texture |
| Settings persistence | `serde` + `serde_json` | `1` | Save/load a JSON config in `%APPDATA%` |
| Easing (optional) | custom module | — | Implement `cubic-bezier` easing in ~30 lines; no dep needed |
| Win32 (fullscreen detect, window props) | `windows-sys` | `0.59` | `Win32::Foundation`, `Win32::UI::WindowsAndMessaging`, `Win32::System::Win32` |

**Do not add:** Bevy, Pixels, Tauri, any webview, any GUI framework (egui/iced/
slint) for the notch itself. The render core is hand-rolled wgpu (§4). GUI
frameworks are only acceptable if they render to wgpu — but the plan below
builds the UI with the same SDF render core, so no GUI framework is needed.

**Version pinning:** pin the versions above. wgpu/winit APIs drift between
minor releases; if a build fails on an API mismatch, the fix is usually a small
adapter, not a rewrite.

---

## 2. Architecture at a glance

```
┌─────────────────────────────────────────────────────────────┐
│  Event Loop (winit)                                          │
│   ├─ State machine: idle / animating / modal (settings)      │
│   ├─ dirty flag + scheduled redraw (WaitUntil)               │
│   └─ Input: mouse (popover, settings), keyboard, timers      │
├─────────────────────────────────────────────────────────────┤
│  Scene Model (app state)                                     │
│   ├─ NotchState: mode, animation progress, gauges, lyric... │
│   ├─ DockState: usage, popover open, hover                  │
│   ├─ Settings: all ~18 settings (serde)                     │
│   └─ Clipboard / file-shelf state                             │
├─────────────────────────────────────────────────────────────┤
│  Render Core (wgpu, immediate-mode draw list)                │
│   ├─ Fill pipeline  (SDF: rect/circle/triangle, gradients)   │
│   ├─ Stroke pipeline (SDF: lines/beziers/arcs)               │
│   ├─ Text pipeline (glyphon)                                │
│   ├─ Icon pipeline (resvg→tiny-skia→texture)                │
│   ├─ Clip stack (rounded-rect masking)                      │
│   └─ Stage transform (1964×1124 scaled to viewport)          │
└─────────────────────────────────────────────────────────────┘
```

The app is a **single borderless topmost window** covering the top strip of
the primary screen. All visuals (notch, dock, popover, settings) are drawn into
that one window at stage coordinates. There is no second window.

---

## 3. Project structure

```
rust/
├── Cargo.toml
├── README.md
├── PLAN.md                       ← this file
└── src/
    ├── main.rs                   ← entry, event loop, state machine, redraw scheduling
    ├── config.rs                 ← Settings struct (serde), load/save, defaults
    ├── stage.rs                  ← Stage/fit-scale math, coordinate helpers
    ├── easing.rs                 ← cubic-bezier + easing functions
    ├── anim.rs                   ← animation timeline/tween helpers, 43 animations
    ├── state/
    │   ├── mod.rs                ← AppState, NotchState, DockState
    │   ├── modes.rs              ← 17 notch modes enum + transition logic
    │   ├── companion.rs          ← companion AI state machine
    │   └── shelf.rs              ← file-shelf memory (files stay where they are)
    ├── input.rs                  ← mouse/keyboard handling, hit-testing
    ├── render/
    │   ├── mod.rs                ← Renderer, DrawList, instance packing, clip stack
    │   ├── shaders.rs            ← WGSL fill + stroke shaders
    │   ├── shapes.rs             ← draw-op helpers (rects, circles, gradients, strokes)
    │   ├── path.rs               ← bezier + arc tessellation
    │   ├── text.rs               ← glyphon text pass
    │   └── icons.rs              ← resvg→tiny-skia→texture icon pass
    ├── notch/
    │   ├── mod.rs                ← notch shell rendering + mode dispatch
    │   ├── modes.rs              ← per-mode layouts (resting, notification, expanded, lyric...)
    │   └── aurora.rs             ← aurora resting glow (4.2s)
    ├── dock/
    │   ├── mod.rs                ← side dock body, gauges, popover
    │   ├── gauges.rs             ← three 36px usage gauges
    │   └── popover.rs            ← 214px popover panel
    ├── settings/
    │   ├── mod.rs                ← settings UI render + interaction (5 sections)
    │   └── widgets.rs            ← toggles, sliders, selectors, headers
    └── fullscreen.rs             ← Win32 full-screen detection (step-aside)
```

**Phase 2a (render core) is already implemented** in `render/{mod,shaders,shapes,path}.rs`
and `main.rs` (verification scene). Build on it; don't rewrite the pipelines.

---

## 4. Render core (Phase 2a — DONE, documented for reference)

This is the foundation everything else uses. It is already written and pushed.
If your IDE starts from the pushed branch, this phase is complete. If starting
from scratch, replicate it.

### 4.1 Two instanced pipelines
Both pipelines share one **corner fullscreen triangle** vertex buffer
`(0,0) (1,0) (0,1)` and expand each triangle to the shape via instancing.

- **Fill pipeline** — instance stride **144 B** = 9×`vec4f`:
  `pos@0, size@4, color@8, color2@12, grad@16, radius@20, edges0@24, edges1@28, edges2@32`
- **Stroke pipeline** — instance stride **64 B** = 4×`vec4f`:
  `start@0, end@4, (thickness,r,g,b)@8, (a,0,0,0)@12`

> **Critical:** vertex-attribute `shader_location` and byte `offset` MUST match
> the WGSL `array<vec4f, N>` interpretation exactly. Location 0 is the corner
> (per-vertex); instance attributes start at location 1. Each successive
> `vec4f` is +16 bytes. A mismatch here silently corrupts rendering — verify with
> the verification scene before building on it.

### 4.2 Fill shader (SDF)
- `sd_rounded_rect`, `sd_circle`, `sd_triangle` (convex polygon SDF = max of
  edge distances).
- Anti-aliasing in physical pixels: `aa = fwidth(d) * max(view.x, view.y)`.
- Gradients: `0=solid`, `1=linear` (dir in local 0..1 coords), `2=radial`.
- Clip stack: up to 4 rounded-rect regions in a uniform buffer; multiply alpha
  by each active clip's SDF.

### 4.3 Stroke shader (SDF)
- Segment SDF: `sd_segment(p,a,b)` with thickness; AA same as fill.
- Round caps/joins: the CPU tessellates beziers/arcs into segments and draws
  overlapping circles at joints (in `shapes.rs`).
- Clip stack applied identically to fills.

### 4.4 Clip stack
`MAX_CLIPS = 4`. A `ClipRect { x, y, w, h, radius }` per level in a uniform
buffer. `DrawList` records `ClipPush`/`ClipPop`. The renderer flushes a batch
when the clip depth changes, using a **snapshot** of the clip stack the batch
was built under (not the live stack).

### 4.5 Blend
`src-alpha` over `dst` for color; `One` over `OneMinusSrcAlpha` for alpha.
Background clear = `#050507`.

### 4.6 Stage transform
Design stage is **1964×1124** (15.6"). `fit_scale(size)`:
`w = size.width * 0.96 / 1964`, `h = (size.height - 190) / 1124`,
`scale = clamp(min(w,h), 0.2, 1.0)`. The stage is centered; `set_view(origin_x,
origin_y, scale)` uploads `(scale, scale, origin_x, origin_y)` to the uniform
buffer. Render at physical resolution → sharp at any DPI.

### 4.7 Verification scene (in main.rs)
Wallpaper (screen gradient + petal radial gradients + horizon), placeholder notch
shell, and primitive tests (arc ring, circle, line, bezier, triangle). **This is
the acceptance test for Phase 2a**: it must compile and render these correctly.

---

## 5. Phase 2b — Text rendering

**Goal:** Render text (notch labels, lyric lines, settings labels, gauge
percentages) on wgpu.

**Dependencies:** add `glyphon = "0.5"`, `rustybuzz = "0.6"` to `Cargo.toml`.

**Tasks:**
1. Create `render/text.rs`. Wrap `glyphon::TextRenderer` (a `wgpu` text renderer
   that uses rustybuzz for shaping).
2. Add a **text pipeline** to the `Renderer`: a glyph atlas texture (managed by
   glyphon) + an instanced glyph draw. Draw glyph quads as filled rects with
   the glyph's UV and color, through a small instanced pipeline (or reuse the
   fill pipeline with a texture bind group — glyphon provides its own layout;
   follow glyphon's documented wgpu integration).
3. Add `DrawList` text ops: `text(x, y, font_size, color, string, align)` and a
   multi-line variant. Support **left / center / right** alignment and a
   baseline-anchored y.
4. Font: bundle a system font (e.g., load a TTF/OTF from a `fonts/` dir, or use
   `glyphon`'s ability to load from bytes). For macOS-like look, a rounded
   sans (e.g., Inter or system) at the sizes the spec calls for.
5. Metrics: implement line wrapping at a max width (for lyric lines and
   settings descriptions).

**Acceptance:** The verification scene shows a few lines of text (a notch label
"MacBook", a lyric line, a percentage "73%") rendered crisply, sharp at 125% and
200% scale.

---

## 6. Phase 2c — Icon rendering

**Goal:** Render the icons used in the notch (mode icons, companion face,
settings glyphs, dock app icons) from SVG.

**Dependencies:** add `resvg = "0.40"`, `tiny-skia = "0.12"` to `Cargo.toml`.

**Tasks:**
1. Create `render/icons.rs`. Build an **icon atlas**: a cached wgpu texture that
   packs many small icons (e.g., 512×512 atlas, each icon 64×64 or 128×128).
2. On first use (or lazily), parse each needed SVG with `resvg`, rasterize with
   `tiny-skia` at the atlas resolution (multiple sizes for sharpness), and write
   pixels into the atlas texture. Track each icon's UV rect.
3. Add a **icon fill op**: `icon(x, y, size, icon_id, color_override)` — draws a
   quad with the icon's UV, tinted if needed. Reuse the fill pipeline with a
   texture bind group (a second bind group layout for the atlas texture, or a
   combined layout).
4. Provide the SVG source for every icon the app uses (mode icons, companion
   expressions, settings glyphs, dock gauge icons). Store under `assets/icons/*.svg`.

**Acceptance:** The verification scene shows 3–4 icons (e.g., a battery, a
notification bell, a gauge) rendered correctly, tinted, and sharp at scale.

---

## 7. Phase 3 — Top notch (17 modes + companion AI + 43 animations)

**Goal:** The main notch: a centered black pill at the top of the screen that
morphs between 17 modes, driven by a companion AI state machine, with 43
animations.

### 7.1 Notch shell & geometry (reference spec)
- Shell is **centered** at the top of the screen.
- Scale factor `--notch-scale: 1.6` (sizes below are design units; multiply by
  scale when laying out relative to the stage).
- Background **`#08080a`** (near-black).
- Ears (side fillets): **26px** radius, shrinking to **14px** at transitions.
- Resting **aurora glow**: a soft animated glow cycle, **4.2s** period.

### 7.2 Sizes by mode (design units, width × height, radius)
| Size | W×H | Radius | Modes |
|---|---|---|---|
| Small | 154×32 | 13 | resting, waving, sleeping, vibing, surprised |
| Medium | 332×68 | 24 | notification, charging, headphones |
| Large | 416×138 | 34 | expanded |
| Lyric | 454×138 | 34 | lyrics |

(The full 17-mode catalogue is in §11.1 — confirm the exact list against the
reference app before finalizing; the sizes above are the confirmed ones.)

### 7.3 Modes enum (`state/modes.rs`)
Define `enum NotchMode` with all 17 variants. Each mode has:
- a target size (from §7.2),
- a set of animated elements (face/eyes, icon, label, glow, gauges),
- entry/exit animations.

### 7.4 Companion AI (`state/companion.rs`)
A state machine for the "companion" — the animated face/expression living in the
notch. States drive the face (eyes, mouth, blush, etc.) and tie into modes:
- Idle / listening / thinking / speaking / happy / surprised / sleeping /
  vibing, etc.
- The companion reacts to user interactions and to system events. Implement as
  a small FSM with timers (no external AI dependency needed for the look; the
  "AI" feel is the expression animation).

### 7.5 Animations (`anim.rs`) — 43 animations
Implement a tiny animation framework:
- `Tween { from, to, duration, easing, on_update }`.
- Easing: implement `cubic-bezier(x1,y1,x2,y2)` (the two curves used everywhere:
  **`(0.22,1,0.36,1)`** for shell/hover (0.65s), **`(0.32,1.18,0.42,1)`** for
  body morph (0.52s)). Also `ease-in-out`, `ease-out`, `ease-in`, linear.
- A `Animator` that owns active tweens, advances by `Duration` deltas, and sets
  the `dirty` flag each step.

The 43 animations fall into groups (implement all):
- **Shell transitions**: size morph between mode sizes (0.52s body morph),
  ear radius morph (26px→14px), position settle.
- **Companion face**: blink, look-left/right, smile, frown, wide-eye, wink,
  blush, mouth open/close, sleeping zz, thinking dots, speaking mouth cycles.
- **Aurora glow**: the 4.2s resting glow pulse.
- **Mode-specific**: notification badge pop, charging pulse, headphones bob,
  lyric line reveal, expanded panel slide, recording level bars, etc.
- **Hover/press**: shell lift, ripple, press scale.

**Acceptance:** The notch cycles through resting → notification → expanded →
lyrics → resting, each transition smooth at the specified easing/timing, with the
companion face animating. No stutter; idle state draws nothing new (no redraw
loop when still).

---

## 8. Phase 4 — Side dock (AI usage dock, gauges, popover)

**Goal:** A side notch on the left edge: an AI usage dock with three gauges and
a popover.

### 8.1 Dock body (reference spec)
- Body height **56px**, background **`#050507`**, radius **`22px 0 0 22px`**
  (rounded top-left/top-right, square bottom).
- Sits on the **left** edge of the screen (top-aligned with the notch strip).

### 8.2 Gauges (`dock/gauges.rs`)
Three circular gauges, each **36px** diameter:
- **Claude 73%**, **ChatGPT 21%**, **Gemini 52%** (default values; these are
  the "AI usage" indicators).
- Each gauge: a track ring + a progress arc (use `stroke_arc` from the render
  core). Animate the arc on value change. Show the percentage text (Phase 2b)
  in the center.

### 8.3 Popover (`dock/popover.rs`)
- **214px** wide popover panel that appears on hover/click on the dock.
- Shows expanded usage detail (per-AI breakdown, recent activity, quick actions).
- Animate open/close (scale + opacity, 0.2s ease-out). Click-outside or Esc
  closes it. Hit-test via `input.rs`.

**Acceptance:** Dock renders on the left edge with three animated gauges;
hovering opens the popover smoothly; clicking outside closes it.

---

## 9. Phase 5 — Settings UI (~18 settings, 5 sections)

**Goal:** A full settings panel opened from the notch, with 5 sections and
~18 settings, persisted to disk.

### 9.1 Settings model (`config.rs`)
A `Settings` struct (serde) with all ~18 settings. Persist to
`%APPDATA%\MacBookNotch\settings.json` (create the dir). Load on startup, save
on change. Provide `Default`.

### 9.2 Sections & settings (~18)
Five sections (match the reference app):
1. **Notch** — e.g., notch scale, resting glow on/off, auto-hide in fullscreen,
   mode defaults, animation speed.
2. **Companion** — companion on/off, expression intensity, name, voice/feedback
   toggles.
3. **Side Dock** — dock on/off, gauge count/which AIs, popover default open,
   dock position.
4. **Widgets** — which widgets are enabled, lyric source, clock format.
5. **Appearance** — accent color, dark/light, transparency, border radius.

(Exact setting names/labels: confirm against the reference app; the count and
sections above are the confirmed structure.)

### 9.3 Settings UI (`settings/mod.rs`, `settings/widgets.rs`)
Render the settings panel **with the same render core** (no GUI framework):
- A centered or side panel with section headers, rows, and controls.
- Controls (`widgets.rs`): **toggle** (switch), **slider** (range), **selector**
  (segmented/dropdown), **color picker**, **header**, **label**. All drawn with
  fills/strokes/text/icons from the render core.
- Mouse interaction: hover highlight, click to toggle/drag slider, scroll for
  long lists. Hit-test in `input.rs`.
- Open/close animation (slide + fade).

**Acceptance:** All 5 sections render with all ~18 settings; toggling/sliders
update both the visual and the persisted JSON; reopening the app restores the
saved values.

---

## 10. Phase 6 — Polish & native parity

**Goal:** Tie it all together to match the native feel.

### 10.1 Event-driven redraw (finalize)
- `dirty` flag set on any state change, input, or active animation step.
- Event loop: `ControlFlow::Wait` when idle+clean; `ControlFlow::WaitUntil(deadline)`
  when an animation/timer is scheduled. Request redraw only when dirty.
- FPS logging (already in main.rs) to confirm idle = 0 redraws.

### 10.2 Full-screen step-aside (`fullscreen.rs`)
- Use `windows-sys` to enumerate top-level windows and detect when a
  **fullscreen** app is running (a window covering the screen with no visible
  border, or a known fullscreen class).
- When detected: animate the notch/dock **aside** (slide off / shrink to a
  pill) and stop drawing the main content. When it clears: animate back.
- Pragmatic approach: a **low-frequency check** (e.g., every 2–5s via a
  `WaitUntil` timer) is acceptable here — this is the one place a light poll is
  reasonable, because Win32 has no event for "another app went fullscreen".
  Keep it cheap and off the hot path.

### 10.3 Files stay where they are (`state/shelf.rs`)
- Maintain a "shelf" of file references the user has interacted with. Storing
  paths (and resolving them on startup); never lose them on restart. This is
  state + persistence, not rendering.

### 10.4 Input (`input.rs`)
- Mouse move → hover state + hit-testing for dock/popover/settings.
- Click → activate (open popover, toggle settings, switch modes).
- Keyboard → Esc closes popover/settings, shortcuts for modes.
- Cursor: hide the cursor over the notch strip when appropriate (optional polish).

### 10.5 Multi-display & scaling
- Use the primary display's top strip. On multi-monitor, anchor to the primary.
- Re-run `fit_scale` + `set_view` on resize/monitor change.

**Acceptance:** App runs at 100%, 125%, 200% scaling with crisp text/icons;
idle consumes no redraw; fullscreen apps trigger step-aside and recovery;
settings persist across restarts.

---

## 11. Reference spec (facts to match exactly)

### 11.1 Top notch — confirmed facts
- Centered top pill, `--notch-scale: 1.6`.
- Shell transition easing: **`cubic-bezier(0.22,1,0.36,1)`**, **0.65s**.
- Body morph easing: **`cubic-bezier(0.32,1.18,0.42,1)`**, **0.52s**.
- Background **`#08080a`**; dock **`#050507`**.
- Sizes: 154×32/r13 (resting, waving, sleeping, vibing, surprised);
  332×68/r24 (notification, charging, headphones); 416×138/r34 (expanded);
  454×138/r34 (lyrics).
- Ears: 26px fillet → 14px.
- Aurora resting glow: **4.2s** cycle.
- **17 modes**, **43 animations**.

> **The exact 17 mode names and 43 animation names** were captured in the
> original `NOTCH_PLAN.md` (lost in a workspace reset). Before finalizing Phase 3,
> open the reference app (`src/App.tsx` + `src/index.css` in the backed-up notch
> repo) and enumerate the exact mode/animation/setting names. The structure and
> sizes above are confirmed; the names should be matched verbatim from the
> reference so the recreation is faithful.

### 11.2 Side dock — confirmed facts
- Body: 56px tall, `#050507`, radius `22px 0 0 22px`.
- Three 36px gauges: **Claude 73% / ChatGPT 21% / Gemini 52%**.
- Popover: **214px** wide.

### 11.3 Stage
- 15.6" stage = **1964×1124** design resolution.

### 11.4 Settings
- 5 sections: **Notch, Companion, Side Dock, Widgets, Appearance**.
- ~**18** settings total.

---

## 12. Execution order & acceptance summary

| Phase | Deliverable | Acceptance test |
|---|---|---|
| 2a | Render core (DONE) | Verification scene compiles + renders wallpaper, notch shell, primitives |
| 2b | Text pass | Text renders crisply at 125%/200% scale |
| 2c | Icon atlas + pass | 3–4 icons render, tinted, crisp |
| 3 | Top notch: 17 modes + companion + 43 anims | Smooth mode transitions at spec easing/timing; idle = no redraw |
| 4 | Side dock + gauges + popover | Gauges animate; popover opens/closes on hover/click |
| 5 | Settings UI (5 sections, ~18) | All settings render + interact + persist to JSON |
| 6 | Polish | Crisp at any scale; fullscreen step-aside; settings persist across restart |

**Build note:** Compile on Windows with `cargo build --release`. wgpu/winit API
drift between versions is the most likely build failure — treat those as small
adapter fixes, not rewrites. Pin the versions in §1.

**Definition of done:** The app runs on Windows 10/11, draws the notch + dock +
settings entirely via wgpu, draws only on change, is sharp at any scale, steps
aside for fullscreen apps, and persists settings — matching the reference app's
look and feel.
