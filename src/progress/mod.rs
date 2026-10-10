//! Player progress and puzzle passwords.
//!
//! TIM.CFG holds two words, written by write_game_progress_to_config
//! (1dff:4bfd): the highest puzzle reached (DS:4eb7) and the sound device
//! index the volume buttons cycle (DS:4ec1, 0..6). Options this port adds
//! live in TIM-BEVY.CFG, so TIM.CFG stays as the original writes it.
//!
//! Passwords: PASSWORD.TXT line n is the password for puzzle n + 1
//! (find_password_file_line_index, 1dff:4ae0). generate_level_password
//! (1000:2809) appends a score code: "-", the score + 0x100000 in hex without
//! its leading digit, then score * (sum of the first three password letters)
//! in base 34; '0' is written 'Z' and 'O' is written 'Y'.
mod graphics;
pub use graphics::RESOLUTIONS;
mod progress;
pub use progress::Progress;
mod passwords;
pub use passwords::Passwords;
mod qol_option;
pub use qol_option::QolOption;

use std::path::{Path, PathBuf};

/// Simulation speed choices for the control panel slider, as (multiplier
/// of the slider's 1X, label).
pub const SPEEDS: [(f32, &str); 10] = [
    (0.25, ".25X"),
    (0.5, ".5X"),
    (0.75, ".75X"),
    (1.0, "1X"),
    (1.25, "1.25X"),
    (1.5, "1.5X"),
    (1.75, "1.75X"),
    (2.0, "2X"),
    (2.5, "2.5X"),
    (3.0, "3X"),
];
pub const NORMAL_SPEED: u8 = 3;
/// The slider's 1X as a share of the PIT-derived native update rate: by eye
/// the original's machines run at about three quarters of it (its update
/// loop often overran the eight-tick minimum).
const ONE_X: f32 = 0.75;

fn config_path(root: &Path) -> PathBuf {
    root.join("TIM.CFG")
}

fn options_path(root: &Path) -> PathBuf {
    root.join("TIM-BEVY.CFG")
}

/// The last machine built for a puzzle, restored when it is opened again.
pub fn puzzle_save_path(root: &Path, level: u16) -> PathBuf {
    root.join("SAVES").join(format!("PUZZLE{level:02}.TIM"))
}

const DIGITS: &[u8] = b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ";

fn radix(mut value: u64, base: u64) -> String {
    let mut digits = Vec::new();
    loop {
        digits.push(DIGITS[(value % base) as usize]);
        value /= base;
        if value == 0 {
            break;
        }
    }
    digits.reverse();
    String::from_utf8(digits).unwrap()
}

/// compute_password_checksum (1000:2a34): letters count from 10 ('A').
fn parse_radix(text: &str, base: u64) -> Option<u64> {
    text.bytes().try_fold(0u64, |total, c| {
        let digit = match c {
            b'0'..=b'9' => c - b'0',
            b'A'..=b'Z' => c - b'A' + 10,
            _ => return None,
        };
        (u64::from(digit) < base).then(|| total * base + u64::from(digit))
    })
}

fn weight(word: &str) -> u64 {
    word.bytes().take(3).map(u64::from).sum()
}

#[cfg(test)]
mod tests;

pub const QOL_OPTIONS: [(QolOption, &str); 12] = [
    (QolOption::PixelScaling, "Pixel scaling"),
    (QolOption::StretchScreen, "Stretch to fill window"),
    (QolOption::SmoothMotion, "Smooth motion"),
    (QolOption::PuzzleHints, "Puzzle hints / solutions"),
    (QolOption::ConnectionHints, "Belt / rope connection hints"),
    (QolOption::PlugPreview, "Preview plugs while placing"),
    (QolOption::ZFlip, "Z to flip a part"),
    (QolOption::SlowScroll, "Slower picker buttons"),
    (QolOption::WheelScroll, "Mouse wheel in item picker"),
    (QolOption::DoubleClickCopy, "Double click to copy"),
    (QolOption::ShiftCopy, "Shift to keep placing"),
    (QolOption::UnlockAll, "Unlock all levels"),
];
