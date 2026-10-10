use super::*;

/// Pointer position in 640x480 source pixels, through the letterboxed
/// viewport and the playfield zoom.
pub(super) fn source_pointer(
    windows: &Query<&Window>,
    touch: &TouchPointer,
    zoom: &ZoomState,
    game: &Game,
) -> Option<Vec2> {
    let window = windows.single().ok()?;
    let pointer = touch.pointer(window)?;
    let (x, y) = window_to_source(window, game, zoom, pointer)?;
    Some(Vec2::new(x, y))
}

/// The source pixel shown at a logical window point.
pub(super) fn window_to_source(
    window: &Window,
    game: &Game,
    zoom: &ZoomState,
    point: Vec2,
) -> Option<(f32, f32)> {
    let point = base_viewport(window, game).source_point((point.x, point.y))?;
    Some(zoom.0.map(point))
}

/// The 640x480 screen fitted to the window, before zoom.
pub(super) fn base_viewport(window: &Window, game: &Game) -> tim_bevy::viewport::Viewport {
    tim_bevy::viewport::Viewport::presentation(
        (window.width(), window.height()),
        window.scale_factor(),
        (640, 480),
        game.front.progress.stretch_screen,
        game.front.progress.pixel_scaling,
    )
}

pub(super) fn handle_input(
    keys: Res<ButtonInput<KeyCode>>,
    mouse: Res<ButtonInput<MouseButton>>,
    mut typed: MessageReader<KeyboardInput>,
    mut wheel: MessageReader<MouseWheel>,
    mut game: ResMut<Game>,
    mut exit: MessageWriter<AppExit>,
) {
    let mut text = String::new();
    for key in typed.read() {
        if key.state.is_pressed() {
            if key.key_code == KeyCode::Backspace {
                text.push('\u{8}');
            } else if let Some(t) = &key.text {
                text.push_str(t);
            }
        }
    }
    game.front.session.editor.connection_hints = game.front.progress.connection_hints;
    game.front.session.editor.plug_preview = game.front.progress.plug_preview;
    let wheel_delta: f32 = wheel
        .read()
        .map(|event| match event.unit {
            MouseScrollUnit::Line => event.y,
            MouseScrollUnit::Pixel => event.y / 40.0,
        })
        .sum::<f32>()
        * if ctrl_held(&keys) { 0.0 } else { 1.0 };
    if matches!(game.mode, Mode::Puzzle | Mode::Panel) && game.front.solutions.open {
        if keys.just_pressed(KeyCode::Escape) {
            game.front.solutions.open = false;
        } else {
            if game.front.solutions.editing_name {
                menus::edit_text(&mut game.front.solutions.name, &text, 30);
                if keys.just_pressed(KeyCode::Enter) {
                    game.front.solutions.editing_name = false;
                }
            }
            let action = if mouse.just_released(MouseButton::Left) {
                if game
                    .cursor
                    .is_some_and(|c| game_screen::inside(menus::SOLUTIONS_BUTTON, c))
                {
                    None
                } else if game.cursor.is_some_and(|c| {
                    !game_screen::inside(menus::solution_panel(&game.front.solutions), c)
                }) {
                    Some(menus::SolutionAction::Done)
                } else {
                    game.cursor
                        .and_then(|c| menus::solution_hit(&game.front.solutions, c))
                }
            } else {
                None
            };
            if let Some(action) = action {
                let game = &mut *game;
                game.front.solution_action(&game.resources, action);
                game.sync_mode();
                if game.front.session.running() {
                    game.before = None;
                }
            }
        }
        game.last_click = None;
        game.scroll_hold = 0.0;
        game.redraw = true;
        return;
    }
    if matches!(game.mode, Mode::Puzzle | Mode::Panel)
        && keys.just_pressed(KeyCode::F2)
        && game.front.dialog.is_none()
        && !game.front.qol_open
    {
        game.front.open_solutions();
        game.before = None;
        game.redraw = true;
        return;
    }
    if matches!(game.mode, Mode::Puzzle | Mode::Panel) && game.front.qol_open {
        if keys.just_pressed(KeyCode::Escape) {
            if !game.front.graphics_menu.close_overlay() {
                game.front.qol_open = false;
            }
        } else if keys.just_pressed(KeyCode::Enter) {
            if !game.front.graphics_menu.close_overlay() {
                game.front.qol_open = false;
            }
        } else {
            if mouse.just_pressed(MouseButton::Left) {
                game.front.graphics_menu.pressed = game.cursor;
            }
            if mouse.just_released(MouseButton::Left) {
                let cursor = game.cursor;
                let front = &mut game.front;
                if let Some(start) = front.graphics_menu.pressed.take() {
                    if let Some(c) = cursor {
                        if front.graphics_menu.click(&mut front.progress, c) {
                            front.save_progress();
                        } else if let Some(row) =
                            menus::qol_hit(c).filter(|row| menus::qol_hit(start) == Some(*row))
                        {
                            front
                                .progress
                                .toggle_qol(tim_bevy::progress::QOL_OPTIONS[row].0);
                            front.save_progress();
                        } else if game_screen::inside(menus::QOL_DONE, start)
                            && game_screen::inside(menus::QOL_DONE, c)
                        {
                            front.qol_open = false;
                        }
                    }
                }
            }
        }
        game.last_click = None;
        game.scroll_hold = 0.0;
        game.redraw = true;
        return;
    }
    if game.mode == Mode::Puzzle
        && game.front.session.phase == Phase::Editing
        && game.front.progress.wheel_scroll
        && wheel_delta != 0.0
        && game
            .cursor
            .is_some_and(|c| (576..640).contains(&c.0) && (64..360).contains(&c.1))
    {
        game.wheel_pending += wheel_delta;
        let steps = game.wheel_pending.trunc().clamp(-5.0, 5.0) as isize;
        game.wheel_pending -= steps as f32;
        game.front.session.editor.scroll_bin(-steps);
        game.redraw = true;
    } else if !game
        .cursor
        .is_some_and(|c| (576..640).contains(&c.0) && (64..360).contains(&c.1))
    {
        game.wheel_pending = 0.0;
    }
    let click = mouse.just_pressed(MouseButton::Left);
    let pressed = |codes: &[KeyCode]| codes.iter().any(|&code| keys.just_pressed(code));
    match game.mode {
        Mode::Intro => {
            if pressed(&[KeyCode::Space, KeyCode::Enter]) || click {
                game.mode = Mode::Attract;
                game.reset_attract();
            }
            if pressed(&[KeyCode::Escape]) {
                exit.write(AppExit::Success);
            }
        }
        Mode::Attract => {
            let auto_panel = std::env::var_os("TIM_START_PANEL").is_some() && game.elapsed > 3.5;
            let auto_level = std::env::var_os("TIM_START_LEVEL").is_some();
            if auto_level {
                game.start_puzzle();
            } else if pressed(&[KeyCode::Enter, KeyCode::Space]) || click || auto_panel {
                game.open_panel();
            } else if pressed(&[KeyCode::Escape]) {
                exit.write(AppExit::Success);
            }
        }
        Mode::Panel => panel_input(&mut game, &keys, &mouse, &text),
        Mode::Puzzle => {
            // run_machine_play_loop: 1-9 and A-G pick a music track.
            if game.front.session.phase == Phase::Editing {
                if let Some(id) = text.chars().find_map(tim_bevy::sound::music_for_key) {
                    game.front.session.music = id;
                }
            }
            puzzle_input(&mut game, &keys, &mouse)
        }
    }
    if game.front.quit {
        game.front.autosave();
        exit.write(AppExit::Success);
    }
}

