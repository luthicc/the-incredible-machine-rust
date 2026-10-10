//! Launch control panel flow around a puzzle or freeform session: the panel
//! controls, their dialogs, progress in TIM.CFG, and moving between puzzles.
//! Input arrives as presses, releases, drags, and typed text so the flow can
//! be driven without a window.
mod after;
pub use after::After;
mod dialog;
pub use dialog::Dialog;
mod view;
pub use view::View;
mod frontend;
pub use frontend::Frontend;

use crate::menus::{Control, FileDialog, FileHit, FileMode, Message, SelectHit, SelectPuzzle};
use crate::progress::{Passwords, Progress};
use crate::puzzle::{Phase, PuzzleSession};
use crate::resources::Resources;
use std::path::PathBuf;

/// A puzzle as the level defines it, or as last saved when `restore`.
fn open_puzzle(
    resources: &Resources,
    root: &std::path::Path,
    level: u16,
    score: i32,
    restore: bool,
) -> Result<PuzzleSession, String> {
    let mut session = PuzzleSession::load(resources, level, score)?;
    if restore {
        if let Ok(bytes) = crate::storage::read(crate::progress::puzzle_save_path(root, level)) {
            // A damaged save falls back to the fresh puzzle.
            let fresh = session.editor.clone();
            if session.restore(&bytes).is_err() {
                session.editor = fresh;
            }
        }
    }
    Ok(session)
}

#[cfg(test)]
mod tests;
