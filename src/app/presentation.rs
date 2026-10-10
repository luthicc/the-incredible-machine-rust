use super::*;

pub(super) fn setup(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    screen: Res<StartupImage>,
) {
    commands.spawn(Camera2d);
    let image = screen_image(&screen.0.rgba, screen.0.width, screen.0.height, 1);
    commands.spawn((Sprite::from_image(images.add(image)), ScreenSprite));
    commands.spawn((
        Sprite::default(),
        bevy::sprite::Anchor::TOP_LEFT,
        Transform::from_xyz(0.0, 0.0, 1.0),
        Visibility::Hidden,
        bevy::camera::visibility::RenderLayers::layer(OVERLAY_LAYER),
        CursorSprite,
    ));
    spawn_magnifiers(&mut commands, &mut images);
}

/// Show the current cursor frame at the exact pointer position.
pub(super) fn place_cursor(
    window: Query<&Window>,
    touch: Res<TouchPointer>,
    game: Res<Game>,
    mut cached: ResMut<CursorImages>,
    mut images: ResMut<Assets<Image>>,
    mut cursors: Query<(&mut Sprite, &mut Transform, &mut Visibility), With<CursorSprite>>,
) {
    let Ok(window) = window.single() else {
        return;
    };
    let Ok((mut sprite, mut transform, mut visibility)) = cursors.single_mut() else {
        return;
    };
    // Touch draws no pointer, except to show a belt or rope being carried;
    // the loupe shows what the finger is on.
    let tool = game.front.session.editor.held.as_ref().is_some_and(|h| {
        matches!(
            h.carry,
            tim_bevy::editor::Carry::Belt { .. } | tim_bevy::editor::Carry::Rope { .. }
        )
    });
    let shown = cursor_frame(&game)
        .filter(|_| !touch.active() || (tool && game.mode == Mode::Puzzle))
        .zip(touch.pointer(window));
    let Some((frame, pointer)) = shown else {
        visibility.set_if_neq(Visibility::Hidden);
        return;
    };
    let viewport = tim_bevy::viewport::Viewport::presentation(
        (window.width(), window.height()),
        window.scale_factor(),
        (640, 480),
        game.front.progress.stretch_screen,
        game.front.progress.pixel_scaling,
    );
    if cached.prescale != viewport.prescale {
        let assets = &game.screen_assets;
        let frames = (0..assets.mouse_frames())
            .filter_map(|i| assets.mouse_frame(i))
            .map(|s| images.add(screen_image(&s.rgba, s.width, s.height, viewport.prescale)))
            .collect();
        *cached = CursorImages {
            prescale: viewport.prescale,
            frames,
        };
    }
    let Some(image) = cached.frames.get(frame) else {
        visibility.set_if_neq(Visibility::Hidden);
        return;
    };
    if sprite.image != *image {
        sprite.image = image.clone();
    }
    let scale = viewport.scale / viewport.prescale as f32;
    transform.scale = Vec3::new(scale, scale, 1.0);
    transform.translation.x = pointer.x - window.width() / 2.0;
    transform.translation.y = window.height() / 2.0 - pointer.y;
    visibility.set_if_neq(Visibility::Visible);
}

