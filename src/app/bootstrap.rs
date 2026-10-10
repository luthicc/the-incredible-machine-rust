use super::*;

pub(super) fn run_game(root: PathBuf) {
    let archive = resources::Resources::load(&root)
        .unwrap_or_else(|e| panic!("Cannot load TIM game data: {e}"));
    let screen = resources::screen(&archive, "SIERRA.SCR", "SIERRA.PAL")
        .unwrap_or_else(|e| panic!("Cannot decode startup screen: {e}"));
    let intro = tim_bevy::startup::animation_frames(&archive)
        .unwrap_or_else(|e| panic!("Cannot compose Sierra animation: {e}"));
    let attract = Attract {
        title: MachineSimulation::load_collection(&archive, "TITLE.GKC")
            .expect("Cannot load title simulation"),
        credits: MachineSimulation::load_collection(&archive, "CREDITS.GKC")
            .expect("Cannot load credits simulation"),
        showing_credits: false,
        updates: 0,
    };
    let font =
        tim_bevy::font::Font::load(&archive).unwrap_or_else(|e| panic!("Cannot load font: {e}"));
    let start = std::env::var("TIM_START_LEVEL")
        .ok()
        .and_then(|v| v.parse::<u16>().ok());
    let front = Frontend::new(&archive, root.clone(), start)
        .unwrap_or_else(|e| panic!("Cannot load the puzzles: {e}"));
    let (window_width, window_height) =
        tim_bevy::progress::RESOLUTIONS[usize::from(front.progress.resolution)];
    let screen_assets = game_screen::Assets::load(&archive)
        .unwrap_or_else(|e| panic!("Cannot load game screen art: {e}"));
    if let Err(error) = tim_bevy::sound::init(&archive, front.progress.volume) {
        eprintln!("Sound disabled: {error}");
    }
    tim_bevy::sound::set_channels(front.progress.music, front.progress.effects);
    let attract_music = machine::collection_metadata(&archive, "TITLE.GKC")
        .map(|h| h.music)
        .unwrap_or(1001);
    App::new()
        // Letterbox bars blend into the game's black border.
        .insert_resource(ClearColor(Color::BLACK))
        .insert_resource(StartupImage(screen))
        .insert_resource(IntroFrames(intro))
        .insert_resource(Game {
            resources: archive,
            font,
            mode: Mode::Intro,
            attract,
            front,
            screen_assets,
            cursor: None,
            prescale: 1,
            before: None,
            frame: 0,
            scroll_hold: 0.0,
            wheel_pending: 0.0,
            last_click: None,
            elapsed: 0.0,
            pending: 0.0,
            redraw: false,
            intro_cues: tim_bevy::startup::sound_cues(),
            intro_shown: 0,
            attract_music,
        })
        .add_plugins(
            DefaultPlugins
                .set(ImagePlugin::default_nearest())
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        title: tim_bevy::branding::TITLE.into(),
                        resolution: bevy::window::WindowResolution::new(
                            window_width,
                            window_height,
                        ),
                        #[cfg(target_arch = "wasm32")]
                        canvas: Some("#game".into()),
                        #[cfg(target_arch = "wasm32")]
                        prevent_default_event_handling: true,
                        ..default()
                    }),
                    ..default()
                }),
        )
        .init_resource::<CursorImages>()
        .init_resource::<TouchPointer>()
        .init_resource::<ZoomState>()
        .init_resource::<LastFrame>()
        .add_systems(
            PreUpdate,
            (emulate_mouse, wheel_zoom, page_back).after(bevy::input::InputSystems),
        )
        .add_systems(Startup, setup)
        .add_plugins(BrandingPlugin)
        .add_systems(
            Update,
            (
                ease_zoom,
                track_cursor,
                handle_input,
                advance,
                apply_graphics,
                resize_screen,
                place_playfield_camera,
                present,
                place_cursor,
                place_loupe,
                captures,
            )
                .chain(),
        )
        .run();
}
