#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NearbyContact {
    pub index: usize,
    pub horizontal_gap: i16,
    pub vertical_gap: i16,
}