/// The presented image: the 640x480 screen enlarged by `prescale` with
/// nearest pixels, sampled linearly when fitted to the window.
pub(super) fn screen_image(rgba: &[u8], width: u32, height: u32, prescale: u32) -> Image {
    let mut image = Image::new(
        Extent3d {
            width: width * prescale,
            height: height * prescale,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        tim_bevy::viewport::upscale(rgba, width, prescale),
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default(),
    );
    image.sampler = ImageSampler::linear();
    image
}

pub(super) fn resize_screen(
    window: Query<&Window>,
    screen: Res<StartupImage>,
    zoom: Res<ZoomState>,
    mut game: ResMut<Game>,
    mut sprites: Query<&mut Transform, With<ScreenSprite>>,
) {
    let Ok(window) = window.single() else {
        return;
    };
    let viewport = base_viewport(window, &game);
    // Enlarge enough that the zoomed playfield stays sharp too.
    let zoomed = viewport.scale.max(viewport.scale_y) * zoom.0.factor * window.scale_factor();
    let prescale = viewport.prescale.max((zoomed.ceil() as u32).clamp(1, 8));
    if prescale != game.prescale {
        game.prescale = prescale;
        game.redraw = true;
    }
    let scale = viewport.scale;
    let sprite_scale = scale / game.prescale as f32;
    for mut transform in &mut sprites {
        transform.scale = Vec3::new(sprite_scale, viewport.scale_y / game.prescale as f32, 1.0);
        transform.translation.x =
            viewport.origin.0 + screen.0.width as f32 * scale / 2.0 - window.width() / 2.0;
        transform.translation.y = window.height() / 2.0
            - viewport.origin.1
            - screen.0.height as f32 * viewport.scale_y / 2.0;
    }
}

/// The MOUSE.BMP frame for the pointer, in the modes that draw one.
pub(super) fn cursor_frame(game: &Game) -> Option<usize> {
    let cursor = game.cursor?;
    let front = &game.front;
    match game.mode {
        Mode::Panel => Some(match front.dialog {
            None => menus::control_cursor(menus::control_at(cursor), front.session.freeform),
            Some(_) => 0,
        }),
        Mode::Puzzle if front.qol_open || front.solutions.open => Some(0),
        Mode::Puzzle => Some(game_screen::puzzle_cursor(&front.session, cursor)),
        Mode::Intro | Mode::Attract => None,
    }
}

pub(super) fn present(
    mut game: ResMut<Game>,
    intro: Res<IntroFrames>,
    startup: Res<StartupImage>,
    mut images: ResMut<Assets<Image>>,
    mut last: ResMut<LastFrame>,
    sprites: Query<&Sprite, With<ScreenSprite>>,
) {
    if !game.redraw {
        return;
    }
    game.redraw = false;
    let rgba = match compose(&game, &intro, &startup) {
        Ok(rgba) => rgba,
        Err(error) => {
            eprintln!("Cannot render {:?}: {error}", game.mode);
            return;
        }
    };
    last.0.clone_from(&rgba);
    let (width, height, prescale) = (startup.0.width, startup.0.height, game.prescale);
    for sprite in &sprites {
        if let Some(image) = images.get_mut(&sprite.image) {
            if image.width() == width * prescale && image.height() == height * prescale {
                image.data = Some(tim_bevy::viewport::upscale(&rgba, width, prescale));
            } else {
                *image = screen_image(&rgba, width, height, prescale);
            }
        }
    }
}

/// Apply menu choices once, allowing the user to resize the window manually afterward.
/// In the browser the canvas takes the chosen size, shrunk to fit the page.
pub(super) fn apply_graphics(
    mut windows: Query<&mut Window>,
    mut game: ResMut<Game>,
    mut applied: Local<Option<(u8, u32, u32)>>,
) {
    let choice = game.front.progress.resolution;
    let (width, height) = tim_bevy::progress::RESOLUTIONS[usize::from(choice)];
    #[cfg(target_arch = "wasm32")]
    let (width, height) = fit_page(width, height);
    if *applied == Some((choice, width, height)) {
        return;
    }
    if let Ok(mut window) = windows.single_mut() {
        if cfg!(target_arch = "wasm32") {
            window.resolution.set(width as f32, height as f32);
        } else {
            window.resolution.set_physical_resolution(width, height);
        }
    }
    *applied = Some((choice, width, height));
    game.redraw = true;
}

/// The largest size up to `width` x `height`, in CSS pixels, that keeps 4:3 inside the page.
#[cfg(target_arch = "wasm32")]
fn fit_page(width: u32, height: u32) -> (u32, u32) {
    let Some(page) = web_sys::window() else {
        return (width, height);
    };
    let size = |v: Result<wasm_bindgen::JsValue, _>| v.ok().and_then(|v| v.as_f64());
    let (Some(page_w), Some(page_h)) = (size(page.inner_width()), size(page.inner_height())) else {
        return (width, height);
    };
    let scale = (page_w / f64::from(width))
        .min(page_h / f64::from(height))
        .min(1.0);
    let height = (f64::from(height) * scale).floor() as u32;
    (height * 4 / 3, height)
}
