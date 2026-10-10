use super::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Facing {
    Left,
    Right,
}

impl Facing {
    pub fn from_render_flags(flags: RenderFlags) -> Self {
        if !flags.contains(RenderFlag::FlipHorizontal) {
            Self::Right
        } else {
            Self::Left
        }
    }
    pub fn sign(self) -> i32 {
        match self {
            Self::Left => -1,
            Self::Right => 1,
        }
    }
}
