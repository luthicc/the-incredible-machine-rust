use super::*;

#[derive(Clone)]
pub(super) struct Sound {
    pub(super) music: bool,
    pub(super) looping: bool,
    pub(super) body: Body,
}
