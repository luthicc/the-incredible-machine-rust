//! One puzzle session: editing between runs, running the machine, the goal
//! check, bonus counters, and the level-complete sequence.
//!
//! Mirrors the game_mode switch between run_machine_play_loop (1dff:18d2,
//! mode 0x1000, editing) and run_game_loop (1000:12ab, mode 0x2000, running),
//! with handle_level_complete (1000:2710) once a goal sets mode 0x200.
mod phase;
pub use phase::Phase;
mod run;
pub use run::Run;
mod puzzle_session;
pub use puzzle_session::PuzzleSession;

use crate::editor::Editor;
use crate::goals::GoalState;
use crate::machine::UpdateFlag;
use crate::simulation::MachineSimulation;

/// One bin template per part the freeform bin offers (definitions 0..0x33
/// except bullet, explosion, rope end, and the holiday parts), taken from the
/// parts the puzzles themselves supply so their flags and anchors are the
/// game's own.
fn freeform_templates(
    resources: &crate::resources::Resources,
) -> Result<Vec<crate::machine::Object>, String> {
    use crate::definition::PartDefinition as Def;
    use crate::machine::ObjectList;
    let mut found: Vec<Option<crate::machine::Object>> = vec![None; 0x33];
    for level in 1..=crate::control_panel::level_count(resources) {
        let objects = crate::machine::collection_objects(
            resources,
            &crate::control_panel::level_name(level),
        )?;
        // Bin parts first: they carry the unlocked flags the player gets.
        let mut ordered: Vec<_> = objects
            .iter()
            .filter(|o| o.list == ObjectList::Inventory)
            .collect();
        ordered.extend(objects.iter().filter(|o| o.list != ObjectList::Inventory));
        for o in ordered {
            let id = o.definition.id() as usize;
            if id < found.len() && found[id].is_none() {
                found[id] = Some(o.clone());
            }
        }
    }
    let excluded = [
        Def::Bullet,
        Def::Explosion,
        Def::RopeEnd,
        Def::JackOLantern,
        Def::HeartBalloon,
        Def::ChristmasTree,
    ];
    Ok(found
        .into_iter()
        .flatten()
        .filter(|o| !excluded.contains(&o.definition))
        .map(|mut o| {
            o.list = ObjectList::Inventory;
            o.update_flags = (o.update_flags
                & !(UpdateFlag::ACTIVE_LIST_MASK | UpdateFlag::Locked.mask()))
                | UpdateFlag::InInventory.mask();
            o.state = o.saved_state;
            o.power_state = 0;
            o.render_flags
                .remove(crate::machine::RenderFlag::BeltConnected);
            o.contact = None;
            o.belt = o.belt.map(|_| [None, None]);
            for rope in o.ropes.iter_mut().flatten() {
                rope.endpoints = [None, None];
                rope.attachment_slots = [0, 0];
                rope.rest_length = 0.0;
            }
            o.linked_objects = [None; 6];
            o.pulley_connection = None;
            o
        })
        .collect())
}

#[cfg(test)]
mod tests;
