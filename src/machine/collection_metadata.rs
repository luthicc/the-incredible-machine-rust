/// Saved header shared by collection decoding, environment forces, and launch UI.
#[derive(Debug)]
pub struct CollectionMetadata {
    pub title: String,
    pub description: String,
    /// Starting Bonus 1 (editing) and Bonus 2 (running), DS:50af/50b1.
    pub bonuses: Option<(u16, u16)>,
    pub air: i16,
    pub gravity: i16,
    pub scroll_limits: Option<(i16, i16)>,
    pub music: u16,
    pub counts: [u16; 3],
    pub(super) records_offset: usize,
}
