use super::*;

pub struct PanelView<'a> {
    pub session: &'a PuzzleSession,
    pub description: &'a str,
    pub volume: u16,
    pub smooth_motion: bool,
    pub puzzle_hints: bool,
    pub music: bool,
    pub effects: bool,
    pub speed: u8,
    pub pressed: Option<Control>,
    pub solution_name: Option<&'a str>,
}
