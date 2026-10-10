use super::*;

pub(super) struct Sample {
    pub(super) id: u16,
    pub(super) looping: bool,
    pub(super) rate: u32,
    pub(super) samples: Arc<[u8]>,
    /// 16.16 position.
    pub(super) position: u64,
}
