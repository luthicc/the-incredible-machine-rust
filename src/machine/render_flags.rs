use super::RenderFlag;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct RenderFlags(pub(super) u16);
impl RenderFlags {
    pub const EMPTY: Self = Self(0);
    pub const FACING_MASK: Self = Self(0x0030);
    pub const UPDATE_PRESERVE_MASK: Self = Self(0xf9bf);
    pub const fn bits(self) -> u16 {
        self.0
    }
    pub const fn contains(self, flag: RenderFlag) -> bool {
        self.0 & flag as u16 != 0
    }
    pub const fn intersects(self, other: Self) -> bool {
        self.0 & other.0 != 0
    }
    pub fn insert(&mut self, flag: RenderFlag) {
        self.0 |= flag as u16;
    }
    pub fn remove(&mut self, flag: RenderFlag) {
        self.0 &= !(flag as u16);
    }
    pub fn toggle(&mut self, flag: RenderFlag) {
        self.0 ^= flag as u16;
    }
    pub fn retain(&mut self, mask: Self) {
        self.0 &= mask.0;
    }
}
impl From<u16> for RenderFlags {
    fn from(bits: u16) -> Self {
        Self(bits)
    }
}
impl From<RenderFlags> for u16 {
    fn from(flags: RenderFlags) -> Self {
        flags.0
    }
}
impl std::ops::BitAnd<u16> for RenderFlags {
    type Output = u16;
    fn bitand(self, rhs: u16) -> u16 {
        self.0 & rhs
    }
}
impl std::ops::BitOr for RenderFlags {
    type Output = Self;
    fn bitor(self, rhs: Self) -> Self {
        Self(self.0 | rhs.0)
    }
}
impl std::ops::BitOr<u16> for RenderFlags {
    type Output = Self;
    fn bitor(self, rhs: u16) -> Self {
        Self(self.0 | rhs)
    }
}
impl std::ops::BitOrAssign<u16> for RenderFlags {
    fn bitor_assign(&mut self, rhs: u16) {
        self.0 |= rhs;
    }
}
impl std::ops::BitAndAssign<u16> for RenderFlags {
    fn bitand_assign(&mut self, rhs: u16) {
        self.0 &= rhs;
    }
}
impl std::ops::BitXorAssign<u16> for RenderFlags {
    fn bitxor_assign(&mut self, rhs: u16) {
        self.0 ^= rhs;
    }
}
impl PartialEq<u16> for RenderFlags {
    fn eq(&self, rhs: &u16) -> bool {
        self.0 == *rhs
    }
}
impl std::fmt::Display for RenderFlags {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}
