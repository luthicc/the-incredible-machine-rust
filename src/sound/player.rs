use super::*;

pub(super) struct Player {
    pub(super) uid: u64,
    pub(super) id: u16,
    pub(super) looping: bool,
    pub(super) data: Arc<[u8]>,
    pub(super) tracks: Vec<Track>,
    pub(super) channels: [Channel; 16],
    pub(super) tick: u32,
    pub(super) loop_tick: Option<u32>,
}
