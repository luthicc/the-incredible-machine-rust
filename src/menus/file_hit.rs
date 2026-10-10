#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FileHit {
    Entry(usize),
    Name,
    Up,
    Down,
    Confirm,
    Cancel,
}
