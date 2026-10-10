use super::RenderFlags;

/// One bit in the native object render and interaction word.
#[repr(u16)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RenderFlag {
    BeltAttachable = 1 << 0,
    BeltConnected = 1 << 1,
    RopeAttachable = 1 << 2,
    TwoRopeAnchors = 1 << 3,
    FlipHorizontal = 1 << 4,
    FlipVertical = 1 << 5,
    ProcessedThisUpdate = 1 << 6,
    ResizeHorizontal = 1 << 7,
    ResizeVertical = 1 << 8,
    CallbackBlocked = 1 << 9,
    RopeProbeBypassed = 1 << 10,
    PowerUpdateCallback = 1 << 11,
    CompositeSprite = 1 << 12,
    Hidden = 1 << 13,
    /// No active bit test or bit-specific writer in this TIM executable.
    /// Retained so saved render words round-trip without losing this reserved bit.
    ReservedBit14 = 1 << 14,
    ForceSurfaceCollision = 1 << 15,
}
impl RenderFlag {
    pub const fn mask(self) -> RenderFlags {
        RenderFlags(self as u16)
    }
}
