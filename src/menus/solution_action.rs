#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SolutionAction {
    Manage,
    Up,
    Down,
    Name,
    Save,
    Update,
    Delete,
    Hint,
    Load,
    Solve,
    Done,
}
