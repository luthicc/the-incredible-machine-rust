/// Optional conveniences exposed by the QoL menu.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum QolOption {
    PixelScaling,
    PuzzleHints,
    ConnectionHints,
    PlugPreview,
    ZFlip,
    SmoothMotion,
    SlowScroll,
    WheelScroll,
    StretchScreen,
    DoubleClickCopy,
    ShiftCopy,
    UnlockAll,
}
