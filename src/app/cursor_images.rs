use super::*;

/// MOUSE.BMP frames as images, enlarged by the current prescale.
#[derive(Resource, Default)]
pub(super) struct CursorImages {
    pub(super) prescale: u32,
    pub(super) frames: Vec<Handle<Image>>,
}
