use super::*;

fn level(name: &str) -> Editor {
    let root = crate::test_support::game_data_root();
    let resources = crate::resources::Resources::load(root).unwrap();
    Editor::new(crate::machine::collection_objects(&resources, name).unwrap())
}

#[test]
fn both_seesaw_ropes_survive_drag_flip_save_and_run() {
    let source = level("L10.LEV");
    let seesaw = level("L5.LEV")
        .objects
        .into_iter()
        .find(|o| o.definition == Def::Seesaw)
        .unwrap();
    let bucket = source
        .objects
        .iter()
        .find(|o| o.definition == Def::Bucket)
        .unwrap()
        .clone();
    let rope = source
        .objects
        .iter()
        .find(|o| o.definition == Def::Rope)
        .unwrap()
        .clone();
    let pulley = source
        .objects
        .iter()
        .find(|o| o.definition == Def::Pulley)
        .unwrap()
        .clone();
    let mut editor = Editor::new(vec![
        seesaw,
        bucket.clone(),
        bucket,
        rope.clone(),
        rope,
        pulley,
    ]);
    for o in &mut editor.objects {
        o.linked_objects.fill(None);
        o.pulley_connection = None;
        o.update_flags &= !0x8000;
    }
    for (i, (x, y)) in [(200, 180), (80, 250), (380, 250), (0, 0), (0, 0), (100, 80)]
        .into_iter()
        .enumerate()
    {
        move_to(&mut editor.objects[i], x, y);
        if !matches!(i, 3 | 4) {
            editor.place(i);
        }
    }
    editor.wire_rope_route(3, &[(1, 0), (5, 0), (0, 0)]);
    editor.wire_rope_route(4, &[(0, 1), (2, 0)]);
    let index = 0;
    let original = editor.objects[index].clone();
    let links = original.linked_objects;
    let ropes: Vec<_> = editor
        .objects
        .iter()
        .enumerate()
        .filter_map(|(i, o)| {
            (o.definition == Def::Rope
                && o.ropes[0]
                    .as_ref()
                    .is_some_and(|r| r.endpoints.contains(&Some(index))))
            .then_some(i)
        })
        .collect();
    assert_eq!(ropes.len(), 2);
    editor.pick_up(index, (original.x + 16, original.y + 16));
    editor.flip(index, 1); // the Z key calls this same path while carrying
    let mut dropped = false;
    for dx in [16, -16, 32, -32, 48, -48, 64, -64] {
        editor.move_held_part(index, original.x + dx, original.y - 48);
        if editor.can_place() {
            editor.click((0, 0));
            dropped = true;
            break;
        }
    }
    assert!(dropped);
    assert_eq!(editor.objects[index].linked_objects, links);
    assert_ne!(editor.objects[index].rope_anchors, original.rope_anchors);
    for &r in &ropes {
        assert_ne!(editor.objects[r].list, ObjectList::Inventory);
    }
    let machine = editor.machine().unwrap();
    for &r in &ropes {
        assert!(machine[r].ropes[0].as_ref().unwrap().rest_length > 0.0);
    }
    let bytes = crate::machine::write_collection_with_bin(&machine, 67, 272, 1001);
    let (_, reopened) = crate::machine::parse_collection(&bytes, false).unwrap();
    let seesaw = reopened
        .iter()
        .find(|o| {
            o.definition == Def::Seesaw && o.saved_position == editor.objects[index].saved_position
        })
        .unwrap();
    assert!(seesaw.linked_objects[..4].iter().all(Option::is_some));
    assert_eq!(seesaw.rope_anchors, editor.objects[index].rope_anchors);
    let mut sim = crate::simulation::MachineSimulation::from_objects(
        reopened,
        crate::forces::initialize(67, 272),
    );
    for _ in 0..30 {
        sim.step().unwrap();
    }
}

#[test]
fn wood_resize_stops_at_last_clear_grid_extent_on_every_side() {
    for handle in [
        Handle::ResizeRight,
        Handle::ResizeLeft,
        Handle::ResizeBottom,
        Handle::ResizeTop,
    ] {
        let mut editor = level("L20.LEV");
        let wood = editor
            .objects
            .iter()
            .find(|o| o.definition == Def::WoodWall)
            .unwrap()
            .clone();
        editor.objects = vec![wood.clone(), wood];
        for o in &mut editor.objects {
            o.flags = 8;
            o.list = ObjectList::Fixed;
        }
        editor.apply_size(0, 104, 104, 32, 32);
        let (bx, by, cursor, expected) = match handle {
            Handle::ResizeRight => (184, 104, (240, 112), (104, 104, 80, 32)),
            Handle::ResizeLeft => (24, 104, (8, 112), (56, 104, 80, 32)),
            Handle::ResizeBottom => (104, 184, (112, 240), (104, 104, 32, 80)),
            Handle::ResizeTop => (104, 24, (112, 8), (104, 56, 32, 80)),
            _ => unreachable!(),
        };
        editor.apply_size(1, bx, by, 32, 32);
        editor.drag_resize(Resize { index: 0, handle }, cursor);
        let o = &editor.objects[0];
        assert_eq!((o.x, o.y, o.width, o.height), expected, "{handle:?}");
        assert!(!editor.collides(0));
        assert_eq!(o.alt_size, (o.width, o.height));
    }
}

