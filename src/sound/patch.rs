use super::*;

#[derive(Clone, Copy, Default)]
pub(super) struct Patch {
    pub(super) modulator: Operator,
    pub(super) carrier: Operator,
}
