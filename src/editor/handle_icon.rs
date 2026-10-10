use super::*;

/// A handle icon from MOUSE.BMP and where draw_hover_highlight draws it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HandleIcon {
    pub handle: Handle,
    pub frame: usize,
    pub at: (i16, i16),
}

impl HandleIcon {
    /// The icon's painted pixels (MOUSE.BMP frames 27..31).
    pub fn opaque(&self) -> Rect {
        let (x0, y0, x1, y1) = match self.frame {
            27 => (1, 0, 11, 12),
            28 => (0, 2, 12, 9),
            29 => (2, 0, 9, 12),
            _ => (0, 0, 12, 12),
        };
        (
            self.at.0 + x0,
            self.at.1 + y0,
            self.at.0 + x1,
            self.at.1 + y1,
        )
    }

    /// The clickable area: the painted pixels plus a little padding.
    pub fn hit(&self) -> Rect {
        grow(self.opaque(), HANDLE_PAD)
    }
}
