//! Named bits of the native +0x06 motion and dispatch word.
//! Runtime and save fields stay u16 so unknown bits round-trip unchanged.
#[repr(u16)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UpdateFlag {
    Supported = 1 << 0,
    SurfaceContact = 1 << 1,
    MovingContact = 1 << 2,
    CollisionResponseHandled = 1 << 3,
    /// Runtime-created records are unlinked and destroyed on reset, rather than restored.
    /// Native reset_object_records (1000:7e45), written by projectile and blast callbacks.
    CreatedDuringSimulation = 1 << 4,
    SurfaceDragBoost = 1 << 5,
    UseAlternateSize = 1 << 6,
    FlipVerticalAllowed = 1 << 9,
    FlipHorizontalAllowed = 1 << 10,
    InInventory = 1 << 11,
    ActiveMoving = 1 << 12,
    ActiveFixed = 1 << 13,
    FixedDefinition = 1 << 14,
    Locked = 1 << 15,
}

impl UpdateFlag {
    pub const fn mask(self) -> u16 {
        self as u16
    }

    /// Low response bits cleared by the caller before its next motion scan.
    pub const CONTACT_RESPONSE_MASK: u16 = Self::Supported.mask()
        | Self::SurfaceContact.mask()
        | Self::MovingContact.mask()
        | Self::CollisionResponseHandled.mask();
    pub const CONTACT_KIND_MASK: u16 = Self::SurfaceContact.mask() | Self::MovingContact.mask();
    pub const ACTIVE_LIST_MASK: u16 = Self::ActiveMoving.mask() | Self::ActiveFixed.mask();
}
