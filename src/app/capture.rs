use super::*;

/// Optional screenshots: TIM_CAPTURE (title), TIM_CAPTURE_MOTION (title after
/// TIM_CAPTURE_MOTION_SECONDS, default 5.5), TIM_CAPTURE_CREDITS (first credits
/// frame), TIM_CAPTURE_PANEL and TIM_CAPTURE_LEVEL (after TIM_CAPTURE_LEVEL_SECONDS,
/// default 2; with TIM_RUN_LEVEL the puzzle starts running).
pub(super) fn captures(
    mut commands: Commands,
    mut game: ResMut<Game>,
    mut done: Local<[bool; 5]>,
    mut mode_since: Local<(Option<Mode>, f32)>,
) {
    if mode_since.0 != Some(game.mode) {
        *mode_since = (Some(game.mode), game.elapsed);
        if game.mode == Mode::Puzzle && std::env::var_os("TIM_RUN_LEVEL").is_some() {
            game.front.session.start();
        }
    }
    let in_mode = game.elapsed - mode_since.1;
    let seconds = |name: &str, default: f32| {
        std::env::var(name)
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(default)
    };
    let title = game.mode == Mode::Attract && !game.attract.showing_credits;
    let triggers = [
        ("TIM_CAPTURE", title && game.elapsed > 4.0),
        (
            "TIM_CAPTURE_MOTION",
            title && game.elapsed > seconds("TIM_CAPTURE_MOTION_SECONDS", 5.5),
        ),
        (
            "TIM_CAPTURE_CREDITS",
            game.mode == Mode::Attract
                && game.attract.showing_credits
                && game.attract.updates > seconds("TIM_CAPTURE_CREDITS_UPDATES", 1.0) as u32,
        ),
        (
            "TIM_CAPTURE_PANEL",
            game.mode == Mode::Panel && in_mode > 0.5,
        ),
        (
            "TIM_CAPTURE_LEVEL",
            game.mode == Mode::Puzzle && in_mode > seconds("TIM_CAPTURE_LEVEL_SECONDS", 2.0),
        ),
    ];
    for (index, (variable, ready)) in triggers.into_iter().enumerate() {
        if done[index] || !ready {
            continue;
        }
        done[index] = true;
        if let Some(path) = std::env::var_os(variable) {
            commands
                .spawn(bevy::render::view::screenshot::Screenshot::primary_window())
                .observe(bevy::render::view::screenshot::save_to_disk(PathBuf::from(
                    path,
                )));
        }
    }
}
