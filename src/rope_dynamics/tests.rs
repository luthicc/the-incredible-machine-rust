use super::*;
use crate::motion::Motion;

fn fixture(routed: bool) -> (Vec<Object>, [PartForce; 58]) {
    let root = crate::test_support::game_data_root();
    let resources = crate::resources::Resources::load(root).unwrap();
    let template = crate::machine::collection_objects(&resources, "L10.LEV").unwrap()[0].clone();
    let mut objects = vec![template; if routed { 5 } else { 3 }];
    for (i, p) in objects.iter_mut().enumerate() {
        p.definition = if i == 0 {
            Def::Rope
        } else if i >= 3 {
            Def::Pulley
        } else {
            Def::Baseball
        };
        p.list = if i == 1 || i == 2 {
            ObjectList::Moving
        } else {
            ObjectList::Fixed
        };
        p.update_flags = if p.list == ObjectList::Moving {
            0x1000
        } else {
            0x4000
        };
        p.render_flags = crate::machine::RenderFlags::EMPTY;
        p.state = 0;
        p.rope_anchors = [(0, 0); 2];
        p.outline = None;
        p.outline_angles.clear();
        p.contact = None;
        p.ropes = [None, None];
        p.linked_objects = [None; 6];
        p.effective_mass = 10;
        p.x = if i == 2 || i == 4 { 100 } else { 0 };
        p.y = if routed && (i == 1 || i == 2) { 50 } else { 0 };
        p.previous_position = (p.x, p.y);
        p.motion = Motion::at_pixels(p.x, p.y);
    }
    let rope = RopeConnection {
        endpoints: [Some(1), Some(2)],
        attachment_slots: [0, 0],
        rest_length: 1.0,
        slack_count: 0,
    };
    objects[0].ropes[0] = Some(rope);
    objects[1].linked_objects[0] = Some(if routed { 3 } else { 2 });
    objects[2].linked_objects[0] = Some(if routed { 4 } else { 1 });
    if routed {
        objects[3].linked_objects[..2].copy_from_slice(&[Some(4), Some(1)]);
        objects[4].linked_objects[..2].copy_from_slice(&[Some(2), Some(3)]);
    }
    objects[0].activation_timer = if routed { 50 } else { 100 };
    objects[0].callback_timer = objects[0].activation_timer;
    (
        objects,
        [PartForce {
            acceleration: 0,
            speed_limit: i16::MAX,
        }; 58],
    )
}
fn respond(objects: &mut [Object], side: usize, forces: &[PartForce; 58]) -> bool {
    process_rope_contact_response(objects, 0, side, forces, &mut ImpulsePriority::default())
}

#[test]
fn slack_does_not_pull_and_tension_uses_integer_end_allowance() {
    let (mut objects, forces) = fixture(false);
    objects[1].list = ObjectList::Fixed;
    objects[2].x = 90;
    objects[2].motion.velocity_x = 1000;
    let before = objects.clone();
    assert!(!respond(&mut objects, 1, &forces));
    assert_eq!(objects, before);
    objects[2].x = 101;
    objects[2].motion.position_x = (101 << 9) + 511;
    assert!(respond(&mut objects, 1, &forces));
    assert_eq!((objects[2].x, objects[2].y), (100, 0));
    assert_eq!(objects[2].motion, Motion::at_pixels(100, 0));
}

#[test]
fn slack_rope_rescan_preserves_contact_and_response_flags() {
    let (mut objects, forces) = fixture(false);
    let prior = crate::machine::ContactRecord {
        target: 1,
        outline_edge: 0,
        angle: 0x8000,
        flag_a: true,
        flag_b: true,
    };
    objects[1].list = ObjectList::Fixed;
    objects[2].contact = Some(prior);
    for flags in [2, 3, 4, 8] {
        objects[2].update_flags = 0x1000 | flags;
        objects[2].contact = Some(prior);
        update_moving_rope(&mut objects, 2, &forces, &mut ImpulsePriority::default());
        assert_eq!(objects[2].contact, Some(prior));
        assert_eq!(objects[2].update_flags & 0xf, flags);
    }
}

