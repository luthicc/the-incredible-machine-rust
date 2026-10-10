use super::*;
use crate::sound::SoundEffect;

pub struct PuzzleSession {
    pub level: u16,
    pub name: String,
    pub title: String,
    pub editor: Editor,
    pub(super) forces: [crate::forces::PartForce; 58],
    pub run: Option<Run>,
    pub phase: Phase,
    /// DS:50af, counts down while editing; DS:50b1, counts down while running.
    /// Initial values are the two LEV header words after the description.
    pub edit_bonus: i16,
    pub run_bonus: i16,
    pub(super) edit_roll: i16,
    pub(super) run_roll: i16,
    /// DS:4ead/4eaf, the running score.
    pub score: i32,
    pub error: Option<String>,
    /// Freeform mode: no goal, no bonuses, any part, adjustable environment.
    pub freeform: bool,
    /// "New Password" for the completed panel (generate_level_password).
    pub new_password: Option<String>,
    /// The two saved environment words (top and bottom launch-panel meters)
    /// and the music word, kept for the panel and for saving.
    pub air: i16,
    pub gravity: i16,
    pub music: u16,
}

impl PuzzleSession {
    pub fn load(
        resources: &crate::resources::Resources,
        level: u16,
        score: i32,
    ) -> Result<Self, String> {
        let name = crate::control_panel::level_name(level);
        let header = crate::machine::collection_metadata(resources, &name)?;
        let objects = crate::machine::collection_objects(resources, &name)?;
        let (edit_bonus, run_bonus) = header.bonuses.unwrap_or((0, 0));
        Ok(Self {
            level,
            name,
            title: header.title,
            editor: Editor::new(objects),
            forces: crate::forces::initialize(header.air, header.gravity),
            run: None,
            phase: Phase::Editing,
            edit_bonus: edit_bonus as i16,
            run_bonus: run_bonus as i16,
            edit_roll: 0,
            run_roll: 0,
            score,
            error: None,
            freeform: false,
            new_password: None,
            air: header.air,
            gravity: header.gravity,
            music: header.music,
        })
    }

    /// Freeform mode (initialize_default_object_collection, 1dff:606b): an
    /// empty machine, or a loaded TIM file, with every part in the bin.
    pub fn freeform(
        resources: &crate::resources::Resources,
        saved: Option<&[u8]>,
    ) -> Result<Self, String> {
        let (air, gravity, music, objects) = match saved {
            Some(bytes) => {
                let (header, objects) = crate::machine::parse_collection(bytes, false)?;
                let placed = objects
                    .into_iter()
                    .filter(|o| o.list != crate::machine::ObjectList::Inventory)
                    .collect();
                (header.air, header.gravity, header.music, placed)
            }
            None => (0x43, 0x110, 0x3e9, Vec::new()),
        };
        Ok(Self {
            level: 0,
            name: "FREEFORM.TIM".into(),
            title: "FREEFORM MODE".into(),
            editor: Editor::freeform(objects, freeform_templates(resources)?),
            forces: crate::forces::initialize(air, gravity),
            run: None,
            phase: Phase::Editing,
            edit_bonus: 0,
            run_bonus: 0,
            edit_roll: 0,
            run_roll: 0,
            score: 0,
            error: None,
            freeform: true,
            new_password: None,
            air,
            gravity,
            music,
        })
    }

    /// The launch panel meters (1dff:333a, 1dff:33d3) in freeform mode.
    pub fn set_environment(&mut self, air: i16, gravity: i16) {
        self.air = air;
        self.gravity = gravity;
        self.forces = crate::forces::initialize(air, gravity);
    }

    /// save_level_without_shared_objects: the placed machine as a TIM file.
    pub fn save_bytes(&self) -> Vec<u8> {
        let mut objects = self.editor.objects.clone();
        if let Some(held) = &self.editor.held {
            objects[held.index].list = crate::machine::ObjectList::Inventory;
        }
        crate::machine::write_collection(&objects, self.air, self.gravity, self.music)
    }

    /// The whole puzzle as it stands in the editor, bin included, for the
    /// per-puzzle save the port keeps (not an original feature).
    pub fn snapshot_bytes(&self) -> Vec<u8> {
        let mut objects = self.editor.objects.clone();
        if let Some(held) = &self.editor.held {
            objects[held.index].list = crate::machine::ObjectList::Inventory;
        }
        crate::machine::write_collection_with_bin(&objects, self.air, self.gravity, self.music)
    }

