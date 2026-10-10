//! The portable player's import screen, shown before game data is available.
mod launcher;
use launcher::Launcher;
mod import_screen;
use import_screen::ImportScreen;

use bevy::{prelude::*, window::FileDragAndDrop};
use font8x8::{BASIC_FONTS, UnicodeFonts};
use std::{
    path::PathBuf,
    sync::{Mutex, mpsc},
};

pub fn run(storage: PathBuf, error: Option<String>) {
    App::new()
        .add_plugins(
            DefaultPlugins
                .set(ImagePlugin::default_nearest())
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        title: tim_bevy::branding::TITLE.into(),
                        resolution: (800, 480).into(),
                        resizable: false,
                        ..default()
                    }),
                    ..default()
                }),
        )
        .insert_resource(Launcher {
            storage,
            status: error.unwrap_or_else(|| "DROP YOUR ORIGINAL GAME ZIP HERE".into()),
            job: None,
        })
        .add_systems(Startup, setup)
        .add_plugins(crate::BrandingPlugin)
        .add_systems(Update, (import_files, draw).chain())
        .run();
}
fn setup(mut commands: Commands, mut images: ResMut<Assets<Image>>, state: Res<Launcher>) {
    commands.spawn(Camera2d);
    commands.spawn((
        Sprite::from_image(images.add(picture(&state.status))),
        ImportScreen,
        Transform::from_scale(Vec3::splat(2.0)),
    ));
}
fn import_files(
    mut drops: MessageReader<FileDragAndDrop>,
    mut state: ResMut<Launcher>,
    mut exit: MessageWriter<AppExit>,
    keys: Res<ButtonInput<KeyCode>>,
    mouse: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window>,
) {
    if keys.just_pressed(KeyCode::Escape) {
        exit.write(AppExit::Success);
    }
    let browse = mouse.just_released(MouseButton::Left)
        && windows
            .single()
            .ok()
            .and_then(|w| w.cursor_position())
            .is_some_and(|p| (48.0..352.0).contains(&p.x) && (290.0..342.0).contains(&p.y));
    if browse && state.job.is_none() {
        let storage = state.storage.clone();
        let (tx, rx) = mpsc::channel();
        state.job = Some(Mutex::new(rx));
        state.status = "CHOOSE YOUR ORIGINAL GAME ZIP".into();
        std::thread::spawn(move || {
            let result = rfd::FileDialog::new()
                .set_title("Open original Incredible Machine ZIP")
                .add_filter("Original game ZIP", &["zip"])
                .pick_file()
                .map(|path| tim_bevy::importer::import(&path, &storage))
                .transpose();
            let _ = tx.send(result);
        });
    }
    for drop in drops.read() {
        if let FileDragAndDrop::DroppedFile { path_buf, .. } = drop {
            if state.job.is_some() {
                continue;
            }
            let source = path_buf.clone();
            let storage = state.storage.clone();
            let (tx, rx) = mpsc::channel();
            state.job = Some(Mutex::new(rx));
            state.status = "IMPORTING ORIGINAL GAME...".into();
            std::thread::spawn(move || {
                let result = tim_bevy::importer::import(&source, &storage).map(Some);
                let _ = tx.send(result);
            });
        }
    }
    let ready = state
        .job
        .as_ref()
        .and_then(|job| job.lock().ok()?.try_recv().ok());
    if let Some(result) = ready {
        state.job = None;
        if matches!(result, Ok(None)) {
            state.status = "DROP YOUR ORIGINAL GAME ZIP HERE".into();
            return;
        }
        match result.and_then(|root| {
            let root = root.ok_or("No ZIP selected")?;
            let exe = tim_bevy::importer::launched_executable()?;
            std::process::Command::new(exe)
                .arg(root)
                .spawn()
                .map_err(|e| format!("Cannot start game: {e}"))?;
            Ok(())
        }) {
            Ok(()) => {
                exit.write(AppExit::Success);
            }
            Err(error) => {
                state.status = format!("{error}\n\nDROP A ZIP TO TRY AGAIN");
            }
        }
    }
}
fn draw(
    state: Res<Launcher>,
    sprites: Query<&Sprite, With<ImportScreen>>,
    mut images: ResMut<Assets<Image>>,
) {
    if !state.is_changed() {
        return;
    }
    for sprite in &sprites {
        if let Some(image) = images.get_mut(&sprite.image) {
            *image = picture(&state.status);
        }
    }
}
fn picture(status: &str) -> Image {
    let (w, h) = (400u32, 240u32);
    let mut rgba = vec![0; (w * h * 4) as usize];
    for pixel in rgba.chunks_exact_mut(4) {
        pixel.copy_from_slice(&[20, 33, 35, 255]);
    }
    for y in 145..171 {
        for x in 24..176 {
            let color = if x == 24 || x == 175 || y == 145 || y == 170 {
                [244, 209, 85, 255]
            } else {
                [40, 62, 62, 255]
            };
            rgba[(y * w as usize + x) * 4..(y * w as usize + x) * 4 + 4].copy_from_slice(&color);
        }
    }
    let mut text = |line: &str, x: usize, y: usize, color: [u8; 4]| {
        for (n, ch) in line.to_uppercase().chars().enumerate() {
            if let Some(glyph) = BASIC_FONTS.get(ch) {
                for (dy, bits) in glyph.iter().enumerate() {
                    for dx in 0..8 {
                        let (px, py) = (x + n * 8 + dx, y + dy);
                        if bits & (1 << dx) != 0 && px < w as usize && py < h as usize {
                            rgba[(py * w as usize + px) * 4..(py * w as usize + px) * 4 + 4]
                                .copy_from_slice(&color);
                        }
                    }
                }
            }
        }
    };
    text("THE INCREDIBLE MACHINE", 60, 28, [244, 209, 85, 255]);
    text(tim_bevy::branding::EDITION, 60, 48, [166, 201, 197, 255]);
    text("BROWSE ZIP...", 36, 154, [244, 209, 85, 255]);
    let mut lines = Vec::new();
    for paragraph in status.lines() {
        let mut line = String::new();
        for word in paragraph.split_whitespace() {
            if line.len() + word.len() + 1 > 44 {
                lines.push(line);
                line = String::new();
            }
            if !line.is_empty() {
                line.push(' ');
            }
            line.push_str(word);
        }
        lines.push(line);
    }
    for (i, line) in lines.iter().take(3).enumerate() {
        text(line, 24, 92 + i * 14, [238, 242, 231, 255]);
    }
    text(
        "ZIP OR EXTRACTED GAME FOLDER",
        24,
        190,
        [166, 201, 197, 255],
    );
    text(
        "MUSIC AND EFFECTS PLAY FROM GAME DATA",
        24,
        208,
        [166, 201, 197, 255],
    );
    let icon = tim_bevy::branding::icon(24);
    for y in 0..icon.height {
        for x in 0..icon.width {
            let src = ((y * icon.width + x) * 4) as usize;
            if icon.rgba[src + 3] != 0 {
                let dst = (((y + 24) * w + x + 24) * 4) as usize;
                rgba[dst..dst + 4].copy_from_slice(&icon.rgba[src..src + 4]);
            }
        }
    }
    super::screen_image(&rgba, w, h, 1)
}
