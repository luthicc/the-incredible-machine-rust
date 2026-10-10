#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
    Editing,
    Running,
    /// draw_level_complete_screen, waiting for a click.
    Complete,
    /// show_two_button_dialog: REPLAY or ADVANCE.
    AskReplay,
    /// Replaying the solution after REPLAY; the goal is not checked again.
    Replaying,
}