#[test]
fn generator_wheel_and_hint_follow_both_flip_directions() {
    let mut editor = level("L1.LEV");
    let mut generator = editor.objects[0].clone();
    generator.definition = Def::ElectricalOutlet;
    generator.x = 200;
    generator.y = 100;
    generator.width = 80;
    generator.height = 32;
    generator.belt_anchor = (56, 18);
    generator.belt_size = 12;
    generator.render_flags = crate::machine::RenderFlag::BeltAttachable.mask();
    generator.list = ObjectList::Fixed;
    editor.objects = vec![
        generator,
        editor
            .objects
            .iter()
            .find(|o| o.definition == Def::Belt && o.list == ObjectList::Inventory)
            .unwrap()
            .clone(),
    ];
    let anchor = |e: &Editor| {
        let (x, y) = e.objects[0].belt_origin();
        (x as i16 + 6 - VIEWPORT.0, y as i16 + 6 - VIEWPORT.1)
    };
    let original = anchor(&editor);
    assert_eq!(editor.attach_point(original).unwrap().part, 0);
    editor.flip(0, 1);
    let mirrored = anchor(&editor);
    assert_eq!(mirrored.0, 225);
    assert!(editor.attach_point(original).is_none());
    assert_eq!(editor.attach_point(mirrored).unwrap().at, mirrored);
    editor.flip(0, 1);
    assert_eq!(anchor(&editor), original);
}

#[test]
fn electrical_motor_rebuilds_its_wheel_when_flipped_and_loaded() {
    let mut editor = level("L13.LEV");
    assert!(editor.take_definition(Def::ElectricalMotor));
    let index = editor.held.as_ref().unwrap().index;
    assert_eq!(editor.objects[index].belt_anchor, (0, 13));
    editor.flip(index, 1);
    assert_eq!(editor.objects[index].belt_anchor, (37, 13));
    editor.held = None;
    editor.objects[index].list = ObjectList::Fixed;
    let at = editor
        .attach_points_on(index)
        .into_iter()
        .find(|p| p.kind == Attach::Belt)
        .unwrap()
        .at;
    assert_eq!(at.0, editor.objects[index].x - VIEWPORT.0 + 37 + 9);
    let (x, _) = editor.objects[index].belt_origin();
    assert_eq!(x, i32::from(editor.objects[index].x) + 37);
    // Saves created before the fix can contain the stale left anchor.
    editor.objects[index].belt_anchor = (0, 13);
    crate::machine::finalize_objects(&mut editor.objects).unwrap();
    assert_eq!(editor.objects[index].belt_anchor, (37, 13));
    editor.flip(index, 1);
    assert_eq!(editor.objects[index].belt_anchor, (0, 13));
}

#[test]
fn connection_preview_rejects_same_occupied_and_distant_belt_ends() {
    let mut editor = level("L1.LEV");
    let mut a = editor.objects[0].clone();
    a.list = ObjectList::Fixed;
    a.x = 100;
    a.y = 100;
    a.width = 32;
    a.height = 32;
    a.render_flags = crate::machine::RenderFlag::BeltAttachable.mask();
    a.belt_anchor = (4, 4);
    a.belt_size = 12;
    let mut b = a.clone();
    b.x = 180;
    let belt = editor
        .objects
        .iter()
        .find(|o| o.definition == Def::Belt && o.list == ObjectList::Inventory)
        .unwrap()
        .clone();
    editor.objects = vec![a, b, belt];
    editor.held = Some(Held {
        index: 2,
        carry: Carry::Belt { first: Some(0) },
    });
    let cursor = (196, 116);
    assert!(editor.connection_preview_target(cursor).is_some());
    assert!(editor.connection_preview_target((116, 116)).is_none());
    editor.objects[1]
        .render_flags
        .insert(RenderFlag::BeltConnected);
    assert!(editor.connection_preview_target(cursor).is_none());
    editor.objects[1]
        .render_flags
        .remove(crate::machine::RenderFlag::BeltConnected);
    editor.objects[1].x = 260;
    assert!(editor.connection_preview_target((276, 116)).is_none());
    editor.objects[1].x = 180;
    editor.click(cursor);
    assert!(editor.held.is_none());
    assert_eq!(editor.objects[2].belt, Some([Some(0), Some(1)]));
}

#[test]
fn bin_groups_follow_native_order_and_counts() {
    let editor = level("L1.LEV");
    let groups: Vec<_> = editor
        .bin_groups()
        .iter()
        .map(|g| (g.definition, g.count))
        .collect();
    // Keys: Belt 10, CatPoweredDrive 37, Incline 44.
    assert_eq!(
        groups,
        vec![(Def::Belt, 3), (Def::CatPoweredDrive, 3), (Def::Incline, 3)]
    );
}