    /// Put back a machine saved by `snapshot_bytes`.
    pub fn restore(&mut self, bytes: &[u8]) -> Result<(), String> {
        let (_, objects) = crate::machine::parse_collection(bytes, false)?;
        if objects.is_empty() {
            return Err("Empty puzzle save".into());
        }
        self.editor = Editor::new(objects);
        Ok(())
    }

    pub fn running(&self) -> bool {
        matches!(self.phase, Phase::Running | Phase::Replaying)
    }

    /// The top-right control (UI element 0x240,0..0x278,0x3f) switches to
    /// mode 0x2000 unless a part is carried.
    pub fn start(&mut self) {
        if self.phase != Phase::Editing || self.editor.held.is_some() {
            return;
        }
        self.launch(Phase::Running);
    }

    pub(super) fn launch(&mut self, phase: Phase) {
        match self.editor.machine() {
            Ok(objects) => {
                self.run = Some(Run {
                    simulation: MachineSimulation::from_objects(objects, self.forces),
                    goals: GoalState::default(),
                });
                self.phase = phase;
                self.error = None;
            }
            Err(error) => self.error = Some(error),
        }
    }

    /// A click while running returns to editing; parts go back to their
    /// placed positions (reset_object_records).
    pub fn stop(&mut self) {
        if self.running() {
            self.run = None;
            self.phase = Phase::Editing;
            crate::sound::stop_machine();
        }
    }

    /// One 1000:0f86 update and the loop around it.
    pub fn tick(&mut self) {
        self.animate_bonuses();
        if !self.running() {
            return;
        }
        let level = self.level;
        let check = self.phase == Phase::Running;
        let Some(run) = self.run.as_mut() else { return };
        if let Err(error) = run.simulation.step() {
            self.error = Some(format!("{error:?}"));
            self.stop();
            return;
        }
        if check && !self.freeform && run.goals.update(level, &run.simulation.objects) {
            self.complete();
        }
    }

    /// animate_status_counters (1000:2510): each bonus drops by one every 22
    /// roll steps; the edit bonus rolls faster while it is large.
    pub(super) fn animate_bonuses(&mut self) {
        let running = self.phase == Phase::Running;
        if (!running || self.edit_roll != 0)
            && self.edit_bonus != 0
            && self.phase != Phase::Complete
        {
            self.edit_roll += match self.edit_bonus {
                ..0x709 => 1,
                ..0xbb9 => 2,
                ..0xfa1 => 3,
                _ => 4,
            };
            if self.edit_roll > 0x15 {
                self.edit_roll = 0;
                self.edit_bonus -= 1;
            }
        }
        if (running || self.run_roll != 0) && self.run_bonus != 0 {
            self.run_roll += 1;
            if self.run_roll > 0x15 {
                self.run_roll = 0;
                self.run_bonus -= 1;
            }
        }
    }

    /// Roll offsets (DS:4eb3, DS:4eb1) for the odometer digits.
    pub fn rolls(&self) -> (i16, i16) {
        (self.edit_roll, self.run_roll)
    }

    /// handle_level_complete: bank both bonuses and show the panel.
    pub(super) fn complete(&mut self) {
        self.score += i32::from(self.edit_bonus) + i32::from(self.run_bonus);
        self.phase = Phase::Complete;
        // No more updates run to stop looping sounds (fans, motors...), so
        // silence the machine before the fanfare, as stop() does.
        crate::sound::stop_machine();
        crate::sound::play_effect(SoundEffect::PuzzleComplete);
    }

    pub fn total_bonus(&self) -> i32 {
        i32::from(self.edit_bonus) + i32::from(self.run_bonus)
    }

    /// Click on the complete panel: ask REPLAY or ADVANCE.
    pub fn acknowledge(&mut self) {
        if self.phase == Phase::Complete {
            self.edit_bonus = 0;
            self.run_bonus = 0;
            self.phase = Phase::AskReplay;
        }
    }

    /// REPLAY reruns the solved machine without checking the goal.
    pub fn replay(&mut self) {
        if self.phase == Phase::AskReplay {
            self.launch(Phase::Replaying);
        }
    }
}
