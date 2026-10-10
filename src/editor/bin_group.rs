use super::*;

/// One parts-bin row: consecutive bin parts of a definition.
#[derive(Clone, Debug, PartialEq)]
pub struct BinGroup {
    pub definition: Def,
    pub count: usize,
    pub first: usize,
}
