//! The wheel's shape, in logical pixels (96 DPI), and where the pointer is on it.

/// Slices in the ring.
pub const SLICES: usize = 8;
/// Radius of the ring.
pub const OUTER: f32 = 140.0;
/// Radius of the hub.
pub const HUB: f32 = 54.0;
/// Room around the ring for its shadow.
pub const MARGIN: f32 = 16.0;
/// Width and height of the wheel's window.
pub const SIZE: f32 = 2.0 * (OUTER + MARGIN);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Hit {
    Hub,
    Slice(usize),
    Outside,
}

/// What is under a point `dx`, `dy` logical pixels from the wheel's centre.
pub fn hit(dx: f32, dy: f32) -> Hit {
    let d = (dx * dx + dy * dy).sqrt();
    if d <= HUB {
        return Hit::Hub;
    }
    if d > OUTER {
        return Hit::Outside;
    }
    // Slice 0 is at the top; slices go clockwise.
    let deg = dy.atan2(dx).to_degrees();
    let step = 360.0 / SLICES as f32;
    let i = ((deg + 90.0 + step / 2.0).rem_euclid(360.0) / step) as usize;
    Hit::Slice(i.min(SLICES - 1))
}

/// The angle, in radians, through the middle of slice `i`.
pub fn slice_angle(i: usize) -> f32 {
    (-90.0 + i as f32 * 360.0 / SLICES as f32).to_radians()
}

/// The window's top-left corner so the wheel is centred on the cursor but
/// stays fully inside the work area (left, top, right, bottom), sliding
/// inward near a screen edge.
pub fn place(cursor: (i32, i32), size: i32, work: (i32, i32, i32, i32)) -> (i32, i32) {
    let (l, t, r, b) = work;
    let clamp = |v: i32, lo: i32, hi: i32| if hi < lo { lo } else { v.clamp(lo, hi) };
    (
        clamp(cursor.0 - size / 2, l, r - size),
        clamp(cursor.1 - size / 2, t, b - size),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hits() {
        assert_eq!(hit(0.0, 0.0), Hit::Hub);
        assert_eq!(hit(0.0, -100.0), Hit::Slice(0));
        assert_eq!(hit(70.0, -70.0), Hit::Slice(1));
        assert_eq!(hit(100.0, 0.0), Hit::Slice(2));
        assert_eq!(hit(0.0, 100.0), Hit::Slice(4));
        assert_eq!(hit(-100.0, 0.0), Hit::Slice(6));
        assert_eq!(hit(-70.0, -70.0), Hit::Slice(7));
        assert_eq!(hit(0.0, -141.0), Hit::Outside);
        // Just either side of the boundary between slices 0 and 1.
        let edge = (-90.0f32 + 22.5).to_radians();
        assert_eq!(
            hit(100.0 * (edge - 0.01).cos(), 100.0 * (edge - 0.01).sin()),
            Hit::Slice(0)
        );
        assert_eq!(
            hit(100.0 * (edge + 0.01).cos(), 100.0 * (edge + 0.01).sin()),
            Hit::Slice(1)
        );
    }

    #[test]
    fn slides_inward_at_edges() {
        let work = (0, 0, 1920, 1040);
        assert_eq!(place((960, 500), 312, work), (804, 344));
        assert_eq!(place((10, 10), 312, work), (0, 0));
        assert_eq!(place((1915, 1035), 312, work), (1608, 728));
        // A second monitor to the left of the main one.
        assert_eq!(place((-5, 500), 312, (-1920, 0, 0, 1080)), (-312, 344));
    }
}
