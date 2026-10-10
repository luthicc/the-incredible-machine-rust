pub(super) enum Command {
    Play(u16),
    Stop(u16),
    StopEffects,
    Volume(u16),
    /// Port option: the music and effects switches on the control panel.
    Channels {
        music: bool,
        effects: bool,
    },
}
