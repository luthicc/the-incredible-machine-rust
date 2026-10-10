/// Runtime native collision record: object +0x84 and outline edge +0x8a.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ContactRecord {
    pub target: usize,
    pub outline_edge: u16,
    pub angle: u16,
    pub flag_a: bool,
    pub flag_b: bool,
}
