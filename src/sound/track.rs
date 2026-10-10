pub(super) struct Track {
    pub(super) start: usize,
    pub(super) end: usize,
    pub(super) position: usize,
    /// Tick of the next event.
    pub(super) next: u32,
    pub(super) status: u8,
    pub(super) done: bool,
}
