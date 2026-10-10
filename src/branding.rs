//! Shared player branding, drawn in the game's DOS palette.
use crate::resources::Screen;

pub const TITLE: &str = "The Incredible Machine - Bevy 1.0";
pub const EDITION: &str = "BEVY 1.0";

/// A compact gold gear, with a transparent background and dark outline.
/// Drawing at the requested size keeps the small menu icon crisp.
pub fn icon(size: u32) -> Screen {
    let mut screen = Screen {
        width: size,
        height: size,
        rgba: vec![0; (size * size * 4) as usize],
    };
    for y in 0..size {
        for x in 0..size {
            let dx = (x as f32 + 0.5) * 32.0 / size as f32 - 16.0;
            let dy = (y as f32 + 0.5) * 32.0 / size as f32 - 16.0;
            let radius = dx.hypot(dy);
            let outer = if (dy.atan2(dx) * 8.0).cos() > 0.0 {
                15.0
            } else {
                12.0
            };
            if radius > outer || radius < 5.0 {
                continue;
            }
            let color = if radius > outer - 1.2 || radius < 6.2 {
                [0, 0, 0, 255]
            } else if dy < -2.0 {
                [255, 227, 113, 255]
            } else {
                [243, 178, 0, 255]
            };
            let offset = ((y * size + x) * 4) as usize;
            screen.rgba[offset..offset + 4].copy_from_slice(&color);
        }
    }
    screen
}
