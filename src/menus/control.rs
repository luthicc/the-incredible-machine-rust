/// Launch panel controls (UI element list DS:4e77).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Control {
    /// 0x8000: the PLAY switch and the machine preview.
    Play,
    /// 0x4000 / 0x2000: the volume buttons step the sound device 0..6.
    VolumeUp,
    VolumeDown,
    /// 0x1000.
    Quit,
    /// 0x800.
    Restart,
    /// 0x400: enter freeform mode.
    Freeform,
    /// 0x200.
    SelectPuzzle,
    /// 0x100 / 0x80: freeform only.
    Load,
    Save,
    /// 0x40 / 0x20: the two environment meters.
    TopMeter,
    BottomMeter,
    /// Port options in the black band below the status boards: drawing
    /// parts between updates, the music and effects switches, and the
    /// simulation speed slider.
    Qol,
    Solutions,
    Music,
    Effects,
    Speed,
}
