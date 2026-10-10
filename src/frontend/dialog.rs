use super::*;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Dialog {
    Message(Message, After),
    Select(SelectPuzzle),
    File(FileDialog),
}