#[test]
fn carried_part_snaps_to_grid_and_places_only_when_clear() {
    let mut editor = level("L1.LEV");
    editor.take_from_bin(2);
    let index = editor.held.as_ref().unwrap().index;
    assert_eq!(editor.objects[index].definition, Def::Incline);
    editor.hover((100, 101));
    // ((100 & !15) - 8, (101 & !15) - 8).
    assert_eq!((editor.objects[index].x, editor.objects[index].y), (88, 88));
    assert_eq!(editor.bin_groups()[2].count, 2);
    // A brick wall spans x 72..312 at y 328: dropping onto it is refused.
    editor.hover((100, 330));
    assert!(!editor.can_place());
    editor.click((100, 330));
    assert!(editor.held.is_some());
    editor.hover((100, 101));
    editor.click((100, 101));
    assert!(editor.held.is_none());
    assert_eq!(editor.objects[index].list, ObjectList::Fixed);
    assert_eq!(editor.objects[index].update_flags & 0x3800, 0x2000);
    assert_eq!(editor.objects[index].saved_position, (88, 88));
}

#[test]
fn fixed_parts_collide_by_saved_box_not_outline() {
    // Puzzle 5: the seesaw's 36-pixel extent overhangs its 80x32 box.
    // On the plank at y 136 the native box test (1000:3e23) allows it.
    let mut editor = level("L5.LEV");
    let slot = editor
        .visible_bin()
        .iter()
        .position(|(_, g)| g.definition == Def::Seesaw)
        .unwrap();
    editor.take_from_bin(slot);
    let index = editor.held.as_ref().unwrap().index;
    editor.hover((80, 112));
    assert_eq!(
        (editor.objects[index].x, editor.objects[index].y),
        (72, 104)
    );
    assert!(editor.can_place());
    // One row lower its box overlaps the plank's.
    editor.hover((80, 128));
    assert!(!editor.can_place());
}

#[test]
fn belt_links_two_belt_ends_within_reach() {
    let mut editor = level("L1.LEV");
    // Place a mouse cage left of the lower conveyor (56,280, 96x16).
    editor.take_from_bin(1);
    let cage = editor.held.as_ref().unwrap().index;
    editor.hover((16, 300));
    editor.click((16, 300));
    assert!(editor.held.is_none(), "cage placement collided");
    editor.take_from_bin(0);
    let belt = editor.held.as_ref().unwrap().index;
    let cage_screen = (
        editor.objects[cage].x + 8 + 10,
        editor.objects[cage].y + 8 + 10,
    );
    editor.click(cage_screen);
    let conveyor = 8;
    let conveyor_screen = (56 + 8 + 20, 280 + 8 + 8);
    editor.click(conveyor_screen);
    assert!(editor.held.is_none(), "belt did not connect");
    assert_eq!(
        editor.objects[belt].belt,
        Some([Some(cage), Some(conveyor)])
    );
    assert_eq!(editor.objects[belt].list, ObjectList::Fixed);
    // Moving beyond the saved belt span warns red and breaking the span
    // stows the belt after the dragged part is placed.
    editor.objects[belt].update_flags |= 0x8000;
    let before_segments = editor.objects[cage].belt_origin();
    editor.pick_up(cage, cage_screen);
    assert_eq!(editor.objects[belt].list, ObjectList::Fixed);
    editor.hover((cage_screen.0, cage_screen.1 - 16));
    assert!(
        editor.can_place(),
        "an overstretched belt must not block placement"
    );
    assert_eq!(
        editor.held_connection_states(),
        vec![(belt, None, false)],
        "an overstretched belt should be marked red"
    );
    assert_ne!(editor.objects[cage].belt_origin(), before_segments);
    editor.click((cage_screen.0, cage_screen.1 - 16));
    assert!(editor.held.is_none());
    assert_eq!(editor.objects[belt].list, ObjectList::Inventory);
    assert_eq!(editor.objects[belt].belt, Some([None, None]));
    assert!(
        !editor.objects[conveyor]
            .render_flags
            .contains(RenderFlag::BeltConnected)
    );
    // Returning the cage to the bin leaves the already broken belt gone.
    editor.return_to_bin(cage);
    assert_eq!(editor.objects[belt].list, ObjectList::Inventory);
    assert!(
        !editor.objects[conveyor]
            .render_flags
            .contains(RenderFlag::BeltConnected)
    );
}

