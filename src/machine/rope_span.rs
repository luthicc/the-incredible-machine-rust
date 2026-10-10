#[derive(Clone, Copy, Debug)]
pub(crate) struct RopeSpan {
    pub from: usize,
    pub from_slot: usize,
    pub to: usize,
    pub to_slot: usize,
    pub start: (i32, i32),
    pub end: (i32, i32),
}
