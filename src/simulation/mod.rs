//! Update-list selection from 1000:0f86. Callers re-evaluate each pass after
//! preceding callbacks, since callbacks may change flags and power state.
use crate::definition::PartDefinition as Def;
use crate::machine::{Object, ObjectList, PropertyFlag, RenderFlag, RenderFlags, UpdateFlag};
use crate::simulation_error::SimulationError;

mod rope_contact_flag;
use rope_contact_flag::RopeContactFlag;

mod states;
use states::*;

#[path = "../tables/trig_table.rs"]
mod contact_trig;
#[path = "../tables/part_friction.rs"]
mod part_friction;
#[path = "../tables/part_restitution.rs"]
mod part_restitution;
#[path = "../rope_cut.rs"]
mod rope_cut;
#[path = "../rope_dynamics/mod.rs"]
mod rope_dynamics;

mod collision_response;
use collision_response::*;
#[path = "parts/power_distributor.rs"]
mod power_distributor;
use power_distributor::*;
#[path = "parts/fish.rs"]
mod fish;
use fish::*;
mod rotation;
pub use rotation::*;
#[path = "parts/flashlight.rs"]
mod flashlight;
use flashlight::*;
#[path = "parts/scissors.rs"]
mod scissors;
use scissors::*;
mod dispatch;
pub use dispatch::*;
#[path = "parts/bellows.rs"]
mod bellows;
use bellows::*;
#[path = "parts/trampoline.rs"]
mod trampoline;
use trampoline::*;
#[path = "parts/mouse.rs"]
mod mouse;
use mouse::*;
#[path = "parts/cat.rs"]
mod cat;
use cat::*;
mod part_helpers;
use part_helpers::*;
#[path = "parts/jack_in_the_box.rs"]
mod jack_in_the_box;
use jack_in_the_box::*;
#[path = "parts/monkey.rs"]
mod monkey;
use monkey::*;
#[path = "parts/magnifying_glass.rs"]
mod magnifying_glass;
use magnifying_glass::*;
#[path = "parts/boxing_glove.rs"]
mod boxing_glove;
use boxing_glove::*;
#[path = "parts/balloon.rs"]
mod balloon;
use balloon::*;
#[path = "parts/dynamite.rs"]
mod dynamite;
use dynamite::*;
#[path = "parts/rocket.rs"]
mod rocket;
use rocket::*;
#[path = "parts/detonator.rs"]
mod detonator;
use detonator::*;
#[path = "parts/explosion.rs"]
mod explosion;
use explosion::*;
#[path = "parts/candle.rs"]
mod candle;
use candle::*;
#[path = "parts/solar_panel.rs"]
mod solar_panel;
use solar_panel::*;
#[path = "parts/windmill.rs"]
mod windmill;
use windmill::*;
#[path = "parts/fan.rs"]
mod fan;
use fan::*;
#[path = "parts/light.rs"]
mod light;
pub use light::*;
#[path = "parts/cannon.rs"]
mod cannon;
use cannon::*;
mod rope_contacts;
use rope_contacts::*;
#[path = "parts/seesaw.rs"]
mod seesaw;
use seesaw::*;
#[path = "parts/gun.rs"]
mod gun;
use gun::*;
mod impulse_priority;
use impulse_priority::*;
#[path = "parts/bucket.rs"]
mod bucket;
use bucket::*;
mod machine_simulation;
pub use machine_simulation::*;

#[cfg(test)]
mod interaction_tests;

/// Compatibility name for the existing title presentation.
pub type TitleSimulation = MachineSimulation;

#[path = "parts/bullet.rs"]
mod bullet;
use bullet::*;
