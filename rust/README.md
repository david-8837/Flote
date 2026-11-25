# MacBook Dynamic Notch Recreation — Rust

A native Rust recreation of the MacBook Dynamic Notch app (17 top-notch modes,
side AI dock, full settings UI), built on **winit + wgpu** — GPU-rendered with
SDF anti-aliasing, no web engine, drawing only when something changes.

## Status

**Phase 2a (render core) is implemented.** The verification scene in
`src/main.rs` draws the wallpaper, a placeholder notch shell, and primitive
tests (arc ring, circle, line, bezier, triangle) to validate the two instanced
pipelines.

Phases still to come (per `RUST_PLAN.md`):
- Phase 2b — text rendering (glyphon + rustybuzz)
- Phase 2c — icon rendering (svg-render)
- Phase 3 — top notch modes, animations, timing
- Phase 4 — side dock, gauges, popover
- Phase 5 — settings UI
- Phase 6 — polish (shadows, easing, focus, fullscreen step-aside)

## Running

```sh
cargo run --release
```

Requires a GPU with wgpu support (Vulkan on Linux, Metal on macOS, DX12 on
Windows). Builds on Windows 10 and 11.

## Architecture

```
src/
├── main.rs            # event loop, fit-scale, verification scene
└── render/
    ├── mod.rs         # Renderer, DrawList, instance packing, clip stack
    ├── shaders.rs     # WGSL fill + stroke shaders
    ├── shapes.rs      # draw-op helpers (rects, circles, gradients, strokes)
    └── path.rs        # bezier + arc tessellation
```

- **Fill pipeline** — one fullscreen triangle instanced 9×vec4 (144 B):
  rounded rects / circles / triangles via SDF, solid/linear/radial gradients,
  stacked shadows.
- **Stroke pipeline** — one fullscreen triangle instanced 4×vec4 (64 B):
  segment SDF with thickness, round caps/joins via overlapping circles; beziers
  and arcs are tessellated on the CPU.
- **Clip stack** — up to 4 rounded-rect clip regions in a uniform buffer;
  batches are flushed when the clip depth changes.
- **Stage scaling** — the 1964×1124 design stage is scaled to the viewport
  (clamped to ≥ 0.2) so it stays sharp at any display scale.
