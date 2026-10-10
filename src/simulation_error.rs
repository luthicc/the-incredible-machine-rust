use crate::definition::PartDefinition;
use std::fmt;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SimulationError {
    CallbackFailure {
        tick: u32,
        object: usize,
        definition: PartDefinition,
        reason: String,
    },
}

impl fmt::Display for SimulationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::CallbackFailure {
                tick,
                object,
                definition,
                reason,
            } => write!(f, "tick {tick}, object {object} ({definition:?}): {reason}"),
        }
    }
}
impl std::error::Error for SimulationError {}
