#[derive(Clone, Copy, Default)]
pub(super) struct Operator {
    pub(super) ksl: u8,
    pub(super) mult: u8,
    pub(super) feedback: u8,
    pub(super) attack: u8,
    pub(super) sustain_level: u8,
    pub(super) sustaining: bool,
    pub(super) decay: u8,
    pub(super) release: u8,
    pub(super) total_level: u8,
    pub(super) am: bool,
    pub(super) vibrato: bool,
    pub(super) ksr: bool,
    /// Inverted connection byte: true is additive.
    pub(super) additive: bool,
    pub(super) waveform: u8,
}
