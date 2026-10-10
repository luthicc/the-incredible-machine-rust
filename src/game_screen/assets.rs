use super::*;

pub struct Assets {
    /// initialize_game_system (1dff:013f): DS:52cb = 3, DS:52c9 = 11.
    pub(crate) panel: [u8; 4],
    pub(crate) picker: [u8; 4],
    pub(crate) gray: [u8; 4],
    pub(crate) dark_gray: [u8; 4],
    pub(crate) gold: [u8; 4],
    pub(crate) border: Vec<Screen>,
    pub(crate) menu: Vec<Screen>,
    pub(crate) icons: Vec<Screen>,
    pub(crate) mouse: Vec<Screen>,
    pub(crate) score: Vec<Screen>,
    pub(crate) digits: Vec<Screen>,
    pub(crate) cp: Vec<Screen>,
    pub(crate) shadow: [u8; 4],
    pub(crate) yellow: [u8; 4],
    pub(crate) label: [u8; 4],
    pub(crate) white: [u8; 4],
    /// Palette 0xa and 0xc: the puzzle list's reached and locked entries.
    pub(crate) green: [u8; 4],
    pub(crate) red: [u8; 4],
}

impl Assets {
    pub fn load(resources: &Resources) -> Result<Self, String> {
        let load = |name| resources::sprites(resources, name, "TIM.PAL");
        let color = |index| resources::palette_color(resources, "TIM.PAL", index);
        Ok(Self {
            panel: color(3)?,
            picker: color(11)?,
            gray: color(7)?,
            dark_gray: color(8)?,
            gold: color(6)?,
            border: load("GP_BORD.BMP")?,
            menu: load("GP_MENU.BMP")?,
            icons: load("ICONS.BMP")?,
            mouse: load("MOUSE.BMP")?,
            score: load("SCORE1.BMP")?,
            digits: load("SCORE2.BMP")?,
            cp: load("CP.BMP")?,
            shadow: color(0)?,
            yellow: color(0xe)?,
            label: color(5)?,
            white: color(0xf)?,
            green: color(0xa)?,
            red: color(0xc)?,
        })
    }
}

impl Assets {
    /// A MOUSE.BMP frame: cursors and hover handles.
    pub fn mouse_frame(&self, frame: usize) -> Option<&Screen> {
        self.mouse.get(frame)
    }

    pub fn mouse_frames(&self) -> usize {
        self.mouse.len()
    }
}
