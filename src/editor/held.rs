use super::*;

#[derive(Clone, Debug, PartialEq)]
pub struct Held {
    pub index: usize,
    pub carry: Carry,
}
