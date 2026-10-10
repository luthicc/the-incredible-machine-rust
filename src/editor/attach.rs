/// Which connection an attach point offers.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Attach {
    Belt,
    /// A rope on this anchor slot.
    Rope(u8),
}
