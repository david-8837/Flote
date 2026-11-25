//! Easing functions. The two cubic-bezier curves used throughout the app:
//! - shell/hover transitions: cubic-bezier(0.22, 1, 0.36, 1)
//! - body morph:              cubic-bezier(0.32, 1.18, 0.42, 1)

/// Evaluate a cubic-bezier easing with control points (x1, y1, x2, y2).
/// `t` is in [0, 1]. Implemented via Newton-Raphson + bisection fallback.
pub fn cubic_bezier(t: f32, x1: f32, y1: f32, x2: f32, y2: f32) -> f32 {
    if t <= 0.0 {
        return 0.0;
    }
    if t >= 1.0 {
        return 1.0;
    }

    // Solve for the parameter s where the x-component equals t.
    let cx = 3.0 * x1;
    let bx = 3.0 * (x2 - x1) - cx;
    let ax = 1.0 - cx - bx;
    let cy = 3.0 * y1;
    let by = 3.0 * (y2 - y1) - cy;
    let ay = 1.0 - cy - by;

    let sample = |s: f32| -> f32 { (ax * s + bx) * s * s };

    // Newton-Raphson.
    let mut s = t;
    for _ in 0..8 {
        let x_est = sample(s) - t;
        if x_est.abs() < 1e-6 {
            break;
        }
        let d = (3.0 * ax * s + 2.0 * bx) * s + cx;
        if d.abs() < 1e-6 {
            break;
        }
        s -= x_est / d;
    }
    // Bisection fallback.
    if !(0.0..=1.0).contains(&s) {
        let (mut lo, mut hi) = (0.0f32, 1.0f32);
        s = t;
        for _ in 0..16 {
            s = (lo + hi) * 0.5;
            if sample(s) < t {
                lo = s;
            } else {
                hi = s;
            }
        }
    }
    (ay * s + by) * s * s
}

pub fn ease_out(t: f32) -> f32 {
    1.0 - (1.0 - t).powi(3)
}

pub fn ease_in(t: f32) -> f32 {
    t * t * t
}

pub fn ease_in_out(t: f32) -> f32 {
    if t < 0.5 {
        4.0 * t * t * t
    } else {
        1.0 - (-2.0 * t + 2.0).powi(3) * 0.5
    }
}

/// Shell / hover easing: cubic-bezier(0.22, 1, 0.36, 1)
pub fn shell(t: f32) -> f32 {
    cubic_bezier(t, 0.22, 1.0, 0.36, 1.0)
}

/// Body morph easing: cubic-bezier(0.32, 1.18, 0.42, 1)
pub fn body_morph(t: f32) -> f32 {
    cubic_bezier(t, 0.32, 1.18, 0.42, 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bezier_endpoints() {
        assert_eq!(cubic_bezier(0.0, 0.22, 1.0, 0.36, 1.0), 0.0);
        assert_eq!(cubic_bezier(1.0, 0.22, 1.0, 0.36, 1.0), 1.0);
    }
}
