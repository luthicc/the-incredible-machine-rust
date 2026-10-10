use super::*;

fn resources() -> crate::resources::Resources {
    let root = crate::test_support::game_data_root();
    crate::resources::Resources::load(root).unwrap()
}

#[test]
fn magnifier_does_not_render_stale_focus_on_first_running_frame() {
    use crate::definition::PartDefinition as Def;

    let resources = resources();
    let mut session = PuzzleSession::load(&resources, 42, 0).unwrap();
    let lens = session
        .editor
        .objects
        .iter()
        .position(|o| o.definition == Def::MagnifyingGlass)
        .unwrap();
    let dynamite = session
        .editor
        .objects
        .iter()
        .position(|o| o.definition == Def::Dynamite)
        .unwrap();
    // Reproduce the malformed saved focus from the user's level 42 save:
    // a distant dynamite is persisted in the magnifier's focus slot.
    session.editor.objects[dynamite].x = session.editor.objects[lens].x + 300;
    session.editor.objects[dynamite].y = session.editor.objects[lens].y;
    session.editor.objects[lens].linked_objects[4] = Some(dynamite);

    session.start();
    let run = session.run.as_ref().unwrap();
    assert!(!run.simulation.has_stepped());
    let objects = &run.simulation.objects;
    let first_running_frame = crate::machine::live_collection_frame_with_magnifier_ray(
        &resources,
        &session.name,
        0,
        objects,
        session.running() && run.simulation.has_stepped(),
    )
    .unwrap();
    let no_focus_ray =
        crate::machine::live_collection_frame(&resources, &session.name, 0, objects).unwrap();
    assert_eq!(first_running_frame.rgba, no_focus_ray.rgba);

    session.tick();
    let simulation = &session.run.as_ref().unwrap().simulation;
    assert!(simulation.has_stepped());
}

#[test]
fn puzzle20_saved_solution_propagates_seesaw_tilts_through_pulleys() {
    use crate::definition::PartDefinition as Def;
    let resources = resources();
    let mut session = PuzzleSession::load(&resources, 20, 0).unwrap();
    session
        .restore(include_bytes!(
            "../test_data/puzzle20-seesaw-regression.TIM"
        ))
        .unwrap();
    session.start();
    let mut states = [[false; 3]; 3];
    for _ in 0..200 {
        session.tick();
        let objects = &session.run.as_ref().unwrap().simulation.objects;
        for (n, seesaw) in objects
            .iter()
            .filter(|o| o.definition == Def::Seesaw)
            .enumerate()
        {
            states[n][seesaw.state as usize] = true;
        }
        crate::machine::live_collection_frame(&resources, "L20.LEV", 0, objects).unwrap();
        if session.phase == Phase::Complete {
            break;
        }
    }
    // The first and middle seesaws move through every tilt. The final
    // rope loosens when the middle one rises, so its seesaw stays at 2.
    assert!(states[0].into_iter().all(|seen| seen));
    assert!(states[2].into_iter().all(|seen| seen));
    assert_eq!(states[1], [false, false, true]);
    assert_eq!(session.error, None);
    assert_eq!(session.phase, Phase::Complete);
}

#[test]
fn puzzle22_tethered_balloon_bounces_off_seesaw_before_hitting_basketball() {
    use crate::definition::PartDefinition as Def;
    let mut session = PuzzleSession::load(&resources(), 22, 0).unwrap();
    session
        .restore(include_bytes!(
            "../test_data/puzzle22-balloon-seesaw-regression.TIM"
        ))
        .unwrap();
    session.start();
    let sim = &mut session.run.as_mut().unwrap().simulation;
    let balloon = sim
        .objects
        .iter()
        .position(|o| o.definition == Def::Balloon && o.saved_position == (182, 299))
        .unwrap();
    let basketball = sim
        .objects
        .iter()
        .position(|o| o.definition == Def::Basketball)
        .unwrap();
    let mut bounce = None;
    let mut lowest_after_bounce = i16::MIN;
    let mut hit = None;
    // Direct simulation also tests the replay path after the goal panel.
    for tick in 0..60 {
        sim.step().unwrap();
        let p = &sim.objects[balloon];
        if p.contact
            .is_some_and(|c| sim.objects[c.target].definition == Def::Seesaw)
            && p.motion.velocity_y > 0
            && bounce.is_none()
        {
            assert_ne!(
                p.update_flags & 2,
                0,
                "rope rescan dropped the bounce classification"
            );
            assert_eq!(p.motion.velocity_y, 1544);
            bounce = Some((tick, p.y));
        }
        if bounce.is_some() && hit.is_none() {
            lowest_after_bounce = lowest_after_bounce.max(p.y);
        }
        if p.contact.is_some_and(|c| c.target == basketball) {
            assert_eq!(
                (p.state, p.power_state),
                (0, 0),
                "balloon must still be inflated at the hit"
            );
            hit = Some(tick);
            break;
        }
    }
    assert_eq!(bounce, Some((32, 110)));
    assert!(
        lowest_after_bounce >= 120,
        "balloon never dropped after impact"
    );
    assert_eq!(hit, Some(45));
    assert_ne!(sim.objects[basketball].motion.velocity_x, 0);
}

