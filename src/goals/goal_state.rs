use super::*;

/// Per-run goal bookkeeping; initialize_level_state clears it at every start.
#[derive(Clone, Debug, Default)]
pub struct GoalState {
    /// `level_failed_flag` and the nine words after it (DS:4e9d..).
    pub flags: [u16; 10],
    /// `level_timer`, incremented after each goal check.
    pub level_timer: u16,
    /// DS:4e87, advance_global_frame_timer (1000:1451): counts updates since
    /// the game screen was drawn, wrapping 0x2a00 to 0x1c00.
    pub frame_timer: u16,
}

impl GoalState {
    /// One run_game_loop iteration after process_machine_simulation_update:
    /// advance the frame timer, check the goal, then advance level_timer.
    pub fn update(&mut self, level: u16, objects: &[Object]) -> bool {
        self.frame_timer = self.frame_timer.wrapping_add(1);
        if self.frame_timer == 0x2a00 {
            self.frame_timer = 0x1c00;
        }
        let solved = check(level, objects, self);
        self.level_timer = self.level_timer.wrapping_add(1);
        solved
    }
}
