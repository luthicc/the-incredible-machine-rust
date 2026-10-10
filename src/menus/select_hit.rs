#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SelectHit {
    Entry(u16),
    PageUp,
    PageDown,
    Password,
    Done,
}