/// The launch control panel (run_launch_control_panel): controls act on
/// release, the meters while held, and dialogs take typed text.
pub(super) fn panel_input(
    game: &mut Game,
    keys: &ButtonInput<KeyCode>,
    mouse: &ButtonInput<MouseButton>,
    text: &str,
) {
    let pressed = |codes: &[KeyCode]| codes.iter().any(|&code| keys.just_pressed(code));
    let enter = pressed(&[KeyCode::Enter, KeyCode::NumpadEnter]);
    let escape = pressed(&[KeyCode::Escape]);
    let game = &mut *game;
    let front = &mut game.front;
    if front.dialog.is_some() {
        let typed: String = text
            .chars()
            .filter(|c| *c != '\r' && *c != '\n' && *c != '\u{1b}')
            .collect();
        if !typed.is_empty() || enter || escape {
            front.type_text(&game.resources, &typed, enter, escape);
        }
    } else if escape {
        front.activate(&game.resources, menus::Control::Quit);
    } else if enter || pressed(&[KeyCode::Space]) {
        front.activate(&game.resources, menus::Control::Play);
    }
    if let Some(cursor) = game.cursor {
        if mouse.just_pressed(MouseButton::Left) {
            front.panel_press(cursor);
        } else if mouse.pressed(MouseButton::Left) {
            front.panel_drag(cursor);
        }
        if mouse.just_released(MouseButton::Left) {
            if matches!(front.dialog, Some(Dialog::Message(..))) {
                front.message_click(&game.resources, &game.font, cursor);
            } else {
                front.panel_release(&game.resources, cursor);
            }
        }
    }
    game.sync_mode();
}

