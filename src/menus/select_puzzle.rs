use super::*;

/// run_puzzle_password_screen (1dff:10c0).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SelectPuzzle {
    /// DS:542c: first listed puzzle, pages of 21 (compute_puzzle_list_page_offset).
    pub first: u16,
    /// DS:542a: the chosen puzzle.
    pub selected: u16,
    pub password: String,
    pub typing: bool,
}

impl SelectPuzzle {
    pub fn new(selected: u16) -> Self {
        let mut first = 1;
        while first + 0x14 < selected {
            first += 0x15;
        }
        Self {
            first,
            selected,
            password: String::new(),
            typing: false,
        }
    }

    /// UI element list DS:4e71.
    pub fn hit(&self, cursor: (i16, i16)) -> Option<SelectHit> {
        if inside((0x30, 0x4c, 0x1c0, 0x11d), cursor) {
            return Some(SelectHit::Entry(
                self.first + ((cursor.1 - 0x4c) / 10) as u16,
            ));
        }
        [
            ((0x1cc, 0x42, 0x1ec, 0x62), SelectHit::PageUp),
            ((0x1cc, 0x108, 0x1ec, 0x128), SelectHit::PageDown),
            ((0x90, 0x13c, 0x158, 0x14c), SelectHit::Password),
            ((0x1f0, 300, 0x218, 0x154), SelectHit::Done),
        ]
        .into_iter()
        .find(|(clip, _)| inside(*clip, cursor))
        .map(|(_, hit)| hit)
    }

    /// draw_password_entry_dialog and draw_puzzle_list_entries: the current
    /// puzzle in white, reached puzzles in green, the rest in red.
    #[allow(clippy::too_many_arguments)]
    pub fn render(
        &self,
        screen: &mut Screen,
        assets: &Assets,
        font: &Font,
        titles: &[String],
        reached: u16,
        completed: &std::collections::BTreeSet<u16>,
        pressed: Option<SelectHit>,
        frame: u32,
    ) {
        bordered_panel(screen, assets, (0x20, 0x20, 0x220, 0x158));
        label_bar(screen, assets, font, "SELECT PUZZLE", 0xa8, 0x27, 0xc0);
        label_bar(screen, assets, font, "PASSWORD", 0x20, 0x13c, 0x60);
        for clip in [
            (0x1cc, 0x42, 0x20, 0x20),
            (0x1cc, 0x108, 0x20, 0x20),
            (0x1f0, 300, 0x28, 0x28),
        ] {
            beveled_box(screen, assets, clip);
        }
        let cp = &assets.cp;
        blit(
            screen,
            &cp[37 + usize::from(pressed == Some(SelectHit::PageUp))],
            0x1d4,
            0x46,
            ALL,
        );
        blit(
            screen,
            &cp[39 + usize::from(pressed == Some(SelectHit::PageDown))],
            0x1d4,
            0x110,
            ALL,
        );
        blit(
            screen,
            &cp[16 + usize::from(pressed == Some(SelectHit::Done))],
            0x200,
            0x12e,
            ALL,
        );
        viewport_frame(screen, assets, (0x90, 0x13c, 0x130, 0x10), assets.shadow);
        let caret = if self.typing && frame & 8 != 0 {
            "_"
        } else {
            ""
        };
        font.draw(
            screen,
            &format!("{}{caret}", self.password),
            0x94,
            0x140,
            assets.white,
        );
        viewport_frame(screen, assets, (0x30, 0x48, 400, 0xd8), assets.shadow);
        for row in 0..0x15u16 {
            let level = self.first + row;
            let Some(title) = titles.get(usize::from(level).wrapping_sub(1)) else {
                break;
            };
            let done = completed.contains(&level);
            let color = if level == self.selected {
                assets.white
            } else if done {
                assets.yellow
            } else if level > reached {
                assets.red
            } else {
                assets.green
            };
            let y = 0x4c + 10 * i32::from(row);
            font.draw(screen, &format!("PUZZLE {level}: {title}"), 0x34, y, color);
            if done {
                // Solved: a tick at the right of the row (port addition).
                let tick = [(0x1ae, y + 4), (0x1b1, y + 7), (0x1b7, y + 1)];
                for dx in 0..2 {
                    let shift = |(x, y): (i32, i32)| (x + dx, y);
                    line(screen, shift(tick[0]), shift(tick[1]), assets.yellow, ALL);
                    line(screen, shift(tick[1]), shift(tick[2]), assets.yellow, ALL);
                }
            }
        }
    }
}
