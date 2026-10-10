use super::*;
use crate::progress::{Progress, RESOLUTIONS};

#[derive(Default, Debug)]
pub struct GraphicsMenu {
    pub resolution_open: bool,
    pub pressed: Option<(i16, i16)>,
}
const RESOLUTION: Clip = (330, 98, 510, 118);

impl GraphicsMenu {
    /// Dropdowns consume every click, including clicks outside their list.
    /// Keep resolution choices open as the window changes size beneath the pointer.
    pub fn click(&mut self, progress: &mut Progress, c: (i16, i16)) -> bool {
        if self.resolution_open {
            if (330..510).contains(&c.0) && (118..278).contains(&c.1) {
                progress.resolution = ((c.1 - 118) / 20) as u8;
            } else {
                self.resolution_open = false;
            }
            return true;
        }
        if inside(RESOLUTION, c) {
            self.resolution_open = true;
            return true;
        }
        false
    }
    pub fn close_overlay(&mut self) -> bool {
        let open = self.resolution_open;
        self.resolution_open = false;
        self.pressed = None;
        open
    }
    pub fn overlay_open(&self) -> bool {
        self.resolution_open
    }
}
fn field(
    screen: &mut Screen,
    assets: &Assets,
    font: &Font,
    clip: Clip,
    label: &str,
    selected: bool,
) {
    fill(
        screen,
        clip,
        if selected { assets.gold } else { assets.gray },
    );
    font.draw(screen, label, clip.0 + 6, clip.1 + 6, assets.shadow);
}
/// Two-pixel frame using the same black and gold as the DOS controls.
fn dropdown_border(screen: &mut Screen, assets: &Assets, (x0, y0, x1, y1): Clip) {
    for (inset, color) in [(0, assets.shadow), (1, assets.gold)] {
        let (l, t, r, b) = (x0 + inset, y0 + inset, x1 - 1 - inset, y1 - 1 - inset);
        line(screen, (l, t), (r, t), color, ALL);
        line(screen, (l, b), (r, b), color, ALL);
        line(screen, (l, t), (l, b), color, ALL);
        line(screen, (r, t), (r, b), color, ALL);
    }
}

pub(super) fn draw_graphics(
    screen: &mut Screen,
    assets: &Assets,
    font: &Font,
    p: &Progress,
    menu: &GraphicsMenu,
) {
    font.draw(screen, "Resolution", 130, 104, assets.shadow);
    let (w, h) = RESOLUTIONS[usize::from(p.resolution)];
    field(
        screen,
        assets,
        font,
        RESOLUTION,
        &format!("{w} x {h}"),
        false,
    );
    dropdown_border(screen, assets, RESOLUTION);
    // Separate arrow button, anchored to the right edge instead of the text.
    let arrow_left = RESOLUTION.2 - 24;
    line(
        screen,
        (arrow_left, 100),
        (arrow_left, 115),
        assets.dark_gray,
        ALL,
    );
    line(
        screen,
        (arrow_left + 1, 100),
        (arrow_left + 1, 115),
        assets.white,
        ALL,
    );
    let center = RESOLUTION.2 - 12;
    for row in 0..5 {
        let half_width = 4 - row;
        line(
            screen,
            (center - half_width, 105 + row),
            (center + half_width, 105 + row),
            assets.shadow,
            ALL,
        );
    }
    if menu.resolution_open {
        fill(screen, (332, 120, 512, 280), assets.shadow);
        for (i, (w, h)) in RESOLUTIONS.iter().enumerate() {
            let y = 118 + i as i32 * 20;
            field(
                screen,
                assets,
                font,
                (330, y, 510, y + 20),
                &format!("{w} x {h}"),
                i == usize::from(p.resolution),
            );
        }
        dropdown_border(screen, assets, (330, 118, 510, 278));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn resolution_navigation_keeps_overlay_open_and_consumes_underlying_switches() {
        let mut p = Progress::default();
        let mut menu = GraphicsMenu::default();
        assert!(menu.click(&mut p, (400, 108)));
        for i in 0..RESOLUTIONS.len() {
            assert!(menu.click(&mut p, (400, 128 + i as i16 * 20)));
            assert_eq!(p.resolution, i as u8);
            assert!(menu.resolution_open);
        }
        assert!(menu.click(&mut p, (200, 132))); // Pixel scaling behind dropdown.
        assert!(!menu.resolution_open);
        assert!(p.pixel_scaling);
    }
}
