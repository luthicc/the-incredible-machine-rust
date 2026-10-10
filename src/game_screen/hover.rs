use super::*;

/// What the cursor is over, for the native cursor shapes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Hover {
    Nothing,
    Start,
    BinSlot(usize),
    ScrollUp,
    ScrollDown,
    /// The highlighted part and the handle under the cursor, if any.
    Part(usize, Option<Handle>),
    /// A free belt or rope anchor a click would attach to (port shortcut).
    Attach(AttachPoint),
}
