use super::*;
#[test]
fn scaling_toggle_updates_image_and_transform_in_same_frame_with_qol_open() {
    let original = std::env::var_os("TIM_GAME_DATA")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".."));
    let archive = resources::Resources::load(&original).unwrap();
    let root = std::env::temp_dir().join(format!("tim-scaling-{}", std::process::id()));
    let mut front = Frontend::new(&archive, root.clone(), Some(1)).unwrap();
    front.qol_open = true;
    front.progress.resolution = 6;
    front.progress.stretch_screen = true;
    let font = tim_bevy::font::Font::load(&archive).unwrap();
    let screen_assets = game_screen::Assets::load(&archive).unwrap();
    let attract = Attract {
        title: MachineSimulation::load_collection(&archive, "TITLE.GKC").unwrap(),
        credits: MachineSimulation::load_collection(&archive, "CREDITS.GKC").unwrap(),
        showing_credits: false,
        updates: 0,
    };
    let game = Game {
        resources: archive,
        font,
        mode: Mode::Panel,
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
        redraw: true,
        intro_cues: Vec::new(),
        intro_shown: 0,
        attract_music: 1001,
    };
    let mut images = Assets::<Image>::default();
    let screen = resources::Screen {
        width: 640,
        height: 480,
        rgba: vec![0; 640 * 480 * 4],
    };
    let handle = images.add(screen_image(&screen.rgba, 640, 480, 1));
    let mut app = App::new();
    app.init_resource::<ButtonInput<KeyCode>>()
        .init_resource::<ButtonInput<MouseButton>>()
        .init_resource::<ZoomState>()
        .init_resource::<LastFrame>()
        .add_message::<KeyboardInput>()
        .add_message::<MouseWheel>()
        .add_message::<AppExit>();
    app.insert_resource(images)
        .insert_resource(StartupImage(screen))
        .insert_resource(IntroFrames(Vec::new()))
        .insert_resource(game);
    app.world_mut().spawn(Window {
        resolution: (1600, 900).into(),
        ..default()
    });
    let sprite = app
        .world_mut()
        .spawn((
            Sprite::from_image(handle.clone()),
            ScreenSprite,
            Transform::default(),
        ))
        .id();
    app.add_systems(Update, (handle_input, resize_screen, present).chain());
    app.update();
    assert_eq!(
        app.world()
            .resource::<Assets<Image>>()
            .get(&handle)
            .unwrap()
            .width(),
        1920
    );
    {
        let mut game = app.world_mut().resource_mut::<Game>();
        game.front.progress.stretch_screen = false;
        game.redraw = true;
    }
    app.update();
    assert!(app.world().resource::<Game>().front.qol_open);
    assert_eq!(
        app.world()
            .resource::<Assets<Image>>()
            .get(&handle)
            .unwrap()
            .width(),
        1280
    );
    let transform = app.world().get::<Transform>(sprite).unwrap();
    assert_eq!(transform.scale, Vec3::new(0.9375, 0.9375, 1.0));
    {
        let mut game = app.world_mut().resource_mut::<Game>();
        game.front.progress.pixel_scaling = false;
        game.front.progress.stretch_screen = true;
        game.redraw = true;
    }
    app.update();
    assert!(app.world().resource::<Game>().front.qol_open);
    assert_eq!(
        app.world()
            .resource::<Assets<Image>>()
            .get(&handle)
            .unwrap()
            .width(),
        640
    );
    let transform = app.world().get::<Transform>(sprite).unwrap();
    assert_eq!(transform.scale, Vec3::ONE);
    assert_eq!(transform.translation, Vec3::ZERO);
    let switches: Vec<bool> = tim_bevy::progress::QOL_OPTIONS
        .iter()
        .map(|(option, _)| {
            app.world()
                .resource::<Game>()
                .front
                .progress
                .qol_value(*option)
        })
        .collect();
    app.add_systems(
        Update,
        apply_graphics.after(handle_input).before(resize_screen),
    );
    click_menu(&mut app, (400, 108));
    for (i, &(width, height)) in tim_bevy::progress::RESOLUTIONS.iter().enumerate() {
        click_menu(&mut app, (400, 128 + i as i16 * 20));
        let window = app
            .world_mut()
            .query::<&Window>()
            .single(app.world())
            .unwrap();
        assert_eq!(
            (window.physical_width(), window.physical_height()),
            (width, height)
        );
        let front = &app.world().resource::<Game>().front;
        assert!(front.qol_open && front.graphics_menu.resolution_open);
        let actual: Vec<bool> = tim_bevy::progress::QOL_OPTIONS
            .iter()
            .map(|(option, _)| front.progress.qol_value(*option))
            .collect();
        assert_eq!(
            actual, switches,
            "dropdown must not change switches behind it"
        );
    }
    click_menu(&mut app, (200, 132));
    assert!(!app.world().resource::<Game>().front.progress.pixel_scaling);
    assert!(app.world().resource::<Game>().front.qol_open);
    app.world_mut()
        .query::<&mut Window>()
        .single_mut(app.world_mut())
        .unwrap()
        .resolution
        .set_physical_resolution(900, 600);
    app.update();
    let window = app
        .world_mut()
        .query::<&Window>()
        .single(app.world())
        .unwrap();
    assert_eq!(
        (window.physical_width(), window.physical_height()),
        (900, 600)
    );
    std::fs::remove_dir_all(root).unwrap();
}

fn click_menu(app: &mut App, cursor: (i16, i16)) {
    app.world_mut().resource_mut::<Game>().cursor = Some(cursor);
    {
        let mut mouse = app.world_mut().resource_mut::<ButtonInput<MouseButton>>();
        mouse.clear();
        mouse.press(MouseButton::Left);
    }
    app.update();
    {
        let mut mouse = app.world_mut().resource_mut::<ButtonInput<MouseButton>>();
        mouse.clear();
        mouse.release(MouseButton::Left);
    }
    app.update();
    app.world_mut()
        .resource_mut::<ButtonInput<MouseButton>>()
        .clear();
}
