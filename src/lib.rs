//! Native TIM data, simulation, editor, and frontend models.
//! See src/README.md for the source map and ordering constraints.

// Native resources, definitions, and integer motion.
pub mod definition;
pub mod font;
pub mod forces;
pub mod machine;
pub mod motion;
pub mod resources;
pub mod startup;

// Geometry, callbacks, and the ordered simulation update.
pub mod blast;
pub mod callback_catalogue;
pub mod callback_coverage;
pub mod contact;
#[path = "tables/jack_tables.rs"]
mod jack_tables;
pub mod outline;
#[path = "tables/part_extents.rs"]
mod part_extents;
pub mod simulation;
pub mod simulation_error;
pub mod spatial;

// Puzzle editing, progression, and saved solutions.
pub mod default_solutions;
pub mod editor;
pub mod goals;
pub mod importer;
pub mod progress;
pub mod puzzle;
pub mod solutions;
pub mod storage;

// Frontend models, drawing, and audio.
pub mod branding;
pub mod control_panel;
pub mod frontend;
pub mod game_screen;
pub mod menus;
pub mod sound;
pub mod viewport;

#[cfg(test)]
mod test_support;
