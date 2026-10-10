use super::*;

pub(super) fn compose(
    game: &Game,
    intro: &IntroFrames,
    startup: &StartupImage,
) -> Result<Vec<u8>, String> {
    Ok(match game.mode {
        Mode::Intro => {
            let frame = ((game.elapsed - 1.2) / 0.06).floor();
            if frame < 0.0 {
                startup.0.rgba.clone()
            } else {
                intro
                    .0
                    .get(frame as usize)
                    .or(intro.0.last())
                    .unwrap_or(&startup.0)
                    .rgba
                    .clone()
            }
        }
        Mode::Attract => {
            let attract = &game.attract;
            let (name, simulation) = if attract.showing_credits {
                ("CREDITS.GKC", &attract.credits)
            } else {
                ("TITLE.GKC", &attract.title)
            };
            let objects = match game
                .before
                .as_ref()
                .filter(|_| game.front.progress.smooth_motion)
            {
                Some(before) => machine::blend_positions(&simulation.objects, before, game.pending),
                None => simulation.objects.clone(),
            };
            machine::live_collection_frame(&game.resources, name, 0, &objects)?.rgba
        }
        Mode::Panel => {
            let front = &game.front;
            let assets = &game.screen_assets;
            let view = menus::PanelView {
                solution_name: front.solutions.selected_name(),
                session: &front.session,
                description: front.description(),
                volume: front.progress.volume,
                smooth_motion: front.progress.smooth_motion,
                puzzle_hints: front.progress.puzzle_hints,
                music: front.progress.music,
                effects: front.progress.effects,
                speed: front.progress.speed,
                pressed: front.pressed,
            };
            let mut screen = menus::render_panel(&game.resources, assets, &game.font, &view)?;
            match &front.dialog {
                Some(Dialog::Message(message, _)) => {
                    let hovered = game.cursor.and_then(|c| message.choice(&game.font, c));
                    message.render(&mut screen, assets, &game.font, hovered);
                }
                Some(Dialog::Select(select)) => select.render(
                    &mut screen,
                    assets,
                    &game.font,
                    &front.titles,
                    front.available_levels(),
                    &front.progress.completed,
                    front.select_pressed,
                    game.frame,
                ),
                Some(Dialog::File(file)) => file.render(
                    &mut screen,
                    assets,
                    &game.font,
                    front.file_pressed,
                    game.frame,
                ),
                None => {}
            }
            if front.qol_open {
                menus::draw_qol(
                    &mut screen,
                    assets,
                    &game.font,
                    &front.progress,
                    &front.graphics_menu,
                    game.cursor,
                );
            }
            if front.solutions.open {
                menus::draw_solutions(&mut screen, assets, &game.font, &front.solutions);
            }
            screen.rgba
        }
        Mode::Puzzle => {
            let smooth = game.front.progress.smooth_motion;
            let blend = game
                .before
                .as_deref()
                .filter(|_| smooth)
                .map(|before| (before, game.pending));
            let front = &game.front;
            let mut screen = game_screen::render_blended(
                &game.resources,
                &game.screen_assets,
                &game.font,
                &front.session,
                game.cursor,
                game.frame,
                blend,
            )?;
            let options = menus::Options {
                smooth_motion: front.progress.smooth_motion,
                puzzle_hints: front.progress.puzzle_hints,
                music: front.progress.music,
                effects: front.progress.effects,
                speed: front.progress.speed,
            };
            menus::draw_options(
                &mut screen,
                &game.screen_assets,
                &game.font,
                &options,
                front.pressed,
                front.solutions.selected_name(),
            );
            if front.qol_open {
                menus::draw_qol(
                    &mut screen,
                    &game.screen_assets,
                    &game.font,
                    &front.progress,
                    &front.graphics_menu,
                    game.cursor,
                );
            }
            if front.solutions.open {
                menus::draw_solutions(
                    &mut screen,
                    &game.screen_assets,
                    &game.font,
                    &front.solutions,
                );
            }
            screen.rgba
        }
    })
}