#[test]
fn clicking_a_free_belt_anchor_takes_a_belt_and_starts_it_there() {
    let mut editor = level("L1.LEV");
    editor.take_from_bin(1);
    let cage = editor.held.as_ref().unwrap().index;
    editor.hover((16, 300));
    editor.click((16, 300));
    let o = editor.objects[cage].clone();
    // The middle of the belt wheel, not the anchor's corner.
    let half = o.belt_size / 2;
    let anchor = (
        o.x + 8 + i16::from(o.belt_anchor.0) + half,
        o.y + 8 + i16::from(o.belt_anchor.1) + half,
    );
    let belts = editor.bin_count(Def::Belt);
    assert_eq!(belts, 3);
    // Only near the anchor itself.
    assert_eq!(editor.attach_point((anchor.0 + 10, anchor.1)), None);
    let point = editor.attach_point((anchor.0 + 2, anchor.1 - 1)).unwrap();
    assert_eq!((point.part, point.kind), (cage, Attach::Belt));
    // Clicking it attaches rather than picking the cage up.
    editor.click((anchor.0 + 2, anchor.1 - 1));
    let held = editor.held.clone().unwrap();
    assert_eq!(held.carry, Carry::Belt { first: Some(cage) });
    assert_eq!(editor.objects[held.index].definition, Def::Belt);
    assert_eq!(editor.bin_count(Def::Belt), belts - 1);
    editor.click((56 + 8 + 20, 280 + 8 + 8));
    assert!(editor.held.is_none(), "belt did not connect");
    let conveyor = 8;
    // Belted now: the same square takes the cage's end off again, and
    // the belt is held with its conveyor end still attached.
    let point = editor.attach_point(anchor).unwrap();
    assert_eq!(point.connection, Some(held.index));
    editor.click(anchor);
    assert_eq!(
        editor.held.clone().unwrap(),
        Held {
            index: held.index,
            carry: Carry::Belt {
                first: Some(conveyor)
            }
        }
    );
    assert!(
        !editor.objects[cage]
            .render_flags
            .contains(RenderFlag::BeltConnected)
    );
    assert!(
        !editor.objects[conveyor]
            .render_flags
            .contains(RenderFlag::BeltConnected)
    );
    assert_eq!(
        editor.bin_count(Def::Belt),
        belts - 1,
        "the belt is in hand, not the bin"
    );
    // Clicking the cage again puts it back.
    editor.click(anchor);
    assert!(editor.held.is_none());
    assert_eq!(
        editor.objects[held.index].belt,
        Some([Some(conveyor), Some(cage)])
    );
}

fn belt_centre(e: &Editor, part: usize) -> (i16, i16) {
    let o = &e.objects[part];
    let half = o.belt_size / 2;
    (
        o.x + 8 + i16::from(o.belt_anchor.0) + half,
        o.y + 8 + i16::from(o.belt_anchor.1) + half,
    )
}

#[test]
fn the_puzzles_own_belts_cannot_be_detached() {
    let mut editor = level("L8.LEV");
    // The level belts the bottom windmill to the gear and locks the belt.
    let belt = editor
        .objects
        .iter()
        .position(|o| o.definition == Def::Belt && o.list != ObjectList::Inventory)
        .unwrap();
    assert!(!editor.unlocked(belt));
    let [Some(a), Some(b)] = editor.objects[belt].belt.unwrap() else {
        panic!()
    };
    for end in [a, b] {
        let at = belt_centre(&editor, end);
        assert_eq!(editor.attach_point(at), None, "locked belt end offered");
        editor.click(at);
        assert_eq!(editor.objects[belt].belt, Some([Some(a), Some(b)]));
    }
}

#[test]
fn a_free_anchor_is_offered_before_a_partner_is_within_reach() {
    let mut editor = level("L1.LEV");
    editor.take_from_bin(1);
    let cage = editor.held.as_ref().unwrap().index;
    // Far from every free belt end (the conveyors are at the bottom left).
    editor.hover((520, 40));
    editor.click((520, 40));
    assert!(editor.held.is_none(), "cage placement collided");
    let point = editor.attach_point(belt_centre(&editor, cage)).unwrap();
    assert_eq!(point.connection, None);
    assert!(editor.attach(point));
    assert_eq!(
        editor.held.as_ref().unwrap().carry,
        Carry::Belt { first: Some(cage) }
    );
    editor.cancel();
    // Beside the lower conveyor it is still offered.
    let (x, y) = (editor.objects[cage].x + 8, editor.objects[cage].y + 8);
    editor.click((x + 4, y + 4));
    editor.hover((16, 300));
    editor.click((16, 300));
    assert!(editor.held.is_none());
    assert!(editor.attach_point(belt_centre(&editor, cage)).is_some());
}

