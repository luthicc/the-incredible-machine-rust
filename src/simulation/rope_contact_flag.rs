//! Temporary callback word produced by native 1000:6de9.
//! Direction matches use that routine's direction-mode comparisons; they
//! describe the route query rather than a part's persistent motion flags.
#[repr(u16)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum RopeContactFlag {
    LocalDirectionMatch = 1 << 0,
    RemoteDirectionMismatch = 1 << 1,
    RemoteDirectionMatch = 1 << 2,
    RouteLeftOfTarget = 1 << 3,
    RouteAtOrRightOfTarget = 1 << 4,
    ProbeOnly = 1 << 15,
}

impl RopeContactFlag {
    pub(super) const fn mask(self) -> u16 {
        self as u16
    }

    pub(super) const DIRECTION_MASK: u16 =
        Self::LocalDirectionMatch.mask() | Self::REMOTE_DIRECTION_MASK;
    pub(super) const REMOTE_DIRECTION_MASK: u16 =
        Self::RemoteDirectionMismatch.mask() | Self::RemoteDirectionMatch.mask();
    pub(super) const HORIZONTAL_SIDE_MASK: u16 =
        Self::RouteLeftOfTarget.mask() | Self::RouteAtOrRightOfTarget.mask();
    pub(super) const PROBE_REMOTE_MASK: u16 = Self::ProbeOnly.mask() | Self::REMOTE_DIRECTION_MASK;
    pub(super) const PROBE_SIDE_MASK: u16 = Self::ProbeOnly.mask() | Self::HORIZONTAL_SIDE_MASK;
}