#[test]
fn diagonal_projection_reconstructs_horizontal_velocity_only() {
    let (mut objects, forces) = fixture(false);
    objects[1].list = ObjectList::Fixed;
    objects[2].x = 120;
    objects[2].y = 80;
    objects[2].previous_position = (100, 60);
    objects[2].motion.velocity_y = 1111;
    assert!(respond(&mut objects, 1, &forces));
    // Octagonal length is 150; DOS truncates signed division.
    assert_eq!((objects[2].x, objects[2].y), (80, 53));
    assert_eq!(objects[2].motion.velocity_x, -20 * 512);
    assert_eq!(objects[2].motion.velocity_y, 1111);
    assert_eq!(objects[2].motion.pixels(), (80, 53));
}

#[test]
fn routed_slack_moves_between_ends_in_both_directions() {
    for side in 0..2 {
        let (mut objects, forces) = fixture(true);
        let moving = side + 1;
        let peer = 2 - side;
        objects[moving].y = 60;
        objects[peer].y = 30;
        let positions = [(objects[1].x, objects[1].y), (objects[2].x, objects[2].y)];
        assert!(!respond(&mut objects, side, &forces));
        assert_eq!(allowance(&objects, 0, side), 60);
        assert_eq!(allowance(&objects, 0, 1 - side), 40);
        assert_eq!(
            [(objects[1].x, objects[1].y), (objects[2].x, objects[2].y)],
            positions
        );
        let rope = objects[0].ropes[0].as_ref().unwrap();
        assert_eq!(
            crate::machine::rope_render_segments(&objects, &objects[0], rope)
                .unwrap()
                .len(),
            18
        );
    }
}

#[test]
fn heavier_load_pulls_lighter_load_without_equal_split() {
    for side in 0..2 {
        let (mut objects, forces) = fixture(true);
        objects[side + 1].y = 70;
        objects[side + 1].effective_mass = 100;
        objects[2 - side].effective_mass = 50;
        assert!(respond(&mut objects, side, &forces));
        // (20 * (100 - 50) + 100) / 100 = 11 pixels.
        assert_eq!(objects[side + 1].y, 61);
        assert_eq!(objects[2 - side].y, 39);
        assert_eq!(allowance(&objects, 0, side), 61);
        assert_eq!(allowance(&objects, 0, 1 - side), 39);
    }
}

#[test]
fn equal_or_lighter_load_does_not_move_heavier_peer() {
    for mass in [5, 10] {
        let (mut objects, forces) = fixture(true);
        objects[1].y = 70;
        objects[1].effective_mass = mass;
        assert!(respond(&mut objects, 0, &forces));
        assert_eq!((objects[1].y, objects[2].y), (50, 50));
        assert_eq!(
            (objects[0].activation_timer, objects[0].callback_timer),
            (50, 50)
        );
    }
}

#[test]
fn blocked_lighter_load_gives_back_unusable_rope_transfer() {
    let (mut objects, forces) = fixture(true);
    objects[1].y = 70;
    objects[1].effective_mass = 100;
    objects[2].effective_mass = 50;
    objects[2].width = 4;
    objects[2].height = 4;
    objects[2].outline = Some(vec![(0, 0), (3, 0), (3, 3), (0, 3)]);
    objects[2].outline_angles = crate::outline::edge_angles(objects[2].outline.as_ref().unwrap());
    let mut wall = objects[3].clone();
    wall.definition = Def::WoodWall;
    wall.x = 90;
    wall.y = 35;
    wall.width = 20;
    wall.height = 10;
    wall.previous_position = (wall.x, wall.y);
    wall.outline = crate::outline::vertices(&wall);
    wall.outline_angles = crate::outline::edge_angles(wall.outline.as_ref().unwrap());
    objects.push(wall);
    respond(&mut objects, 0, &forces);
    assert!(
        objects[2].y >= 45,
        "lighter load crossed its blocking wall: {}",
        objects[2].y
    );
    assert!(
        objects[1].y <= 55,
        "blocked load must not provide all 11 pixels"
    );
    assert_eq!(objects[0].activation_timer + objects[0].callback_timer, 100);
}