#[test]
fn puzzle18_cat_walk_and_bowl_break_keep_native_geometry() {
    use crate::definition::PartDefinition as Def;
    let mut session = PuzzleSession::load(&resources(), 18, 0).unwrap();
    session
        .restore(include_bytes!(
            "../test_data/puzzle18-cat-fish-regression.TIM"
        ))
        .unwrap();
    let fish = session
        .editor
        .objects
        .iter()
        .position(|o| o.definition == Def::Fish)
        .unwrap();
    let intact_angles = session.editor.objects[fish].outline_angles.clone();
    session.start();
    let mut walking = [false; 8];
    let mut broken = false;
    let mut walk_steps = 0;
    for _ in 0..400 {
        let before = crate::machine::positions(&session.run.as_ref().unwrap().simulation.objects);
        session.tick();
        let objects = &session.run.as_ref().unwrap().simulation.objects;
        let blended = crate::machine::blend_positions(objects, &before, 0.5);
        for (i, cat) in objects
            .iter()
            .enumerate()
            .filter(|(_, o)| o.definition == Def::Cat)
        {
            if (2..=9).contains(&cat.state) {
                walking[usize::from(cat.state - 2)] = true;
            }
            if cat.state == 2 && (cat.x - before[i].0).abs() == 32 {
                walk_steps += 1;
                assert_eq!(
                    blended[i].x, cat.x,
                    "native walking anchors must not get a second interpolated step"
                );
            }
        }
        if objects[fish].state >= 11 {
            broken = true;
            assert_eq!(
                objects[fish].outline.as_deref().unwrap(),
                &[(8, 47), (24, 44), (39, 47)]
            );
            assert_eq!(objects[fish].outline_angles, intact_angles[..3]);
        }
        if session.phase == Phase::Complete {
            break;
        }
    }
    assert!(walking.into_iter().all(|state| state));
    assert!(walk_steps > 0);
    assert!(broken);
    assert_eq!(session.error, None);
    assert_eq!(session.phase, Phase::Complete);
}

#[test]
fn detonator_composite_renders_all_press_states_in_both_directions() {
    let resources = resources();
    let session = PuzzleSession::load(&resources, 17, 0).unwrap();
    let template = session
        .editor
        .objects
        .iter()
        .find(|o| o.definition == crate::definition::PartDefinition::Detonator)
        .unwrap();
    for flipped in [false, true] {
        let mut previous = None;
        for state in 0..3 {
            let mut part = template.clone();
            part.state = state;
            part.x = 100;
            part.y = 100;
            part.render_flags
                .remove(crate::machine::RenderFlag::FlipHorizontal);
            if flipped {
                part.render_flags
                    .insert(crate::machine::RenderFlag::FlipHorizontal);
            }
            let frame =
                crate::machine::live_collection_frame(&resources, "L17.LEV", 0, &[part]).unwrap();
            if let Some(previous) = previous {
                assert_ne!(
                    frame.rgba, previous,
                    "pressed states must visibly move: state={state}, flipped={flipped}"
                );
            }
            previous = Some(frame.rgba);
        }
    }
}

#[test]
fn puzzle17_cannonball_presses_detonator_and_creates_explosion() {
    use crate::definition::PartDefinition as Def;
    let resources = resources();
    let assets = crate::game_screen::Assets::load(&resources).unwrap();
    let font = crate::font::Font::load(&resources).unwrap();
    let mut session = PuzzleSession::load(&resources, 17, 0).unwrap();
    session
        .restore(include_bytes!(
            "../test_data/puzzle17-detonator-regression.TIM"
        ))
        .unwrap();
    session.start();
    let mut impacted = false;
    let mut pressed = false;
    let mut exploded = false;
    for tick in 0..80 {
        session.tick();
        crate::game_screen::render(&resources, &assets, &font, &session, None, tick)
            .unwrap_or_else(|error| {
                panic!("puzzle 17 render failed on update {}: {error}", tick + 1)
            });
        let objects = &session.run.as_ref().unwrap().simulation.objects;
        impacted |= objects.iter().any(|p| {
            p.definition == Def::CannonProjectile
                && p.contact
                    .is_some_and(|c| c.target == 10 && c.outline_edge == 0)
        });
        pressed |= objects[10].definition == Def::Detonator
            && objects[10].state == 2
            && objects[10].power_state == 1;
        exploded |= objects
            .iter()
            .any(|p| p.definition == Def::Explosion && p.x == 353 && p.y == 121);
    }
    assert!(impacted, "cannonball never reached the detonator head");
    assert!(pressed, "detonator did not finish its press animation");
    assert!(
        exploded,
        "detonator did not create its native nose explosion"
    );
    assert_eq!(session.error, None);
}

