use super::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Resize {
    pub index: usize,
    pub handle: Handle,
}
