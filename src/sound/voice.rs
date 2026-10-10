#[derive(Clone, Copy, Default)]
pub(super) struct Voice {
    pub(super) owner: Option<(u64, u8)>,
    pub(super) note: u8,
    pub(super) velocity: u8,
    pub(super) patch: Option<usize>,
    pub(super) on: bool,
    pub(super) sustained: bool,
    pub(super) age: u64,
}