#[test]
fn rope_projection_is_applied_during_each_moving_update_and_resets() {
    let (mut objects, forces) = fixture(true);
    objects[1].motion.velocity_y = 10 * 512;
    objects[2].list = ObjectList::Fixed;
    let initial = objects.clone();
    let mut sim = super::super::MachineSimulation::from_objects(objects, forces);
    sim.step().unwrap();
    assert_eq!(sim.objects[1].y, 50);
    assert_eq!(sim.objects[1].motion.velocity_y, 0);
    assert_eq!(sim.objects[1].motion.pixels(), (0, 50));
    for i in [3, 4] {
        assert_eq!(sim.objects[i].rope_anchors, initial[i].rope_anchors);
        assert_eq!(sim.objects[i].state, initial[i].state);
    }
    sim.reset();
    assert_eq!(sim.objects, initial);
}

#[test]
fn loose_end_cannot_hold_back_a_direct_physical_endpoint() {
    let (mut objects, forces) = fixture(false);
    objects[1].definition = Def::RopeEnd;
    objects[2].x = 150;
    assert!(!respond(&mut objects, 1, &forces));
    assert_eq!(objects[2].x, 150);
    assert!(respond(&mut objects, 0, &forces));
    assert_eq!(objects[1].x, 50);
}

#[test]
fn cut_tail_unwraps_nearest_pulley_in_both_directions() {
    for side in 0..2 {
        let (mut objects, forces) = fixture(true);
        let loose_side = 1 - side;
        let loose = 2 - side;
        let pulley = if loose_side == 0 { 3 } else { 4 };
        let saved_links = [Some(4), Some(1)];
        objects[pulley].linked_objects[2..4].copy_from_slice(&saved_links);
        objects[loose].definition = Def::RopeEnd;
        objects[side + 1].y = 70;
        set_allowance(&mut objects, 0, loose_side, 10);
        assert!(!respond(&mut objects, side, &forces));
        assert_eq!(
            objects[side + 1].y,
            70,
            "cut tail must not pin the real part"
        );
        assert_eq!(objects[pulley].linked_objects[..2], [None, None]);
        assert_eq!(objects[pulley].linked_objects[2..4], saved_links);
        let rope = objects[0].ropes[0].as_ref().unwrap();
        let spans = rope_spans(&objects, rope).unwrap();
        assert_eq!(spans.len(), 2);
        assert!(!spans.iter().any(|s| s.from == pulley || s.to == pulley));
        assert_eq!(allowance(&objects, 0, side), 60);
    }
}

#[test]
fn loose_tail_can_run_off_an_entire_three_pulley_route() {
    for side in 0..2 {
        let (mut objects, forces) = fixture(true);
        let mut middle = objects[3].clone();
        middle.x = 50;
        middle.y = -50;
        middle.linked_objects[..2].copy_from_slice(&[Some(4), Some(3)]);
        objects.push(middle);
        objects[3].linked_objects[0] = Some(5);
        objects[4].linked_objects[1] = Some(5);
        objects[2 - side].definition = Def::RopeEnd;
        for _ in 0..3 {
            objects[side + 1].y += 200;
            assert!(!respond(&mut objects, side, &forces));
        }
        for i in 3..=5 {
            assert_eq!(objects[i].linked_objects[..2], [None, None]);
        }
        assert_eq!(objects[1].linked_objects[0], Some(2));
        assert_eq!(objects[2].linked_objects[0], Some(1));
        let rope = objects[0].ropes[0].as_ref().unwrap();
        assert_eq!(rope_spans(&objects, rope).unwrap().len(), 1);
        objects[side + 1].y += 200;
        let position = objects[side + 1].y;
        assert!(!respond(&mut objects, side, &forces));
        assert_eq!(objects[side + 1].y, position);
    }
}
