use super::*;

/// The pointer, drawn over the screen at window resolution so it moves
/// smoothly instead of stepping by whole source pixels.
#[derive(Component)]
pub(super) struct CursorSprite;
