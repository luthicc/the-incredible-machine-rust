use super::*;

pub(super) struct Driver {
    pub(super) patches: Vec<Patch>,
    pub(super) rhythm_keys: [u8; RHYTHM_KEYS],
    pub(super) frequencies: [u16; 48],
    pub(super) velocity_curve: [u8; 64],
    pub(super) volume_curve: [u8; 64],
}
