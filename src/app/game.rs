use super::*;

#[derive(Resource)]
pub(super) struct Game {
    pub(super) resources: resources::Resources,
    pub(super) font: tim_bevy::font::Font,
    pub(super) mode: Mode,
    pub(super) attract: Attract,
    /// The control panel, its dialogs, and the puzzle or freeform session.
    pub(super) front: Frontend,
    pub(super) screen_assets: game_screen::Assets,
    /// Pointer in 640x480 source pixels, when over the window.
    pub(super) cursor: Option<(i16, i16)>,
    /// Whole-number enlargement of the presented image (Viewport::prescale).
    pub(super) prescale: u32,
    /// Running machine's object positions before its latest update, drawn
    /// blended towards the current ones by `pending` (presentation only).
    pub(super) before: Option<Vec<(i16, i16)>>,
    /// Presentation frames, for marching highlights and the start icon.
    pub(super) frame: u32,
    /// Time when a held picker arrow should next repeat.
    pub(super) scroll_hold: f32,
    pub(super) wheel_pending: f32,
    /// The last playfield click: when, where, and the part it picked up,
    /// for double-click "take another".
    pub(super) last_click: Option<(
        f32,
        (i16, i16),
        Option<tim_bevy::definition::PartDefinition>,
    )>,
    /// Seconds since launch, used by the intro and capture triggers.
    pub(super) elapsed: f32,
    /// Fractional simulation updates owed to the current mode.
    pub(super) pending: f32,
    pub(super) redraw: bool,
    /// Sounds per introduction frame, and how many frames have been shown.
    pub(super) intro_cues: Vec<Vec<tim_bevy::sound::SoundEffect>>,
    pub(super) intro_shown: usize,
    /// The title machine's music (its header word).
    pub(super) attract_music: u16,
}

impl Game {
    /// Mirror the frontend's view in the presentation mode.
    pub(super) fn sync_mode(&mut self) {
        if matches!(self.mode, Mode::Panel | Mode::Puzzle) {
            self.mode = match self.front.view {
                View::Panel => Mode::Panel,
                View::Game => Mode::Puzzle,
            };
        }
        self.redraw = true;
    }

    pub(super) fn open_panel(&mut self) {
        if self.mode == Mode::Attract {
            tim_bevy::sound::stop_machine();
        }
        self.front.open_panel();
        self.mode = Mode::Panel;
        self.redraw = true;
    }

    pub(super) fn start_puzzle(&mut self) {
        if self.mode == Mode::Attract {
            tim_bevy::sound::stop_machine();
        }
        self.front.view = View::Game;
        self.mode = Mode::Puzzle;
        self.before = None;
        self.pending = 0.0;
        self.redraw = true;
    }

    pub(super) fn reset_attract(&mut self) {
        self.attract.title.reset();
        self.attract.credits.reset();
        self.attract.showing_credits = false;
        self.attract.updates = 0;
        self.before = None;
        self.pending = 0.0;
    }
}
