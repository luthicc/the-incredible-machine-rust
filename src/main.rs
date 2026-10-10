#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]
#[path = "app/startup_image.rs"]
mod startup_image;
use startup_image::StartupImage;
#[path = "app/intro_frames.rs"]
mod intro_frames;
use intro_frames::IntroFrames;
#[path = "app/mode.rs"]
mod mode;
use mode::Mode;
#[path = "app/attract.rs"]
mod attract;
use attract::Attract;
#[path = "app/game.rs"]
mod game;
use game::Game;
#[path = "app/screen_sprite.rs"]
mod screen_sprite;
use screen_sprite::ScreenSprite;
#[path = "app/cursor_sprite.rs"]
mod cursor_sprite;
use cursor_sprite::CursorSprite;
#[path = "app/cursor_images.rs"]
mod cursor_images;
use cursor_images::CursorImages;
#[path = "app/branding.rs"]
mod branding;
use branding::BrandingPlugin;

use bevy::input::keyboard::KeyboardInput;
use bevy::input::mouse::{MouseScrollUnit, MouseWheel};
use bevy::{
    asset::RenderAssetUsages,
    image::ImageSampler,
    prelude::*,
    render::render_resource::{Extent3d, TextureDimension, TextureFormat},
};
use std::path::PathBuf;
#[cfg(not(target_arch = "wasm32"))]
mod launcher;
use tim_bevy::{
    frontend::{Dialog, Frontend, View},
    game_screen, machine, menus,
    puzzle::Phase,
    resources,
    simulation::MachineSimulation,
};

/// Title and credits run for 273 and 339 updates (1dff:04ce).
const TITLE_UPDATES: u32 = 273;
const CREDITS_UPDATES: u32 = 339;
/// Native update rate: initialize_timer(13) programs the PIT divisor to
/// 0xffff / 13, so it ticks at 1193182 / 5041 = 236.7 Hz, and run_game_loop /
/// run_machine_play_loop wait for at least 8 ticks per update: 29.59 updates
/// per second at most. TIM_UPDATES_PER_SECOND overrides it.
const NATIVE_UPDATES_PER_SECOND: f32 = 1_193_182.0 / (0xffff / 13) as f32 / 8.0;

fn updates_per_second() -> f32 {
    static RATE: std::sync::OnceLock<f32> = std::sync::OnceLock::new();
    *RATE.get_or_init(|| {
        std::env::var("TIM_UPDATES_PER_SECOND")
            .ok()
            .and_then(|v| v.parse::<f32>().ok())
            .filter(|v| *v > 0.0)
            .unwrap_or(NATIVE_UPDATES_PER_SECOND)
    })
}

#[cfg(not(target_arch = "wasm32"))]
fn main() {
    let mut args = std::env::args_os().skip(1);
    let first = args.next();
    let verify = first.as_deref() == Some(std::ffi::OsStr::new("--verify-data"));
    let input = if verify { args.next() } else { first };
    if verify && input.is_none() {
        eprintln!("Usage: tim-bevy --verify-data GAME.zip");
        std::process::exit(1);
    }
    let storage = tim_bevy::importer::storage_directory().unwrap_or_else(|e| {
        eprintln!("{e}");
        std::process::exit(1)
    });
    let result = if let Some(input) = input {
        let source = PathBuf::from(input);
        if source.is_dir() && resources::Resources::load(&source).is_ok() {
            Ok(Some(source))
        } else {
            tim_bevy::importer::import(&source, &storage).map(Some)
        }
    } else {
        Ok(tim_bevy::importer::selected_game(&storage).or_else(|| {
            std::env::current_dir()
                .ok()
                .filter(|p| tim_bevy::importer::validate(p).is_ok())
        }))
    };
    let root = match result {
        Ok(Some(root)) => root,
        Ok(None) => {
            launcher::run(storage, None);
            return;
        }
        Err(error) if !verify => {
            launcher::run(storage, Some(error));
            return;
        }
        Err(error) => {
            eprintln!("{error}");
            std::process::exit(1);
        }
    };
    if verify {
        match tim_bevy::importer::verify_audio(&root) {
            Ok(peaks) => {
                println!("Original game imported: {}", root.display());
                for (id, peak) in peaks {
                    println!("Sound {id}: runtime synthesis peak {peak:.3}");
                }
                println!("No WAV files or DOS executable required.");
            }
            Err(error) => {
                eprintln!("{error}");
                std::process::exit(1);
            }
        }
        return;
    }
    run_game(root);
}

