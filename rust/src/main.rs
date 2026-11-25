//! MacBook Dynamic Notch Recreation — native Rust rebuild (winit + wgpu).
//!
//! Phase 2a: GPU render core is live. This main draws a verification scene —
//! the wallpaper and a placeholder notch shell plus primitive tests (arc ring,
//! circle, stroke) — to validate the fill/stroke pipelines.

mod render;

use std::time::Instant;

use render::{shapes::rgba, DrawList, Renderer};
use winit::dpi::{LogicalSize, PhysicalSize};
use winit::event::{ControlFlow, Event, WindowEvent};
use winit::event_loop::EventLoopBuilder;
use winit::window::WindowBuilder;

/// Design resolution of the laptop screen (15.6" stage, same as the web app).
const STAGE_W: f32 = 1964.0;
const STAGE_H: f32 = 1124.0;
/// Minimum scale of the stage (same clamp as the web recreation).
const MIN_SCALE: f32 = 0.2;

/// Same fit math as the web app: scale the 1964x1124 stage to the viewport
/// (96% width, 190px reserved below), clamped to [0.2, 1].
fn fit_scale(size: PhysicalSize<u32>) -> f32 {
    let w = (size.width as f32 * 0.96) / STAGE_W;
    let h = (size.height as f32 - 190.0) / STAGE_H;
    w.min(h).min(1.0).max(MIN_SCALE)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    pollster::block_on(run())
}

async fn run() -> Result<(), Box<dyn std::error::Error>> {
    let mut event_loop = EventLoopBuilder::new().build()?;

    let window = WindowBuilder::new()
        .with_title("MacBook Dynamic Notch Recreation — Rust")
        .with_inner_size(LogicalSize::new(1440, 900))
        .build(&event_loop)?;

    let mut renderer = Renderer::new(&window).await?;

    let mut scale = fit_scale(window.inner_size());
    let mut frames = 0u32;
    let mut last_fps_print = Instant::now();

    // Prime the first frame.
    window.request_redraw();

    event_loop.run(move |event, _target, control_flow| {
        // Wait for events/redraws only — no idle loop (draws only when something moves).
        *control_flow = ControlFlow::Wait;

        match event {
            Event::WindowEvent { event, .. } => match event {
                WindowEvent::CloseRequested => *control_flow = ControlFlow::Exit,
                WindowEvent::Resized(_) => {
                    scale = fit_scale(window.inner_size());
                    renderer.resize(&window);
                    window.request_redraw();
                }
                WindowEvent::RedrawRequested => {
                    frames += 1;

                    // Stage rect: the 1964x1124 stage, scaled and centered.
                    let stage_w = STAGE_W * scale;
                    let stage_h = STAGE_H * scale;
                    let size = window.inner_size();
                    let origin_x = (size.width as f32 - stage_w) * 0.5;
                    let origin_y = (size.height as f32 - stage_h) * 0.5;
                    renderer.set_view(origin_x, origin_y, scale);

                    let mut list = DrawList::new();
                    draw_scene(&mut list);
                    renderer.render(&list);

                    if last_fps_print.elapsed().as_secs_f64() >= 1.0 {
                        let secs = last_fps_print.elapsed().as_secs_f64();
                        println!("fps: {:.1} (stage scale: {:.2})", frames as f64 / secs, scale);
                        frames = 0;
                        last_fps_print = Instant::now();
                    }
                }
                _ => {}
            },
            _ => {}
        }
    });

    Ok(())
}

/// Verification scene for the render core: wallpaper + placeholder notch +
/// primitive tests (arc ring, circle, stroke, triangle).
fn draw_scene(list: &mut DrawList) {
    // --- Wallpaper (same look as the web version) ---
    // Screen gradient: #e6edfc -> #c6d5f8 -> #7a92e8 -> #2c3a94 -> #0a0c1e
    list.fill_rect_lin(
        0.0,
        0.0,
        STAGE_W,
        STAGE_H,
        rgba(230, 237, 252, 1.0),
        rgba(10, 12, 30, 1.0),
        0.0,
        1.0,
    );

    // Petals (soft radial gradients, same colors as the web wallpaper-art).
    let petal_center = rgba(132, 108, 240, 0.82);
    let petal_inner = rgba(145, 114, 244, 0.80);
    let petal_outer = rgba(138, 104, 232, 0.78);
    // Center petal (approx stage coords; web art is scaled 2.35).
    list.fill_circle_grad(982.0, 42.0, 280.0, petal_center, rgba(25, 36, 112, 0.2));
    list.fill_circle_grad(857.0, 75.0, 260.0, petal_inner, rgba(92, 98, 242, 0.65));
    list.fill_circle_grad(1107.0, 75.0, 260.0, petal_inner, rgba(92, 98, 242, 0.65));
    list.fill_circle_grad(737.0, 247.0, 250.0, petal_outer, rgba(78, 76, 198, 0.52));
    list.fill_circle_grad(1227.0, 247.0, 250.0, petal_outer, rgba(78, 76, 198, 0.52));

    // Horizon band.
    list.fill_rect_lin(
        0.0,
        552.0,
        STAGE_W,
        540.0,
        rgba(122, 146, 235, 0.10),
        rgba(5, 5, 7, 1.0),
        0.0,
        1.0,
    );

    // --- Placeholder notch shell (top center, resting size) ---
    let notch_x = STAGE_W * 0.5 - 77.0;
    let notch_y = 0.0;
    list.shadow_rounded_rect(notch_x, notch_y, 154.0, 32.0, 13.0, 0.9);
    list.fill_rounded_rect(notch_x, notch_y, 154.0, 32.0, 13.0, rgba(8, 8, 10, 1.0));

    // --- Primitive tests ---
    // Arc ring (usage-gauge style): red arc, top-starting, 250° sweep.
    list.stroke_arc(1964.0 - 100.0, 300.0, 18.0, -90.0, 250.0, 3.2, rgba(232, 77, 28, 1.0), 64);
    // Solid circle + stroke line.
    list.fill_circle(200.0, 200.0, 30.0, rgba(48, 209, 88, 1.0));
    list.stroke_line(100.0, 100.0, 400.0, 100.0, 4.0, rgba(255, 255, 255, 1.0));
    // Quadratic bezier stroke (eye-arch style).
    list.stroke_quad_bezier(
        (300.0, 400.0),
        (350.0, 350.0),
        (400.0, 400.0),
        3.0,
        rgba(255, 255, 255, 1.0),
        16,
    );
    // Triangle fill (bolt-style).
    list.fill_triangle(
        (500.0, 380.0),
        (520.0, 440.0),
        (560.0, 360.0),
        rgba(255, 255, 255, 1.0),
    );
}
