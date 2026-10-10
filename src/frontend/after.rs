/// What a YES (or CONTINUE) does once a message closes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum After {
    Nothing,
    Quit,
    Restart,
    EnterFreeform,
    LeaveFreeform,
}