#[cfg(target_arch = "wasm32")]
fn main() {}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen::prelude::wasm_bindgen]
pub fn start_game(bytes: &[u8]) -> Result<(), wasm_bindgen::JsValue> {
    let root = prepare_browser_zip(bytes)?;
    run_game(root);
    Ok(())
}

#[cfg(target_arch = "wasm32")]
fn prepare_browser_zip(bytes: &[u8]) -> Result<PathBuf, wasm_bindgen::JsValue> {
    console_error_panic_hook::set_once();
    let archives = tim_bevy::importer::archives_from_zip(bytes)
        .map_err(|e| wasm_bindgen::JsValue::from_str(&e))?;
    tim_bevy::storage::restore().map_err(|e| wasm_bindgen::JsValue::from_str(&e.to_string()))?;
    let mut hash = 0xcbf29ce484222325u64;
    for data in archives.values() {
        for b in data {
            hash = (hash ^ u64::from(*b)).wrapping_mul(0x100000001b3);
        }
    }
    let root = PathBuf::from(format!("/web/tim-{hash:016x}"));
    tim_bevy::storage::create_dir_all(&root)
        .map_err(|e| wasm_bindgen::JsValue::from_str(&e.to_string()))?;
    for (name, data) in archives {
        tim_bevy::storage::write(root.join(name), data)
            .map_err(|e| wasm_bindgen::JsValue::from_str(&e.to_string()))?;
    }
    tim_bevy::importer::validate(&root).map_err(|e| wasm_bindgen::JsValue::from_str(&e))?;
    Ok(root)
}

/// The page's back button: drop the held part, close a menu, or return to
/// the control panel, as Escape does.
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen::prelude::wasm_bindgen]
pub fn go_back() {
    touch::BACK_REQUESTED.store(true, std::sync::atomic::Ordering::Relaxed);
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen::prelude::wasm_bindgen]
pub fn audio_samples(count: usize) -> Vec<f32> {
    tim_bevy::sound::browser_samples(count.min(16384))
}

/// Validate the actual browser build without starting its graphical event loop.
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen::prelude::wasm_bindgen]
pub fn verify_browser_game(bytes: &[u8]) -> Result<String, wasm_bindgen::JsValue> {
    let check = || -> Result<String, String> {
        let root = prepare_browser_zip(bytes).map_err(|e| format!("{e:?}"))?;
        let resources = resources::Resources::load(&root)?;
        let mut front = Frontend::new(&resources, root.clone(), Some(4))?;
        front.open_solutions();
        let name = front.solutions.selected_name().unwrap_or("").to_owned();
        let (_, target) = machine::parse_collection(&front.solutions.selected_bytes()?, false)?;
        let count = front.session.editor.objects.len();
        let mut hints = 0;
        while tim_bevy::solutions::hint(&mut front.session.editor, &target)? {
            hints += 1;
            if hints > count {
                return Err("Hints failed to finish".into());
            }
            front.session.restore(&front.session.snapshot_bytes())?;
        }
        if front.session.editor.objects.len() != count || hints != 4 {
            return Err("Puzzle 4 hint regression".into());
        }
        let peaks = tim_bevy::importer::verify_audio(&root)?;
        Ok(format!(
            "Browser ZIP validated; selected {name}; Puzzle 4 completed in {hints} hints without duplication; {} audio probes audible",
            peaks.len()
        ))
    };
    check().map_err(|e| wasm_bindgen::JsValue::from_str(&e))
}

#[cfg(test)]
#[path = "app/presentation_tests.rs"]
mod presentation_tests;

#[path = "app/bootstrap.rs"]
mod bootstrap;
use bootstrap::*;

#[path = "app/presentation.rs"]
mod presentation;
use presentation::*;

#[path = "app/input.rs"]
mod input;
use input::*;

#[path = "app/touch.rs"]
mod touch;
use touch::*;

#[path = "app/magnify.rs"]
mod magnify;
use magnify::*;

#[path = "app/advance.rs"]
mod advance;
use advance::*;

#[path = "app/composition.rs"]
mod composition;
use composition::*;

#[path = "app/capture.rs"]
mod capture;
use capture::*;
