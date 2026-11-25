//! Icon rendering: each icon is drawn procedurally with the SDF fill/stroke
//! primitives (no SVG rasterization). Icons are drawn within a 24x24 local
//! space, scaled to the requested size and tinted with the given color.

use super::DrawList;

/// Draw an icon by ID. `(x, y)` is the top-left of the size×size box.
pub fn draw_icon(list: &mut DrawList, id: u32, x: f32, y: f32, size: f32, color: [f32; 4]) {
    let s = size / 24.0;
    let ox = x;
    let oy = y;
    // Local (lx, ly) in 0..24 → absolute.
    let px = |lx: f32, ly: f32| (ox + lx * s, oy + ly * s);
    let rr = |lx: f32, ly: f32, w: f32, h: f32, r: f32| {
        let (ax, ay) = px(lx, ly);
        list.fill_rounded_rect(ax, ay, w * s, h * s, r * s, color);
    };
    let circ = |lx: f32, ly: f32, r: f32| {
        let (ax, ay) = px(lx, ly);
        list.fill_circle(ax, ay, r * s, color);
    };
    let line = |lx1: f32, ly1: f32, lx2: f32, ly2: f32, t: f32| {
        let (ax1, ay1) = px(lx1, ly1);
        let (ax2, ay2) = px(lx2, ly2);
        list.stroke_line(ax1, ay1, ax2, ay2, t * s, color);
    };
    let arc = |cx: f32, cy: f32, r: f32, a0: f32, a1: f32, t: f32| {
        let (ax, ay) = px(cx, cy);
        list.stroke_arc(ax, ay, r * s, a0, a1, t * s, color, 16);
    };
    let tri = |a: (f32, f32), b: (f32, f32), c: (f32, f32)| {
        let (ax, ay) = px(a.0, a.1);
        let (bx, by) = px(b.0, b.1);
        let (cx2, cy2) = px(c.0, c.1);
        list.fill_triangle((ax, ay), (bx, by), (cx2, cy2), color);
    };

    match id {
        crate::assets::ICON_BATTERY => {
            rr(2, 7, 17, 10, 2.5); // outline (filled hollow look via two rects)
            // Draw outline as stroke + inner fill.
            let (ox2, oy2) = px(2, 7);
            list.stroke_rounded_rect(ox2, oy2, 17 * s, 10 * s, 2.5 * s, 1.5 * s, color, 8);
            rr(4, 9, 9, 6, 1.0); // fill
            rr(20, 9, 2, 4, 1.0); // tip
        }
        crate::assets::ICON_BELL => {
            // Bell body: approximate with a stroked path of arcs + lines.
            arc(12, 6, 6, 180.0, 360.0, 1.8); // top dome
            line(6, 6, 6, 13, 1.8);
            line(18, 6, 18, 13, 1.8);
            line(5, 13, 19, 13, 1.8);
            arc(12, 17, 2.5, 0.0, 360.0, 1.8); // bottom dots
        }
        crate::assets::ICON_CHARGING => {
            tri((13, 2), (5, 14), (10, 14));
            tri((12, 14), (19, 14), (11, 22));
        }
        crate::assets::ICON_HEADPHONES => {
            arc(12, 7, 8, 180.0, 360.0, 1.8); // band
            rr(3, 7, 4, 7, 1.5); // left cup
            rr(17, 7, 4, 7, 1.5); // right cup
        }
        crate::assets::ICON_MUSIC => {
            circ(7, 17, 3); // note head
            line(9.5, 17, 9.5, 4, 1.8); // stem
            line(9.5, 4, 17, 7, 1.8); // flag
            circ(17, 7, 2.5);
        }
        crate::assets::ICON_ALERT => {
            tri((12, 3), (3, 21), (21, 21)); // triangle outline (stroke via fill tri then... use stroke)
            // Redraw as stroked triangle: use three lines.
            let (a, b, c) = ((12, 3), (3, 21), (21, 21));
            let (ax, ay) = px(a.0, a.1);
            let (bx, by) = px(b.0, b.1);
            let (cx2, cy2) = px(c.0, c.1);
            list.stroke_line(ax, ay, bx, by, 1.8 * s, color);
            list.stroke_line(bx, by, cx2, cy2, 1.8 * s, color);
            list.stroke_line(cx2, cy2, ax, ay, 1.8 * s, color);
            line(12, 9, 12, 14, 1.8); // exclamation
            circ(12, 17, 1.2);
        }
        crate::assets::ICON_MIC => {
            rr(9, 3, 6, 11, 3); // mic body
            arc(12, 16, 5, 0.0, 360.0, 1.8); // arc
            line(12, 21, 12, 23, 1.8); // stem
        }
        crate::assets::ICON_FOCUS => {
            arc(12, 12, 8, 0.0, 360.0, 1.8);
            arc(12, 12, 4, 0.0, 360.0, 1.8);
            circ(12, 12, 1.5);
        }
        crate::assets::ICON_LYRICS => {
            line(3, 6, 15, 6, 1.8);
            line(3, 10, 15, 10, 1.8);
            line(3, 14, 11, 14, 1.8);
            line(3, 18, 11, 18, 1.8);
            circ(18, 16, 3);
        }
        crate::assets::ICON_NOTIFICATION => {
            // Filled bell.
            tri((12, 3), (6, 16), (18, 16));
            rr(5, 16, 14, 3, 1.5);
            circ(12, 21, 2);
        }
        _ => {}
    }
}