#[test]
fn a_fan_dropped_beside_an_outlet_plugs_in_and_unplugs_when_moved() {
    let mut editor = level("L37.LEV");
    let outlet = editor
        .objects
        .iter()
        .position(|o| o.definition == Def::PowerDistributor)
        .unwrap();
    assert_eq!(editor.objects[outlet].linked_objects[4..], [None, None]);
    let switch = editor.objects[outlet].state & 4;
    assert!(editor.take_definition(Def::Fan));
    let fan = editor.held.as_ref().unwrap().index;
    // Try spots around the outlet until the fan fits and plugs in.
    let o = editor.objects[outlet].clone();
    let (ox, oy) = (o.x + 8, o.y + 8);
    let spots = (-80..=80)
        .step_by(16)
        .flat_map(|dx| (-64..=64).step_by(16).map(move |dy| (ox + dx, oy + dy)));
    let mut placed = false;
    for (x, y) in spots {
        editor.hover((x, y));
        if editor.can_place() {
            let preview = editor.placement_preview();
            let preview_outlet = preview[fan].linked_objects[4];
            if preview_outlet == Some(outlet) {
                assert_eq!(preview[outlet].state, switch | 1);
                assert_eq!(preview[outlet].linked_objects[4], Some(fan));
                assert_eq!(editor.objects[outlet].linked_objects[4..], [None, None]);
                assert_eq!(editor.objects[outlet].state, switch);
                assert_eq!(editor.objects[fan].linked_objects[4], None);
            }
            editor.click((x, y));
            assert_eq!(editor.objects[fan].linked_objects[4], preview_outlet);
            if editor.objects[fan].linked_objects[4] == Some(outlet) {
                placed = true;
                break;
            }
            let (fx, fy) = (editor.objects[fan].x + 8, editor.objects[fan].y + 8);
            editor.click((fx + 2, fy + 2));
        }
    }
    assert!(placed, "no spot beside the outlet plugged the fan in");
    let o = &editor.objects[outlet];
    assert_eq!(o.linked_objects[4], Some(fan));
    // The plug shows in socket one: state bit 0, saved with it.
    assert_eq!((o.state, o.saved_state), (switch | 1, switch | 1));
    // The machine the run starts from keeps the link.
    assert_eq!(
        editor.machine().unwrap()[outlet].linked_objects[4],
        Some(fan)
    );
    // Picking the fan up pulls the plug.
    let (fx, fy) = (editor.objects[fan].x + 8, editor.objects[fan].y + 8);
    editor.click((fx + 2, fy + 2));
    assert_eq!(editor.held.as_ref().map(|h| h.index), Some(fan));
    assert_eq!(editor.objects[outlet].linked_objects[4], None);
    assert_eq!(editor.objects[outlet].state, switch);
    assert_eq!(editor.objects[fan].linked_objects[4], None);
}

#[test]
fn rope_anchors_offer_a_rope_but_pulleys_do_not() {
    let mut editor = level("L10.LEV");
    let slot = |e: &Editor, d| {
        e.bin_groups()
            .iter()
            .position(|g| g.definition == d)
            .unwrap()
    };
    editor.take_from_bin(slot(&editor, Def::Gun));
    let gun = editor.held.as_ref().unwrap().index;
    editor.hover((296 + 8, 216 + 8));
    editor.click((296 + 8, 216 + 8));
    let o = editor.objects[gun].clone();
    let anchor = (
        o.x + 8 + i16::from(o.rope_anchors[0].0),
        o.y + 8 + i16::from(o.rope_anchors[0].1),
    );
    assert_eq!(
        editor.attach_point(anchor).map(|p| (p.part, p.kind)),
        Some((gun, Attach::Rope(0)))
    );
    let pulley = &editor.objects[15];
    assert_eq!(pulley.definition, Def::Pulley);
    let at = (
        pulley.x + 8 + i16::from(pulley.rope_anchors[0].0),
        pulley.y + 8 + i16::from(pulley.rope_anchors[0].1),
    );
    assert!(editor.attach_point(at).is_none_or(|p| p.part != 15));
    editor.click(anchor);
    assert_eq!(
        editor.held.as_ref().unwrap().carry,
        Carry::Rope {
            route: vec![(gun, 0)]
        }
    );
}

#[test]
fn detaching_a_rope_end_keeps_its_far_end_and_pulleys() {
    let mut editor = level("L10.LEV");
    let slot = |e: &Editor, d| {
        e.bin_groups()
            .iter()
            .position(|g| g.definition == d)
            .unwrap()
    };
    editor.take_from_bin(slot(&editor, Def::Gun));
    let gun = editor.held.as_ref().unwrap().index;
    editor.hover((296 + 8, 216 + 8));
    editor.click((296 + 8, 216 + 8));
    editor.take_from_bin(slot(&editor, Def::Bucket));
    let bucket = editor.held.as_ref().unwrap().index;
    editor.hover((200 + 8, 232 + 8));
    editor.click((200 + 8, 232 + 8));
    editor.take_from_bin(slot(&editor, Def::Rope));
    let rope = editor.held.as_ref().unwrap().index;
    let centre = |e: &Editor, i: usize| {
        let o = &e.objects[i];
        (o.x + 8 + o.width as i16 / 2, o.y + 8 + o.height as i16 / 2)
    };
    let pulley = 15;
    for part in [gun, pulley, bucket] {
        let c = centre(&editor, part);
        editor.click(c);
    }
    assert!(editor.held.is_none());
    let o = &editor.objects[bucket];
    let bucket_end = (
        o.x + 8 + i16::from(o.rope_anchors[0].0),
        o.y + 8 + i16::from(o.rope_anchors[0].1),
    );
    let point = editor.attach_point(bucket_end).unwrap();
    assert_eq!((point.part, point.connection), (bucket, Some(rope)));
    editor.click(bucket_end);
    // Held from the gun over the pulley; the bucket and pulley are free.
    assert_eq!(
        editor.held.as_ref().unwrap().carry,
        Carry::Rope {
            route: vec![(gun, 0), (pulley, 0)]
        }
    );
    assert_eq!(editor.objects[bucket].linked_objects[0], None);
    assert_eq!(editor.objects[pulley].linked_objects[..2], [None, None]);
    assert_eq!(editor.objects[rope].list, ObjectList::Inventory);
    // Tying it back to the bucket rebuilds the same rope.
    let c = centre(&editor, bucket);
    editor.click(c);
    assert!(editor.held.is_none());
    let connection = editor.objects[rope].ropes[0].clone().unwrap();
    assert_eq!(connection.endpoints, [Some(gun), Some(bucket)]);
    assert_eq!(
        editor.objects[pulley].linked_objects[..2],
        [Some(bucket), Some(gun)]
    );
    // Taking the gun's end instead reverses the route.
    let o = &editor.objects[gun];
    let gun_end = (
        o.x + 8 + i16::from(o.rope_anchors[0].0),
        o.y + 8 + i16::from(o.rope_anchors[0].1),
    );
    editor.click(gun_end);
    assert_eq!(
        editor.held.as_ref().unwrap().carry,
        Carry::Rope {
            route: vec![(bucket, 0), (pulley, 0)]
        }
    );
}

