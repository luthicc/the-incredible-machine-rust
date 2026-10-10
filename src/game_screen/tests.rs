use super::*;
use crate::definition::PartDefinition as Def;

fn resources() -> Resources {
    Resources::load(crate::test_support::game_data_root()).unwrap()
}
fn blank() -> Screen {
    Screen {
        width: 640,
        height: 480,
        rgba: vec![0; 640 * 480 * 4],
    }
}

#[test]
fn bonus_scrolls_each_borrowing_digit_in_both_counters() {
    let assets = Assets::load(&resources()).unwrap();
    for value in [401, 400, 1000] {
        let mut still = blank();
        let mut rolling = blank();
        status_page(&mut still, &assets, 0, (value, 0), (value, 0));
        status_page(&mut rolling, &assets, 0, (value, 10), (value, 10));
        for right in [0x184, 0x238] {
            let mut borrow = true;
            let mut remaining = value;
            for digit in 0..4 {
                let x = right - digit * 32;
                let changed = (395..438).any(|y| {
                    let offset = (y * 640 + x) * 4;
                    let width = assets.digits[0].width as usize * 4;
                    still.rgba[offset..offset + width] != rolling.rgba[offset..offset + width]
                });
                assert_eq!(
                    changed, borrow,
                    "value={value}, digit={digit}, counter={right}"
                );
                borrow &= remaining % 10 == 0;
                remaining /= 10;
            }
        }
    }
}

#[test]
fn carried_rope_end_and_pulley_render_taut_before_placement() {
    let res = resources();
    for routed in [false, true] {
        let mut session = PuzzleSession::load(&res, 20, 0).unwrap();
        session
            .restore(include_bytes!(
                "../test_data/puzzle20-seesaw-regression.TIM"
            ))
            .unwrap();
        let owner = session
            .editor
            .objects
            .iter()
            .position(|o| {
                o.definition == Def::Rope
                    && o.ropes[0].as_ref().is_some_and(|r| {
                        (crate::machine::rope_spans(&session.editor.objects, r)
                            .unwrap()
                            .len()
                            > 1)
                            == routed
                    })
            })
            .unwrap();
        let connection = session.editor.objects[owner].ropes[0].clone().unwrap();
        let part = if routed {
            crate::machine::rope_spans(&session.editor.objects, &connection).unwrap()[0].to
        } else {
            connection.endpoints[0].unwrap()
        };
        let o = &session.editor.objects[part];
        let cursor = (
            o.x - VIEWPORT.0 + o.width as i16 / 2,
            o.y - VIEWPORT.1 + o.height as i16 / 2,
        );
        let grab = (o.width as i16 / 2, o.height as i16 / 2);
        session.editor.objects[part].list = crate::machine::ObjectList::Inventory;
        session.editor.held = Some(crate::editor::Held {
            index: part,
            carry: crate::editor::Carry::Part { grab },
        });
        session.editor.hover((cursor.0 + 24, cursor.1 - 16));
        assert!(session.editor.held.is_some());
        // Old end allowances would visibly sag during the move.
        session.editor.objects[owner].activation_timer += 100;
        session.editor.objects[owner].callback_timer += 100;
        let before = session.editor.objects.clone();
        assert!(
            crate::machine::rope_render_segments(&before, &before[owner], &connection)
                .unwrap()
                .len()
                > crate::machine::rope_spans(&before, &connection)
                    .unwrap()
                    .len()
        );
        let carried = playfield(&res, &Assets::load(&res).unwrap(), &session, None, None).unwrap();
        assert_eq!(
            session.editor.objects[owner].activation_timer, before[owner].activation_timer,
            "render preview must not mutate the editor"
        );
        // Placement recomputes the lengths; the carried frame must already
        // match that taut geometry, including both pulley tangents.
        crate::machine::finalize_objects(&mut session.editor.objects).unwrap();
        let taut = playfield(&res, &Assets::load(&res).unwrap(), &session, None, None).unwrap();
        assert_eq!(carried.rgba, taut.rgba, "routed={routed}");
    }
}

#[test]
fn carried_part_has_native_green_or_red_marching_outline() {
    let res = resources();
    let assets = Assets::load(&res).unwrap();
    let mut session = PuzzleSession::load(&res, 1, 0).unwrap();
    assert!(session.editor.take_definition(Def::Incline));
    let index = session.editor.held.as_ref().unwrap().index;
    for (cursor, valid) in [((100, 101), true), ((100, 350), false)] {
        session.editor.hover(cursor);
        assert_eq!(session.editor.can_place(), valid);
        let mut screen = blank();
        carry_overlay(&mut screen, &assets, &session, cursor, 0);
        let o = &session.editor.objects[index];
        let x = (o.x - VIEWPORT.0).max(8) as usize;
        let y = (o.y - VIEWPORT.1).max(8) as usize;
        let expected = if valid { assets.green } else { assets.red };
        assert!(
            (x..(x + o.width as usize).min(568)).any(|px| {
                let offset = (y * 640 + px) * 4;
                screen.rgba[offset..offset + 4] == expected
            }),
            "missing placement outline: valid={valid}"
        );
    }
}

#[test]
fn sidebar_and_background_use_distinct_native_palette_entries() {
    let res = resources();
    let font = Font::load(&res).unwrap();
    let session = PuzzleSession::load(&res, 1, 0).unwrap();
    let assets = Assets::load(&res).unwrap();
    assert_eq!(
        assets.panel,
        resources::palette_color(&res, "TIM.PAL", 3).unwrap()
    );
    assert_eq!(
        assets.picker,
        resources::palette_color(&res, "TIM.PAL", 11).unwrap()
    );
    let screen = render(&res, &assets, &font, &session, None, 0).unwrap();
    let pixel = |x: usize, y: usize| screen.rgba[(y * 640 + x) * 4..(y * 640 + x + 1) * 4].to_vec();
    assert_eq!(pixel(580, 340), assets.picker);
    assert_eq!(pixel(576, 95), assets.gold);
    assert_eq!(pixel(10, 100), assets.panel);
    // Original border art must retain its DOS palette colours.
    let original = resources::sprites(&res, "GP_BORD.BMP", "TIM.PAL").unwrap();
    for (frame, native) in assets.border.iter().zip(original) {
        assert_eq!(frame.rgba, native.rgba);
    }
}
