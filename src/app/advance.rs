use super::*;

pub(super) fn advance(time: Res<Time>, intro: Res<IntroFrames>, mut game: ResMut<Game>) {
    let game = &mut *game;
    let delta = time.delta_secs();
    game.elapsed += delta;
    // select_sound_bank: the title machines' music, else the puzzle's.
    tim_bevy::sound::select_music(match game.mode {
        Mode::Intro => None,
        Mode::Attract => Some(game.attract_music),
        Mode::Panel | Mode::Puzzle => Some(game.front.session.music),
    });
    match game.mode {
        Mode::Intro => {
            let shown = (((game.elapsed - 1.2) / 0.06).floor().max(-1.0) + 1.0) as usize;
            while game.intro_shown < shown.min(game.intro_cues.len()) {
                for &effect in &game.intro_cues[game.intro_shown] {
                    tim_bevy::sound::play_effect(effect);
                }
                game.intro_shown += 1;
            }
            // Startup table advances after six original countdown ticks.
            if game.elapsed >= 1.2 + intro.0.len() as f32 * 0.06 {
                game.mode = Mode::Attract;
                game.reset_attract();
            }
            game.redraw = true;
        }
        Mode::Attract => {
            game.pending += delta * updates_per_second();
            while game.pending >= 1.0 {
                game.pending -= 1.0;
                game.redraw = true;
                let attract = &mut game.attract;
                let limit = if attract.showing_credits {
                    CREDITS_UPDATES
                } else {
                    TITLE_UPDATES
                };
                if attract.updates >= limit {
                    tim_bevy::sound::stop_machine();
                    attract.showing_credits = !attract.showing_credits;
                    attract.updates = 0;
                    if attract.showing_credits {
                        attract.credits.reset();
                    } else {
                        attract.title.reset();
                    }
                }
                let simulation = if attract.showing_credits {
                    &mut attract.credits
                } else {
                    &mut attract.title
                };
                game.before = Some(machine::positions(&simulation.objects));
                // A callback failure freezes that machine until its next reset.
                if let Err(error) = simulation.step() {
                    eprintln!("Attract simulation stopped: {error:?}");
                }
                attract.updates += 1;
            }
            // Redraw every frame so the machine moves between updates.
            game.redraw = true;
        }
        Mode::Panel => {
            game.pending += delta * updates_per_second();
            while game.pending >= 1.0 {
                game.pending -= 1.0;
                game.frame = game.frame.wrapping_add(1);
            }
            game.redraw = true;
        }
        Mode::Puzzle => {
            // The control panel's speed slider scales a running machine only;
            // editing keeps the native rate so the edit bonus is unchanged.
            let speed = if game.front.session.running() {
                game.front.progress.speed()
            } else {
                1.0
            };
            game.pending += delta * updates_per_second() * speed;
            while game.pending >= 1.0 {
                game.pending -= 1.0;
                game.frame = game.frame.wrapping_add(1);
                let session = &game.front.session;
                game.before = session
                    .run
                    .as_ref()
                    .filter(|_| session.running())
                    .map(|run| machine::positions(&run.simulation.objects));
                game.front.session.tick();
                game.front.update_progress();
                // Keep the puzzle's machine saved while it is being built.
                if game.frame % 30 == 0 && !game.front.session.running() {
                    game.front.autosave();
                }
            }
            game.redraw = true;
        }
    }
}