#[test]
fn take_definition_puts_another_from_the_bin_in_hand() {
    let mut editor = level("L1.LEV");
    editor.take_from_bin(2);
    editor.hover((100, 101));
    editor.click((100, 101));
    assert!(editor.held.is_none());
    assert!(editor.take_definition(Def::Incline));
    assert_eq!(editor.bin_count(Def::Incline), 1);
    // Not while something is already in hand.
    assert!(!editor.take_definition(Def::Incline));
    editor.cancel();
    assert!(editor.take_definition(Def::Incline));
    editor.hover((200, 101));
    editor.click((200, 101));
    assert!(editor.take_definition(Def::Incline));
    editor.hover((300, 101));
    editor.click((300, 101));
    assert_eq!(editor.bin_count(Def::Incline), 0);
    assert!(!editor.take_definition(Def::Incline));
}

#[test]
fn pulley_orientation_matches_original_saved_machines() {
    for name in ["TITLE.GKC", "L10.LEV", "L19.LEV"] {
        let mut editor = level(name);
        let original = editor.objects.clone();
        crate::machine::refresh_pulleys(&mut editor.objects, None);
        let mut connected = 0;
        for (index, o) in editor.objects.iter().enumerate() {
            if o.definition == Def::Pulley && o.linked_objects[..2].iter().all(Option::is_some) {
                connected += 1;
                assert_eq!(
                    o.state, original[index].state,
                    "native pulley frame: {name} part {index}"
                );
                assert_eq!(
                    o.rope_anchors, original[index].rope_anchors,
                    "native pulley tangents: {name} part {index}"
                );
            }
        }
        assert!(connected > 0);
    }
}

#[test]
fn rope_preview_updates_pulley_quadrant_and_uses_both_tangents() {
    let mut editor = level("L10.LEV");
    // A synthetic route makes the four native quadrants easy to inspect.
    let pulley = 15;
    let first = 0;
    editor.objects[first].definition = Def::RopeEnd;
    editor.objects[first].rope_anchors = [(0, 0); 2];
    editor.objects[pulley].x = 200;
    editor.objects[pulley].y = 200;
    editor.held = Some(Held {
        index: 1,
        carry: Carry::Rope {
            route: vec![(first, 0), (pulley, 0)],
        },
    });
    let original = editor.objects[pulley].rope_anchors;
    for (previous, cursor, state, anchors) in [
        ((240, 240), (160, 240), 2, [(15, 6), (0, 6)]),
        ((160, 240), (160, 160), 3, [(10, 15), (10, 0)]),
        ((160, 160), (240, 160), 0, [(0, 10), (15, 10)]),
        ((240, 160), (240, 240), 1, [(6, 0), (6, 15)]),
    ] {
        editor.objects[first].x = previous.0;
        editor.objects[first].y = previous.1;
        let mut objects = editor.objects.clone();
        editor.preview_rope(&mut objects, (cursor.0 - VIEWPORT.0, cursor.1 - VIEWPORT.1));
        assert_eq!(objects[pulley].state, state);
        assert_eq!(objects[pulley].rope_anchors, anchors);
        // Native 24de:1dcf and 1dff:2140 use anchor 0 for the
        // incoming rope, anchor 1 for the outgoing cursor guide.
        // Each segment must stay on its own side of the pulley face.
        let incoming = crate::machine::rope_point(&objects[pulley], 0);
        let outgoing = crate::machine::rope_point(&objects[pulley], 1);
        let axis = if state % 2 == 0 { 0 } else { 1 };
        let coordinates = |p: (i32, i32)| if axis == 0 { p.0 } else { p.1 };
        assert_eq!(
            (coordinates(incoming) - 208).signum(),
            (coordinates((previous.0 as i32, previous.1 as i32)) - 208).signum()
        );
        assert_eq!(
            (coordinates(outgoing) - 208).signum(),
            (coordinates((cursor.0 as i32, cursor.1 as i32)) - 208).signum()
        );
        assert_eq!(
            editor.objects[pulley].rope_anchors, original,
            "preview must not alter the machine"
        );
    }
}

