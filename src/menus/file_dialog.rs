use super::*;

/// run_file_dialog (1dff:4c36) over the *.TIM machines in the game directory.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FileDialog {
    pub mode: FileMode,
    pub directory: String,
    pub files: Vec<String>,
    pub scroll: usize,
    pub name: String,
    pub typing: bool,
}

impl FileDialog {
    pub fn open(mode: FileMode, directory: &std::path::Path) -> Self {
        let mut files: Vec<String> = crate::storage::read_dir(directory)
            .map(|entries| {
                entries
                    .flatten()
                    .filter_map(|e| e.file_name().into_string().ok())
                    .filter(|n| n.to_uppercase().ends_with(".TIM"))
                    .collect()
            })
            .unwrap_or_default();
        files.sort();
        Self {
            mode,
            directory: directory.display().to_string(),
            files,
            scroll: 0,
            name: String::new(),
            typing: mode == FileMode::Save,
        }
    }

    /// set_file_name_extension: names are kept to DOS 8.3 with ".TIM".
    pub fn file_name(&self) -> Option<String> {
        let stem: String = self
            .name
            .trim()
            .to_uppercase()
            .split('.')
            .next()
            .unwrap_or("")
            .chars()
            .filter(|c| c.is_ascii_alphanumeric() || *c == '_' || *c == '-')
            .take(8)
            .collect();
        (!stem.is_empty()).then(|| format!("{stem}.TIM"))
    }

    /// UI element list DS:4e75.
    pub fn hit(&self, cursor: (i16, i16)) -> Option<FileHit> {
        if inside((0x40, 0x7c, 0xb0, 0xf3), cursor) {
            let row = ((cursor.1 - 0x7c) / 10) as usize + self.scroll;
            return (row < self.files.len()).then_some(FileHit::Entry(row));
        }
        [
            ((0x90, 0x10c, 0x100, 0x11c), FileHit::Name),
            ((0xbc, 0x74, 0xdc, 0x94), FileHit::Up),
            ((0xbc, 0xe0, 0xdc, 0x100), FileHit::Down),
            ((0x40, 0x130, 0x90, 0x144), FileHit::Confirm),
            ((0xc0, 0x130, 0x110, 0x144), FileHit::Cancel),
        ]
        .into_iter()
        .find(|(clip, _)| inside(*clip, cursor))
        .map(|(_, hit)| hit)
    }

    pub fn scroll_by(&mut self, delta: isize) {
        let max = self.files.len().saturating_sub(12);
        self.scroll = (self.scroll as isize + delta).clamp(0, max as isize) as usize;
    }

    /// draw_file_dialog and its field/list drawers.
    pub fn render(
        &self,
        screen: &mut Screen,
        assets: &Assets,
        font: &Font,
        pressed: Option<FileHit>,
        frame: u32,
    ) {
        bordered_panel(screen, assets, (0x30, 0x31, 0x110, 0x149));
        beveled_box(screen, assets, (0x36, 0x129, 0x40, 0x20));
        beveled_box(screen, assets, (0xb6, 0x129, 0x50, 0x20));
        let (title, confirm) = match self.mode {
            FileMode::Load => ("LOAD MACHINE", "LOAD"),
            FileMode::Save => ("SAVE MACHINE", "SAVE"),
        };
        label_bar(screen, assets, font, title, 0x50, 0x34, 0xa0);
        button(
            screen,
            assets,
            font,
            confirm,
            0x40,
            0x130,
            pressed == Some(FileHit::Confirm),
        );
        button(
            screen,
            assets,
            font,
            "CANCEL",
            0xc0,
            0x130,
            pressed == Some(FileHit::Cancel),
        );
        beveled_box(screen, assets, (0xbc, 0x74, 0x20, 0x20));
        beveled_box(screen, assets, (0xbc, 0xe0, 0x20, 0x20));
        let cp = &assets.cp;
        blit(
            screen,
            &cp[37 + usize::from(pressed == Some(FileHit::Up))],
            0xc4,
            0x78,
            ALL,
        );
        blit(
            screen,
            &cp[39 + usize::from(pressed == Some(FileHit::Down))],
            0xc4,
            0xe8,
            ALL,
        );
        // Directory text, clipped from the left to fit (draw_file_dialog_directory_text).
        viewport_frame(screen, assets, (0x40, 0x56, 0xb8, 0x10), assets.shadow);
        let mut dir: &str = &self.directory;
        while font.width(dir) >= 0xad {
            let mut chars = dir.chars();
            chars.next();
            dir = chars.as_str();
        }
        font.draw(screen, dir, 0x44, 0x5a, assets.white);
        viewport_frame(screen, assets, (0x40, 0x78, 0x70, 0x80), assets.shadow);
        for (row, file) in self.files.iter().skip(self.scroll).take(12).enumerate() {
            let color = if file.eq_ignore_ascii_case(&self.file_name().unwrap_or_default()) {
                assets.yellow
            } else {
                assets.white
            };
            font.draw(screen, file, 0x44, 0x78 + 4 + 10 * row as i32, color);
        }
        label_bar(screen, assets, font, "File Name:", 0x30, 0x10c, 0x54);
        viewport_frame(screen, assets, (0x90, 0x10c, 0x70, 0x10), assets.shadow);
        let caret = if self.typing && frame & 8 != 0 {
            "_"
        } else {
            ""
        };
        font.draw(
            screen,
            &format!("{}{caret}", self.name.to_uppercase()),
            0x94,
            0x110,
            assets.white,
        );
    }
}
