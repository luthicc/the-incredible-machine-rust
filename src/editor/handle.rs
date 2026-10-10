/// Hover handles from compute_object_hover_region (1000:4830). The click
/// actions come from the jump table at 1dff:1cfe.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Handle {
    /// 1: bottom-left square, flip (definition +0x2c callback, mode 1).
    FlipHorizontal,
    /// 2: bottom-right square, flip mode 2.
    FlipVertical,
    /// 3..6: side squares, resize (1dff:2826).
    ResizeLeft,
    ResizeRight,
    ResizeTop,
    ResizeBottom,
    /// 7: the part body, carry it.
    Body,
    /// 8: top-left square, return the part to the bin (1dff:2743).
    Remove,
}