#[test]
fn rope_runs_from_an_end_over_a_pulley_to_the_far_end() {
    let mut editor = level("L10.LEV");
    let slot = |e: &Editor, d| {
        e.bin_groups()
            .iter()
            .position(|g| g.definition == d)
            .unwrap()
    };
    // A gun on the floor right of the free pulley (257,215), a bucket
    // hanging left of it.
    editor.take_from_bin(slot(&editor, Def::Gun));
    let gun = editor.held.as_ref().unwrap().index;
    editor.hover((296 + 8, 216 + 8));
    editor.click((296 + 8, 216 + 8));
    assert!(editor.held.is_none(), "gun collided");
    editor.take_from_bin(slot(&editor, Def::Bucket));
    let bucket = editor.held.as_ref().unwrap().index;
    editor.hover((200 + 8, 232 + 8));
    editor.click((200 + 8, 232 + 8));
    assert!(editor.held.is_none(), "bucket collided");
    editor.take_from_bin(slot(&editor, Def::Rope));
    let rope = editor.held.as_ref().unwrap().index;
    let centre = |e: &Editor, i: usize| {
        let o = &e.objects[i];
        (o.x + 8 + o.width as i16 / 2, o.y + 8 + o.height as i16 / 2)
    };
    let pulley = 15;
    for part in [gun, pulley, bucket] {
        let c = centre(&editor, part);
        editor.click(c);
    }
    assert!(editor.held.is_none(), "rope did not complete");
    let connection = editor.objects[rope].ropes[0].clone().unwrap();
    assert_eq!(connection.endpoints, [Some(gun), Some(bucket)]);
    assert_eq!(editor.objects[gun].linked_objects[0], Some(pulley));
    assert_eq!(
        editor.objects[pulley].linked_objects[..2],
        [Some(bucket), Some(gun)]
    );
    assert_eq!(editor.objects[bucket].linked_objects[0], Some(pulley));
    let machine = editor.machine().unwrap();
    let spans =
        crate::machine::rope_spans(&machine, machine[rope].ropes[0].as_ref().unwrap()).unwrap();
    assert_eq!(
        spans[0].to_slot, 0,
        "24de:1dcf uses incoming pulley coordinate +0x14"
    );
    assert_eq!(
        spans[1].from_slot, 1,
        "24de:1dcf uses outgoing pulley coordinate +0x18"
    );
    assert_eq!(
        spans[0].end,
        crate::machine::rope_point(&machine[pulley], 0)
    );
    assert_eq!(
        spans[1].start,
        crate::machine::rope_point(&machine[pulley], 1)
    );
    assert!(machine[rope].ropes[0].as_ref().unwrap().rest_length > 0.0);
    // Moving the pulley preserves the rope and both neighboring links.
    editor.pick_up(pulley, centre(&editor, pulley));
    let cursor = (
        editor.objects[pulley].x + 8 + 8,
        editor.objects[pulley].y - 16 + 8 + 8,
    );
    editor.hover(cursor);
    assert_eq!(
        editor.objects[pulley].linked_objects[..2],
        [Some(bucket), Some(gun)]
    );
    assert_eq!(editor.objects[rope].list, ObjectList::Fixed);
    assert!(editor.can_place());
    editor.click(cursor);
    assert!(editor.held.is_none());
    assert_eq!(
        editor.objects[rope].ropes[0].as_ref().unwrap().endpoints,
        [Some(gun), Some(bucket)]
    );
    // Moving either endpoint also preserves the route; cancel returns it
    // to the bin and then unwires the connection, without dangling links.
    editor.pick_up(gun, centre(&editor, gun));
    editor.hover((320, 220));
    assert_eq!(editor.objects[gun].linked_objects[0], Some(pulley));
    assert_eq!(editor.objects[rope].list, ObjectList::Fixed);
    editor.cancel();
    assert_eq!(editor.objects[rope].list, ObjectList::Inventory);
    assert_eq!(editor.objects[pulley].linked_objects[..2], [None, None]);
    assert_eq!(editor.objects[bucket].linked_objects[0], None);
}

