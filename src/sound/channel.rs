#[derive(Clone, Copy)]
pub(super) struct Channel {
    pub(super) program: u8,
    /// Controller 7 halved, as the driver keeps it.
    pub(super) volume: u8,
    pub(super) pitch: u16,
    pub(super) hold: bool,
}

impl Default for Channel {
    fn default() -> Self {
        Self {
            program: 0,
            volume: 63,
            pitch: 0x2000,
            hold: false,
        }
    }
}
