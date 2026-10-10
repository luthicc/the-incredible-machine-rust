//! Named bits of the native object property word, distinct from +0x06.
#[repr(u16)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PropertyFlag {
    ElectricalPlug = 1 << 0,
    ElectricalOutlet = 1 << 1,
    Ignitable = 1 << 2,
    /// Native place_held_object_at_cursor (1dff:21ec) skips 16-pixel grid snapping.
    FreePlacement = 1 << 3,
    CarriedByBucket = 1 << 4,
}

impl PropertyFlag {
    pub const ELECTRICAL_MASK: u16 = Self::ElectricalPlug.mask() | Self::ElectricalOutlet.mask();
    pub const fn mask(self) -> u16 {
        self as u16
    }
}