#[test]
fn handles_outside_the_body_stay_reachable_and_flip_the_cage() {
    let mut editor = level("L1.LEV");
    editor.take_from_bin(1);
    let cage = editor.held.as_ref().unwrap().index;
    // Beside the lower conveyor (56,280): the flip handle overlaps it.
    editor.hover((16, 300));
    editor.click((16, 300));
    assert!(editor.held.is_none());
    let o = editor.objects[cage].clone();
    let (left, bottom) = (o.x + 8, o.y + 8 + o.height as i16);
    editor.hover((left + 10, bottom - 10));
    assert_eq!(editor.hovered, Some(cage));
    // Moving out to the bottom-left flip square keeps the cage selected.
    let flip = (left - 5, bottom + 5);
    editor.hover(flip);
    assert_eq!(
        editor.handle_under(flip),
        Some((cage, Some(Handle::FlipHorizontal)))
    );
    editor.click(flip);
    assert_ne!(
        editor.objects[cage]
            .render_flags
            .contains(RenderFlag::FlipHorizontal),
        o.render_flags.contains(RenderFlag::FlipHorizontal)
    );
    assert_eq!(editor.objects[cage].belt_anchor.0, 3);
}

#[test]
fn handles_survive_overshoot_and_the_safe_triangle() {
    let mut editor = level("L1.LEV");
    editor.take_from_bin(2);
    let incline = editor.held.as_ref().unwrap().index;
    editor.hover((200, 150));
    editor.click((200, 150));
    let o = editor.objects[incline].clone();
    let (x0, y0, w, h) = (o.x + 8, o.y + 8, o.width as i16, o.height as i16);
    // Every drawn icon is clickable, reached in one jump from the body.
    for icon in editor.handle_icons(incline) {
        let (ax, ay, bx, by) = icon.opaque();
        let centre = ((ax + bx) / 2, (ay + by) / 2);
        editor.hover((x0 + w / 2, y0 + h / 2));
        editor.hover(centre);
        assert_eq!(
            editor.handle_under(centre),
            Some((incline, Some(icon.handle)))
        );
    }
    // Overshooting the bottom-right flip by a few pixels keeps the part,
    // so coming back lands on the handle.
    editor.hover((x0 + w / 2, y0 + h / 2));
    let past = (x0 + w + 18, y0 + h + 18);
    editor.hover(past);
    assert_eq!(editor.hovered, Some(incline));
    editor.hover((x0 + w + 5, y0 + h + 6));
    assert_eq!(
        editor.handle_under((x0 + w + 5, y0 + h + 6)),
        Some((incline, Some(Handle::FlipVertical)))
    );
    // From beyond the right flip, heading under the part for the left
    // flip leaves the margins but stays in the safe triangle; resting
    // there lets the selection lapse.
    editor.hover((x0 + w + 20, y0 + h + 20));
    let between = (x0 + w / 2, y0 + h + 22);
    editor.hover(between);
    assert!(!editor.near_handles(incline, between));
    assert_eq!(editor.hovered, Some(incline));
    for _ in 0..=SAFE_TRIANGLE_FRAMES {
        editor.hover(between);
    }
    assert_eq!(editor.hovered, None);
}

#[test]
fn flipping_a_light_moves_its_pull_cord_and_a_monkey_mirrors_its_anchors() {
    let mut editor = level("L40.LEV");
    let light = editor
        .objects
        .iter()
        .position(|o| o.definition == Def::Light)
        .unwrap();
    assert_eq!(
        (
            editor.objects[light].state,
            editor.objects[light].rope_anchors[0]
        ),
        (0, (21, 51))
    );
    editor.flip(light, 2);
    assert_eq!(
        (
            editor.objects[light].state,
            editor.objects[light].saved_state
        ),
        (2, 2)
    );
    assert_eq!(editor.objects[light].rope_anchors[0], (20, 25));
    editor.flip(light, 2);
    assert_eq!(editor.objects[light].rope_anchors[0], (21, 51));

    let mut editor = level("L43.LEV");
    let monkey = editor
        .objects
        .iter()
        .position(|o| o.definition == Def::Monkey)
        .unwrap();
    editor.flip(monkey, 1);
    let o = &editor.objects[monkey];
    // Matches the saved flipped monkeys in L46/L57/L80.
    assert_eq!((o.rope_anchors[0], o.belt_anchor), ((16, 45), (36, 60)));
    editor.flip(monkey, 1);
    let o = &editor.objects[monkey];
    assert_eq!((o.rope_anchors[0], o.belt_anchor), ((75, 45), (47, 60)));
}

#[test]
fn flip_and_resize_follow_definition_callbacks() {
    let mut editor = level("L1.LEV");
    editor.take_from_bin(2);
    let incline = editor.held.as_ref().unwrap().index;
    editor.hover((200, 150));
    editor.click((200, 150));
    let flags = editor.objects[incline].render_flags;
    editor.flip(incline, 1);
    assert_ne!(
        editor.objects[incline]
            .render_flags
            .contains(RenderFlag::FlipHorizontal),
        flags.contains(RenderFlag::FlipHorizontal)
    );
    editor.resizing = Some(Resize {
        index: incline,
        handle: Handle::ResizeRight,
    });
    let o = &editor.objects[incline];
    let right_screen = o.x + 8 + o.width as i16;
    editor.hover((right_screen + 4, 160));
    assert_eq!(
        (editor.objects[incline].width, editor.objects[incline].state),
        (48, 2)
    );
}