/// Game screen input. Left clicks follow the UI elements of
/// create_ui_element_list; a click while the machine runs stops it.
pub(super) fn puzzle_input(
    game: &mut Game,
    keys: &ButtonInput<KeyCode>,
    mouse: &ButtonInput<MouseButton>,
) {
    let was_running = game.front.session.running();
    puzzle_controls(game, keys, mouse);
    // A run started or stopped: its last positions no longer apply.
    if game.front.session.running() != was_running {
        game.before = None;
    }
}

pub(super) fn puzzle_controls(
    game: &mut Game,
    keys: &ButtonInput<KeyCode>,
    mouse: &ButtonInput<MouseButton>,
) {
    let pressed = |codes: &[KeyCode]| codes.iter().any(|&code| keys.just_pressed(code));
    if pressed(&[KeyCode::Escape]) {
        let session = &game.front.session;
        let carrying = session.editor.held.is_some() || session.editor.resizing.is_some();
        if carrying {
            game.front.session.editor.cancel();
        } else {
            game.open_panel();
        }
        return;
    }
    let cursor = game.cursor;
    let click = mouse.just_pressed(MouseButton::Left);
    let right = mouse.just_pressed(MouseButton::Right);
    // The option row (music, effects, speed...) works here too, even while
    // the machine runs; its clicks never reach the playfield.
    if let (true, Some(c)) = (click, cursor) {
        if game.front.option_press(c) {
            return;
        }
    }
    if game.front.pressed.is_some() {
        if mouse.just_released(MouseButton::Left) || !mouse.pressed(MouseButton::Left) {
            game.front
                .option_release(&game.resources, cursor.unwrap_or((-1, -1)));
        } else if let Some(c) = cursor {
            game.front.option_drag(c);
        }
        return;
    }
    let shift = game.front.progress.shift_copy
        && (keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight));
    let double_copy = game.front.progress.double_click_copy;
    let slow_scroll = game.front.progress.slow_scroll;
    let z_flip = game.front.progress.z_flip;
    if !mouse.pressed(MouseButton::Left) {
        game.scroll_hold = 0.0;
    }
    let mut advance = false;
    {
        let session = &mut game.front.session;
        if let Some(c) = cursor {
            session.editor.hover(c);
        }
        match session.phase {
            Phase::Editing => {
                let hovered = cursor.map(|c| game_screen::hover(session, c));
                if z_flip && pressed(&[KeyCode::KeyZ]) {
                    let part = session
                        .editor
                        .held
                        .as_ref()
                        .filter(|h| matches!(h.carry, tim_bevy::editor::Carry::Part { .. }))
                        .map(|h| h.index)
                        .or_else(|| cursor.and_then(|c| session.editor.select_at(c)));
                    if let Some(part) = part {
                        session.editor.flip(part, 1);
                    }
                } else if pressed(&[KeyCode::Space, KeyCode::Enter]) {
                    session.start();
                } else if right {
                    session.editor.cancel();
                } else if click {
                    match (hovered, cursor) {
                        (Some(game_screen::Hover::Start), _) => session.start(),
                        (Some(game_screen::Hover::ScrollUp), _) => {
                            session.editor.scroll_bin(-1);
                            game.scroll_hold = game.elapsed + if slow_scroll { 0.4 } else { 0.1 };
                        }
                        (Some(game_screen::Hover::ScrollDown), _) => {
                            session.editor.scroll_bin(1);
                            game.scroll_hold = game.elapsed + if slow_scroll { 0.4 } else { 0.1 };
                        }
                        (Some(game_screen::Hover::BinSlot(slot)), _) => {
                            if session.editor.held.is_some() {
                                session.editor.cancel();
                            } else {
                                session.editor.take_from_bin(slot);
                                // Follow the cursor this frame, not from its stored spot.
                                if let Some(c) = cursor {
                                    session.editor.hover(c);
                                }
                            }
                        }
                        (_, Some(c)) if game_screen::inside(game_screen::PLAYFIELD, c) => {
                            let editor = &mut session.editor;
                            let definition = |e: &tim_bevy::editor::Editor| {
                                e.held.as_ref().map(|h| e.objects[h.index].definition)
                            };
                            let before = definition(editor);
                            editor.click(c);
                            // A placement finished: Shift, or the second click of
                            // a double click on a placed part, takes another.
                            if let (Some(placed), None) = (before, editor.held.as_ref()) {
                                let double = double_copy
                                    && matches!(game.last_click, Some((at, (x, y), Some(d)))
                                    if game.elapsed - at < 0.4 && (x - c.0).abs() <= 4 && (y - c.1).abs() <= 4 && d == placed);
                                if (shift || double) && editor.take_definition(placed) {
                                    editor.hover(c);
                                }
                            }
                            let picked = if before.is_none() {
                                editor
                                    .held
                                    .as_ref()
                                    .filter(|h| {
                                        matches!(h.carry, tim_bevy::editor::Carry::Part { .. })
                                    })
                                    .map(|h| editor.objects[h.index].definition)
                            } else {
                                None
                            };
                            game.last_click = Some((game.elapsed, c, picked));
                        }
                        _ => {}
                    }
                } else if mouse.pressed(MouseButton::Left) {
                    let direction = match hovered {
                        Some(game_screen::Hover::ScrollUp) => Some(-1),
                        Some(game_screen::Hover::ScrollDown) => Some(1),
                        _ => None,
                    };
                    if let Some(direction) = direction {
                        if game.scroll_hold == 0.0 {
                            game.scroll_hold = game.elapsed + if slow_scroll { 0.4 } else { 0.1 };
                        } else if game.elapsed >= game.scroll_hold {
                            session.editor.scroll_bin(direction);
                            game.scroll_hold = game.elapsed + if slow_scroll { 0.15 } else { 0.05 };
                        }
                    } else {
                        game.scroll_hold = 0.0;
                    }
                }
            }
            Phase::Running => {
                if click || right || pressed(&[KeyCode::Space, KeyCode::Enter]) {
                    session.stop();
                }
            }
            Phase::Replaying => {
                if click || right || pressed(&[KeyCode::Space, KeyCode::Enter]) {
                    session.stop();
                    session.phase = Phase::AskReplay;
                }
            }
            Phase::Complete => {
                if click || right || pressed(&[KeyCode::Space, KeyCode::Enter]) {
                    session.acknowledge();
                }
            }
            Phase::AskReplay => {
                let choice = cursor.and_then(|c| game_screen::replay_choice(&game.font, c));
                if (click && choice == Some(game_screen::ReplayChoice::Replay))
                    || pressed(&[KeyCode::KeyR])
                {
                    session.replay();
                } else if (click && choice == Some(game_screen::ReplayChoice::Advance))
                    || pressed(&[KeyCode::Space, KeyCode::Enter, KeyCode::KeyA])
                {
                    advance = true;
                }
            }
        }
    }
    if advance {
        game.front.advance(&game.resources);
        game.sync_mode();
    }
}

/// Track the pointer in source pixels and hide the system cursor over the
/// game screen, which draws the original MOUSE.BMP cursors. Letterbox bars
/// keep the system cursor.
pub(super) fn track_cursor(
    windows: Query<&Window>,
    touch: Res<TouchPointer>,
    zoom: Res<ZoomState>,
    mut cursors: Query<&mut bevy::window::CursorOptions>,
    mut game: ResMut<Game>,
) {
    let cursor = source_pointer(&windows, &touch, &zoom, &game).map(|p| (p.x as i16, p.y as i16));
    if cursor != game.cursor {
        game.cursor = cursor;
        if matches!(game.mode, Mode::Puzzle | Mode::Panel) {
            game.redraw = true;
        }
    }
    let visible = cursor.is_none() || !matches!(game.mode, Mode::Puzzle | Mode::Panel);
    for mut options in &mut cursors {
        if options.visible != visible {
            options.visible = visible;
        }
    }
}
