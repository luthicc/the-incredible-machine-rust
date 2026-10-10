use super::*;

/// Title machine followed by the credits machine, each a live simulation.
pub(super) struct Attract {
    pub(super) title: MachineSimulation,
    pub(super) credits: MachineSimulation,
    pub(super) showing_credits: bool,
    pub(super) updates: u32,
}