#[test]
fn puzzle13_saved_chain_clears_the_noncolliding_bulb_and_turns_every_gear() {
    use crate::definition::PartDefinition as Def;
    let resources = resources();
    let mut session = PuzzleSession::load(&resources, 13, 0).unwrap();
    session
        .restore(include_bytes!(
            "../test_data/puzzle13-collision-regression.TIM"
        ))
        .unwrap();
    for part in &session.editor.objects {
        if part.definition == Def::Light {
            assert!(
                part.outline.is_none(),
                "native light has no collision outline"
            );
        }
        if part.definition == Def::Bucket {
            assert_eq!(
                part.outline.as_deref(),
                Some(&[(0, 19), (10, 40), (25, 40), (36, 20), (27, 47), (8, 47)][..])
            );
        }
    }
    session.start();
    let mut lower_light_lit = false;
    for _ in 0..180 {
        session.tick();
        let objects = &session.run.as_ref().unwrap().simulation.objects;
        lower_light_lit |= objects
            .iter()
            .any(|o| o.definition == Def::Light && o.saved_state == 2 && o.state == 3);
        for bucket in objects.iter().filter(|o| o.definition == Def::Bucket) {
            assert!(
                bucket
                    .contact
                    .is_none_or(|c| objects[c.target].definition != Def::Light)
            );
        }
        if session.phase == Phase::Complete {
            break;
        }
    }
    assert!(
        lower_light_lit,
        "bucket did not pull the lower light's rope"
    );
    assert_eq!(session.error, None);
    assert_eq!(session.phase, Phase::Complete);
}

#[test]
fn bonuses_start_from_the_level_header_and_roll_down() {
    let mut session = PuzzleSession::load(&resources(), 1, 0).unwrap();
    assert_eq!((session.edit_bonus, session.run_bonus), (400, 300));
    for _ in 0..22 {
        session.tick();
    }
    assert_eq!((session.edit_bonus, session.run_bonus), (399, 300));
    session.start();
    assert!(session.running());
    for _ in 0..22 {
        session.tick();
    }
    assert_eq!((session.edit_bonus, session.run_bonus), (399, 299));
    session.stop();
    assert_eq!(session.phase, Phase::Editing);
}

#[test]
fn freeform_offers_every_part_and_saves_a_loadable_machine() {
    let resources = resources();
    let mut session = PuzzleSession::freeform(&resources, None).unwrap();
    let groups = session.editor.bin_groups();
    assert!(groups.len() >= 40, "only {} freeform parts", groups.len());
    assert!(groups.iter().all(|g| g.count == 1));
    // Taking a part copies its template; the bin keeps offering it.
    session.editor.take_from_bin(0);
    session.editor.hover((200, 100));
    session.editor.click((200, 100));
    assert!(session.editor.held.is_none());
    assert_eq!(session.editor.bin_groups().len(), groups.len());
    let bytes = session.save_bytes();
    let loaded = PuzzleSession::freeform(&resources, Some(&bytes)).unwrap();
    let placed = |s: &PuzzleSession| {
        s.editor
            .objects
            .iter()
            .filter(|o| o.list != crate::machine::ObjectList::Inventory)
            .count()
    };
    assert_eq!(placed(&loaded), 1);
    assert_eq!(loaded.save_bytes(), bytes);
}

#[test]
fn a_met_goal_completes_the_puzzle_and_banks_bonuses() {
    let mut session = PuzzleSession::load(&resources(), 4, 10).unwrap();
    session.start();
    // Puzzle 4 wants a bowling ball below y 0x170: move one there.
    let run = session.run.as_mut().unwrap();
    let ball = run
        .simulation
        .objects
        .iter()
        .position(|o| o.definition == crate::definition::PartDefinition::BowlingBall)
        .unwrap();
    let x = run.simulation.objects[ball].x;
    run.simulation.objects[ball].y = 0x171;
    run.simulation.objects[ball].motion = crate::motion::Motion::at_pixels(x, 0x171);
    let bonus = session.total_bonus();
    session.tick();
    assert_eq!(session.phase, Phase::Complete);
    assert_eq!(session.score, 10 + bonus);
    session.acknowledge();
    assert_eq!(session.phase, Phase::AskReplay);
    session.replay();
    assert_eq!(session.phase, Phase::Replaying);
}
