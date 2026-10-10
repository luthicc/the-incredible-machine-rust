#[cfg(test)]
pub(super) fn unit_vector(x: f64, y: f64) -> (f64, f64) {
    let length = x.hypot(y);
    if length == 0.0 {
        (0.0, 0.0)
    } else {
        (x / length, y / length)
    }
}
/// Resolve the SAT normal to an original outline edge, preserving native
/// vertex numbering. This remains an adapter, not the DOS contact builder.
#[cfg(test)]
pub(super) fn convex_contact(a: &Object, b: &Object) -> Option<(f64, f64, f64)> {
    // Native collision bounds (1000:0386) reject empty extents. The byte
    // outline's width-1 wraps for hidden, zero-width collection walls.
    if a.width == 0 || a.height == 0 || b.width == 0 || b.height == 0 {
        return None;
    }
    if b.definition == Def::Fish {
        let (w, h) = crate::spatial::collision_dimensions(b);
        let (x, y) = a.motion.pixels();
        if i32::from(x) >= i32::from(b.x) + i32::from(w)
            || i32::from(y) >= i32::from(b.y) + i32::from(h)
            || i32::from(x) + i32::from(a.width) <= i32::from(b.x)
            || i32::from(y) + i32::from(a.height) <= i32::from(b.y)
        {
            return None;
        }
    }
    let a_points = a.outline.as_ref()?;
    if b.definition == Def::PowerDistributor {
        // 272c:1556 leaves only the first directed switch edge active.
        // Treating its backing rectangle as solid retriggers the reverse side.
        let points = b.outline.as_ref()?;
        let dx = f64::from(points[1].0) - f64::from(points[0].0);
        let dy = f64::from(points[1].1) - f64::from(points[0].1);
        let (nx, ny) = unit_vector(dy, -dx);
        if f64::from(a.motion.velocity_x) * nx + f64::from(a.motion.velocity_y) * ny >= 0.0 {
            return None;
        }
        convex_part_contact(a, a_points, b, &points[..2])?;
        let line = (f64::from(b.x) + f64::from(points[0].0)) * nx
            + (f64::from(b.y) + f64::from(points[0].1)) * ny;
        let min = a_points
            .iter()
            .map(|&(x, y)| {
                (f64::from(a.motion.position_x) / 512.0 + f64::from(x)) * nx
                    + (f64::from(a.motion.position_y) / 512.0 + f64::from(y)) * ny
            })
            .fold(f64::INFINITY, f64::min);
        return Some((nx, ny, line - min));
    }
    crate::outline::convex_parts(b)
        .iter()
        .filter_map(|part| convex_part_contact(a, a_points, b, part))
        .min_by(|p, q| p.2.total_cmp(&q.2))
}
#[cfg(test)]
pub(super) fn convex_part_contact(
    a: &Object,
    a_points: &[(u8, u8)],
    b: &Object,
    b_points: &[(u8, u8)],
) -> Option<(f64, f64, f64)> {
    let ax = f64::from(a.motion.position_x) / 512.0;
    let ay = f64::from(a.motion.position_y) / 512.0;
    let bx = f64::from(b.x);
    let by = f64::from(b.y);
    let mut best = (0.0, 0.0, f64::INFINITY);
    for points in [a_points, b_points] {
        for (&(x, y), &(ex, ey)) in points.iter().zip(points.iter().cycle().skip(1)) {
            let dx = f64::from(ex) - f64::from(x);
            let dy = f64::from(ey) - f64::from(y);
            let length = dx.hypot(dy);
            if length == 0.0 {
                continue;
            }
            let (nx, ny) = (-dy / length, dx / length);
            let project = |p: &[(u8, u8)], ox: f64, oy: f64| {
                p.iter()
                    .fold((f64::INFINITY, f64::NEG_INFINITY), |(lo, hi), &(px, py)| {
                        let value = (ox + f64::from(px)) * nx + (oy + f64::from(py)) * ny;
                        (lo.min(value), hi.max(value))
                    })
            };
            let (alo, ahi) = project(a_points, ax, ay);
            let (blo, bhi) = project(b_points, bx, by);
            if ahi <= blo || bhi <= alo {
                return None;
            }
            let negative = ahi - blo;
            let positive = bhi - alo;
            let (sign, depth) = if negative < positive {
                (-1.0, negative)
            } else {
                (1.0, positive)
            };
            if depth < best.2 {
                best = (nx * sign, ny * sign, depth);
            }
        }
    }
    best.2.is_finite().then_some(best)
}

use super::*;

#[test]
fn light_rope_contact_sets_power_for_the_light_callback() {
    let mut light = title().objects[0].clone();
    light.definition = Def::Light;
    light.state = 2;
    light.power_state = 0;
    light.pending_light_power = 0;

    assert!(!process_light_rope_contact(&mut light, 0x000c));
    assert_eq!(light.power_state, 1);
    illuminate_light_on_power(std::slice::from_mut(&mut light), 0);
    assert_eq!(
        (light.state, light.power_state, light.pending_light_power),
        (3, 1, 0)
    );
}

#[test]
fn impacted_bullet_hides_and_clears_contact_on_state_two() {
    let mut bullet = title().objects[0].clone();
    bullet.definition = Def::Bullet;
    bullet.state = 0;
    bullet.motion.velocity_x = 0x1000;
    bullet.outline = Some(vec![(28, 0), (39, 1), (39, 5), (28, 6)]);
    bullet.outline_angles = crate::outline::edge_angles(bullet.outline.as_deref().unwrap());

    update_bullet_flight(&mut bullet);

    assert_eq!(bullet.state, 1);
    assert_eq!(bullet.outline.as_deref().unwrap().len(), 4);
    bullet.contact = Some(crate::machine::ContactRecord {
        target: 1,
        outline_edge: 0,
        angle: 0,
        flag_a: false,
        flag_b: false,
    });
    bullet.state = 2;
    update_bullet_flight(&mut bullet);
    assert!(bullet.contact.is_none());
    assert!(
        bullet
            .render_flags
            .contains(crate::machine::RenderFlag::Hidden)
    );
}

#[test]
fn detonator_contact_matches_native_edge_and_head_conditions() {
    let mut objects = title().objects[..2].to_vec();
    objects[0].definition = Def::CannonProjectile;
    objects[0].height = 23;
    objects[1].definition = Def::Detonator;
    objects[1].y = 100;
    // edge, vertical velocity, angle, ball bottom, expected trigger.
    for (edge, velocity, angle, bottom, pressed) in [
        (0, -333, 0x4000, 160, true),
        (0, 0, 0, 123, true),
        (1, 100, 0, 111, true),
        (1, 100, 0x7ff, 111, true),
        (1, 100, 0x800, 111, false),
        (1, 100, 0, 112, false),
        (1, 0, 0, 111, false),
        (1, -100, 0, 111, false),
        (1, 100, 0x4000, 111, false),
    ] {
        objects[0].y = bottom - 23;
        objects[0].motion.velocity_y = velocity;
        objects[1].power_state = 0;
        assert_eq!(
            trigger_detonator_contact(&mut objects, 0, 1, edge, angle),
            CollisionResponse::Ordinary
        );
        assert_eq!(
            objects[1].power_state != 0,
            pressed,
            "edge={edge}, velocity={velocity}, angle={angle}, bottom={bottom}"
        );
    }
}

#[test]
fn title_glove_launch_clears_ramp_and_activates_scissors() {
    let mut sim = title();
    let baseball = sim
        .objects
        .iter()
        .position(|p| p.definition == Def::Baseball)
        .unwrap();
    let scissors = sim
        .objects
        .iter()
        .position(|p| p.definition == Def::Scissors)
        .unwrap();
    let mut cleared_ramp = false;
    let mut scissors_contact = false;
    for _ in 0..273 {
        sim.step().unwrap();
        let ball = &sim.objects[baseball];
        cleared_ramp |= ball.x > 520 && ball.y < 312;
        scissors_contact |= ball.contact.is_some_and(|c| c.target == scissors)
            && sim.objects[scissors].power_state != 0;
    }
    assert!(cleared_ramp, "baseball fell back at the title ramp");
    assert!(
        scissors_contact,
        "baseball never activated the scissors by contact"
    );
    assert_eq!(sim.objects[scissors].state, 1);
}

#[test]
fn title_scissors_split_tether_and_release_balloon_once() {
    let mut sim = title();
    let original = sim.objects.clone();
    let balloon = sim
        .objects
        .iter()
        .position(|p| p.definition == Def::Balloon)
        .unwrap();
    let scissors = sim
        .objects
        .iter()
        .position(|p| p.definition == Def::Scissors)
        .unwrap();
    for _ in 0..176 {
        sim.step().unwrap();
    }
    assert_eq!(
        sim.objects
            .iter()
            .filter(|p| p.definition == Def::Rope)
            .count(),
        original
            .iter()
            .filter(|p| p.definition == Def::Rope)
            .count()
            + 1
    );
    let after_cut = sim.objects.len();
    update_scissors_snap(&mut sim.objects, scissors).unwrap();
    assert_eq!(sim.objects.len(), after_cut);
    for _ in 176..195 {
        sim.step().unwrap();
    }
    let ends: Vec<_> = sim
        .objects
        .iter()
        .enumerate()
        .filter(|(_, p)| p.definition == Def::RopeEnd)
        .collect();
    assert_eq!(ends.len(), 2);
    assert!(
        sim.objects[balloon].y < original[balloon].y - 30,
        "cut balloon stayed pinned"
    );
    assert!(
        ends.iter().any(|(_, p)| p.y > original[scissors].y + 15),
        "lower loose end did not fall"
    );
    for object in &sim.objects {
        if object.definition == Def::Rope {
            for rope in object.ropes.iter().flatten() {
                let route = crate::machine::rope_spans(&sim.objects, rope).unwrap();
                assert_eq!(route.last().map(|s| s.to), rope.endpoints[1]);
            }
        }
    }

    sim.reset();
    assert_eq!(sim.objects, original);
}

#[test]
fn scissors_split_preserves_pulley_route_and_both_rope_lengths() {
    let original = title();
    let mut objects: Vec<_> = (0..5).map(|_| original.objects[5].clone()).collect();
    for (i, p) in objects.iter_mut().enumerate() {
        p.x = 30;
        p.y = [0, 0, 0, 10, 40][i];
        p.motion = crate::motion::Motion::at_pixels(p.x, p.y);
        p.rope_anchors = [(0, 0); 2];
        p.ropes = [None, None];
        p.linked_objects = [None; 6];
        p.render_flags = crate::machine::RenderFlags::EMPTY;
        p.state = 0;
    }
    objects[0].x = 0;
    objects[0].definition = Def::Scissors;
    objects[1].definition = Def::Rope;
    objects[1].list = ObjectList::Fixed;
    objects[2].definition = Def::Balloon;
    objects[2].list = ObjectList::Moving;
    objects[3].definition = Def::Pulley;
    objects[3].list = ObjectList::Fixed;
    objects[4].definition = Def::EyeHook;
    objects[4].list = ObjectList::Fixed;
    objects[2].linked_objects[0] = Some(3);
    objects[3].linked_objects[0] = Some(4);
    objects[3].linked_objects[1] = Some(2);
    objects[4].linked_objects[0] = Some(3);
    let rope = crate::machine::RopeConnection {
        endpoints: [Some(2), Some(4)],
        attachment_slots: [0, 0],
        rest_length: 40.0,
        slack_count: 0,
    };
    objects[1].ropes[0] = Some(rope);
    objects[1].activation_timer = 10;
    objects[1].callback_timer = 30;
    rope_cut::split_ropes_at_object(&mut objects, 0).unwrap();
    assert_eq!(objects.len(), 8);
    let left = objects[1].ropes[0].as_ref().unwrap();
    let right = objects[7].ropes[0].as_ref().unwrap();
    assert_eq!((objects[7].x, objects[7].y), (0, 0));
    assert_eq!(
        (objects[7].motion.position_x, objects[7].motion.position_y),
        (0, 0)
    );
    assert_eq!((left.rest_length, right.rest_length), (15.0, 25.0));
    assert_eq!(
        (objects[1].activation_timer, objects[1].callback_timer),
        (10, 5)
    );
    assert_eq!(
        (objects[7].activation_timer, objects[7].callback_timer),
        (25, 25)
    );
    let route = crate::machine::rope_spans(&objects, left).unwrap();
    assert_eq!(
        route.iter().map(|s| (s.from, s.to)).collect::<Vec<_>>(),
        vec![(2, 3), (3, 5)]
    );
    assert_eq!(
        crate::machine::rope_spans(&objects, right).unwrap()[0].to,
        4
    );
    assert_eq!(objects[4].linked_objects[0], Some(6));
}

#[test]
fn credits_baseball_switches_torch_and_dynamite_contacts_concrete() {
    let root = crate::test_support::game_data_root();
    let resources = crate::resources::Resources::load(root).unwrap();
    let mut sim = MachineSimulation::load_collection(&resources, "CREDITS.GKC").unwrap();
    let ball = sim
        .objects
        .iter()
        .position(|p| p.definition == Def::Baseball)
        .unwrap();
    let torch = sim
        .objects
        .iter()
        .position(|p| p.definition == Def::Flashlight)
        .unwrap();
    let dynamite = sim
        .objects
        .iter()
        .position(|p| p.definition == Def::Dynamite)
        .unwrap();
    let start = sim.objects[ball].y;
    let mut concrete_contact = false;
    let bowling = sim
        .objects
        .iter()
        .position(|p| p.definition == Def::BowlingBall)
        .unwrap();
    let mut torch_contact = false;
    let mut trampoline_contact = false;
    let mut rat_drive = false;
    let mut seesaw_contact = false;
    let mut underside_deflects_left = false;
    let mut powered_motor_turned = false;
    let mut powered_conveyor_turned = false;
    let cannonball = sim
        .objects
        .iter()
        .position(|p| p.definition == Def::CannonProjectile)
        .unwrap();
    let lower_torch = sim
        .objects
        .iter()
        .position(|p| p.definition == Def::Flashlight && p.y > 300)
        .unwrap();
    let mut jack_ball_reached_torch = false;
    for tick in 0..339 {
        sim.step().unwrap();
        jack_ball_reached_torch |= sim.objects[cannonball]
            .contact
            .is_some_and(|c| c.target == lower_torch)
            && sim.objects[cannonball].motion.velocity_x < 0
            && sim.objects[lower_torch].power_state != 0;
        if let Some(c) = sim.objects[ball].contact {
            torch_contact |= c.target == torch;
            trampoline_contact |= sim.objects[c.target].definition == Def::Trampoline;
            seesaw_contact |= sim.objects[c.target].definition == Def::Seesaw;
        }
        // Accepted trampoline/seesaw callbacks clear their contact record.
        trampoline_contact |= sim
            .objects
            .iter()
            .any(|p| p.definition == Def::Trampoline && p.power_state != 0);
        seesaw_contact |= sim
            .objects
            .iter()
            .any(|p| p.definition == Def::Seesaw && p.power_state != 0 && tick > 70);
        rat_drive |= sim
            .objects
            .iter()
            .any(|p| p.definition == Def::ElectricalOutlet && p.power_state != 0);
        powered_motor_turned |= sim.objects.iter().any(|p| {
            p.definition == Def::ElectricalMotor && p.state != p.saved_state && p.power_state != 0
        });
        powered_conveyor_turned |= sim
            .objects
            .iter()
            .any(|p| p.definition == Def::Motor && p.state != p.saved_state && p.power_state != 0);
        underside_deflects_left |= sim.objects[bowling].contact.is_some_and(|c| {
            sim.objects[c.target].definition == Def::Trampoline && c.outline_edge == 2
        }) && sim.objects[bowling].motion.velocity_x < 0;
        concrete_contact |= sim.objects[dynamite]
            .contact
            .is_some_and(|c| sim.objects[c.target].definition == Def::BrickWall);
        crate::machine::live_collection_frame(&resources, "CREDITS.GKC", tick, &sim.objects)
            .unwrap();
    }
    assert!(sim.objects[ball].y > start, "baseball never fell");
    assert!(
        jack_ball_reached_torch,
        "jack cannonball did not roll left over the lower torch and activate it"
    );
    assert_eq!(
        (sim.objects[torch].power_state, sim.objects[torch].state),
        (1, 1)
    );
    assert!(
        concrete_contact,
        "dynamite fell through its concrete platform"
    );
    assert!(sim.objects[dynamite].y < 200);
    assert!(
        torch_contact && trampoline_contact && rat_drive && seesaw_contact,
        "credits chain did not complete"
    );
    assert!(
        underside_deflects_left,
        "bowling ball did not deflect left below trampoline"
    );
    assert!(
        powered_motor_turned && powered_conveyor_turned,
        "electrical motor did not drive its belt"
    );
    let jack = sim
        .objects
        .iter()
        .find(|p| p.definition == Def::JackInTheBox)
        .unwrap();
    assert_eq!(
        (jack.state, jack.activation_timer),
        (18, 6),
        "credits jack did not pop after winding"
    );
}

#[test]
fn title_text_breaks_bowl_and_settles_underneath_in_sign_slot() {
    let mut sim = title();
    let text = sim
        .objects
        .iter()
        .position(|p| p.definition == Def::TitleThe)
        .unwrap();
    let fish = sim
        .objects
        .iter()
        .position(|p| p.definition == Def::Fish)
        .unwrap();
    let switch = sim
        .objects
        .iter()
        .position(|p| p.definition == Def::PowerDistributor)
        .unwrap();
    let mut broken = false;
    for _ in 0..300 {
        sim.step().unwrap();
        broken |= sim.objects[fish].state >= 11;
    }
    assert!(broken, "THE never broke the bowl");
    assert_eq!(
        sim.objects[switch].power_state, 1,
        "released balloon never switched on the machine"
    );
    assert!((121..=126).contains(&sim.objects[text].x));
    assert_eq!(sim.objects[text].y, 123);
    assert_eq!(
        (
            sim.objects[text].motion.velocity_x,
            sim.objects[text].motion.velocity_y
        ),
        (0, 0)
    );
}

#[test]
fn wall_switch_powers_cable_motor_belt_jack_and_fan_then_switches_off() {
    let mut sim = title();
    let mut plug = sim
        .objects
        .iter()
        .find(|p| p.definition == Def::PowerDistributor)
        .unwrap()
        .clone();
    let root = crate::test_support::game_data_root();
    let resources = crate::resources::Resources::load(root).unwrap();
    let credits = crate::machine::collection_objects(&resources, "CREDITS.GKC").unwrap();
    let motor = credits
        .iter()
        .find(|p| p.definition == Def::ElectricalMotor)
        .unwrap()
        .clone();
    let jack = credits
        .iter()
        .find(|p| p.definition == Def::JackInTheBox)
        .unwrap()
        .clone();
    let mut fan = sim
        .objects
        .iter()
        .find(|p| p.definition == Def::Fan)
        .unwrap()
        .clone();
    let mut belt = credits
        .iter()
        .find(|p| p.definition == Def::Belt)
        .unwrap()
        .clone();
    plug.linked_objects[4] = Some(1);
    plug.linked_objects[5] = Some(3);
    // Plugged in as the editor does (rebind_held_object_attachment), so
    // the saved switch state includes both plugs.
    crate::machine::refresh_outlet_state(&mut plug);
    plug.saved_state = plug.state;
    belt.belt = Some([Some(1), Some(2)]);
    fan.linked_objects.fill(None);
    sim.objects = vec![plug, motor, jack, fan, belt];
    let x = sim.objects[0].x + 8;
    power_distributor_contact_trigger(&mut sim.objects[0], x);
    for _ in 0..60 {
        sim.step().unwrap();
    }
    assert_eq!(sim.objects[1].power_state, 1);
    assert_eq!(
        sim.objects[2].state, 18,
        "powered jack did not finish popping"
    );
    assert_eq!(sim.objects[2].activation_timer, 6);
    assert_eq!(sim.objects[3].power_state, 1);
    let off = adjust_power_distributor_level(&mut sim.objects, 2, 0, 0);
    assert_eq!(off, CollisionResponse::SuppressBounce);
    sim.step().unwrap();
    assert_eq!(sim.objects[1].power_state, 0);
    // Fixed callbacks follow native reverse insertion order, so the jack
    // observes the outlet's off state on the next update.
    sim.step().unwrap();
    assert_eq!(sim.objects[2].power_state, 0);
    assert_eq!(sim.objects[3].power_state, 0);
}

#[test]
fn powered_jack_throws_a_part_above_its_lid_by_native_mass() {
    let root = crate::test_support::game_data_root();
    let resources = crate::resources::Resources::load(root).unwrap();
    let credits = crate::machine::collection_objects(&resources, "CREDITS.GKC").unwrap();
    let mut jack = credits
        .iter()
        .find(|p| p.definition == Def::JackInTheBox)
        .unwrap()
        .clone();
    let mut ball = credits
        .iter()
        .find(|p| p.definition == Def::Baseball)
        .unwrap()
        .clone();
    jack.state = 7;
    jack.activation_timer = 5;
    jack.power_state = 1;
    ball.x = jack.x + 8;
    ball.y = jack.y - 15;
    let mut objects = vec![jack, ball];
    update_jack_in_the_box(&mut objects, 0);
    assert_eq!(objects[0].state, 8);
    assert_eq!(
        (objects[1].motion.velocity_x, objects[1].motion.velocity_y),
        (-0x1400, -0x1400)
    );
}

#[test]
fn lit_flipped_torch_keeps_its_body_at_the_same_screen_pixels() {
    let root = crate::test_support::game_data_root();
    let resources = crate::resources::Resources::load(root).unwrap();
    let torch = crate::machine::collection_objects(&resources, "CREDITS.GKC")
        .unwrap()
        .into_iter()
        .find(|p| p.definition == Def::Flashlight)
        .unwrap();
    assert!(
        torch
            .render_flags
            .contains(crate::machine::RenderFlag::FlipHorizontal)
    );
    let mut objects = vec![torch];
    let unlit =
        crate::machine::live_collection_frame(&resources, "CREDITS.GKC", 0, &objects).unwrap();
    objects[0].state = 1;
    let lit =
        crate::machine::live_collection_frame(&resources, "CREDITS.GKC", 0, &objects).unwrap();
    let p = &objects[0];
    let mut painted = 0;
    let mut same = 0;
    for y in p.y + 16..p.y + 16 + p.height as i16 {
        for x in p.x..p.x + p.width as i16 {
            let index = (y as usize * 640 + x as usize) * 4;
            let a = &unlit.rgba[index..index + 4];
            let b = &lit.rgba[index..index + 4];
            if a != [192, 192, 192, 255] {
                painted += 1;
                same += usize::from(a == b);
            }
        }
    }
    assert!(painted > 100);
    assert!(
        same * 100 >= painted * 90,
        "torch body moved: {same}/{painted} pixels remained"
    );
}

#[test]
fn native_surface_reflection_uses_material_and_fixed_energy_loss() {
    let mut motion = crate::motion::Motion::at_pixels(0, 0);
    motion.velocity_x = 1024;
    motion.velocity_y = 2048;
    reflect_velocity_off_contact_surface(&mut motion, 0, 64, 9695);
    assert_eq!((motion.velocity_x, motion.velocity_y), (1024, -448));
    motion.velocity_y = 200;
    reflect_velocity_off_contact_surface(&mut motion, 0, 64, 9695);
    assert_eq!(motion.velocity_y, 0);
    assert_eq!(rotate_vector_by_angle(1024, 2048, 0x4000), (-2048, 1024));
}

fn title() -> TitleSimulation {
    let root = crate::test_support::game_data_root();
    let resources = crate::resources::Resources::load(root).unwrap();
    TitleSimulation::load(&resources).unwrap()
}

#[test]
fn reset_restores_loaded_graph_and_removes_spawned_parts_for_every_collection() {
    let root = crate::test_support::game_data_root();
    let resources = crate::resources::Resources::load(root).unwrap();
    let mut count = 0;
    for name in resources
        .names()
        .filter(|n| n.ends_with(".LEV") || n.ends_with(".GKC"))
    {
        let mut sim = MachineSimulation::load_collection(&resources, name).unwrap();
        let original = sim.objects.clone();
        let mut spawned = original[0].clone();
        spawned.list = ObjectList::Moving;
        sim.objects.push(spawned);
        for object in &mut sim.objects {
            object.state = 99;
            object.power_state = -1;
            object.callback_timer = 55;
            object.activation_timer = 88;
            object.x = -100;
            object.rope_anchors = [(0, 0); 2];
            object.ropes = [None, None];
            object.linked_objects = [None; 6];
            object.motion.velocity_x = 123;
        }
        sim.trace_tick = 100;
        sim.reset();
        assert_eq!(sim.objects, original, "reset graph mismatch for {name}");
        assert_eq!(sim.trace_tick, 0);
        count += 1;
    }
    assert_eq!(count, 89);
}

#[test]
fn level_render_keeps_spawned_parts_after_inventory_and_hides_disabled_parts() {
    let root = crate::test_support::game_data_root();
    let resources = crate::resources::Resources::load(root).unwrap();
    let mut sim = MachineSimulation::load_collection(&resources, "L1.LEV").unwrap();
    assert!(sim.objects.iter().any(|o| o.list == ObjectList::Inventory));
    let baseline = crate::machine::live_collection_frame(&resources, "L1.LEV", 0, &sim.objects)
        .unwrap()
        .rgba;
    let mut ball = title()
        .objects
        .into_iter()
        .find(|o| o.definition == Def::BowlingBall)
        .unwrap();
    ball.x = 400;
    ball.y = 200;
    ball.state = 0;
    ball.render_flags = crate::machine::RenderFlags::EMPTY;
    ball.list = ObjectList::Moving;
    sim.objects.push(ball);
    let spawned = crate::machine::live_collection_frame(&resources, "L1.LEV", 0, &sim.objects)
        .unwrap()
        .rgba;
    assert_ne!(spawned, baseline);
    sim.objects
        .last_mut()
        .unwrap()
        .render_flags
        .insert(crate::machine::RenderFlag::Hidden);
    let disabled = crate::machine::live_collection_frame(&resources, "L1.LEV", 0, &sim.objects)
        .unwrap()
        .rgba;
    assert_eq!(disabled, baseline);
}

#[test]
fn fishbowl_impact_retains_native_base_vertices_and_original_angles() {
    let mut fish = title()
        .objects
        .into_iter()
        .find(|o| o.definition == Def::Fish)
        .unwrap();
    assert_eq!(
        fish.outline.as_deref().unwrap(),
        &[
            (0, 18),
            (11, 0),
            (37, 0),
            (47, 18),
            (47, 35),
            (39, 47),
            (8, 47),
            (0, 34)
        ]
    );
    let angles = fish.outline_angles.clone();
    assert_eq!(
        advance_fish_state(&mut fish),
        CollisionResponse::SuppressBounce
    );
    assert_eq!(fish.state, 11);
    assert_eq!(
        fish.outline.as_deref().unwrap(),
        &[(8, 47), (24, 44), (39, 47)]
    );
    assert_eq!(
        fish.outline_angles,
        angles[..3],
        "272c:1c9e does not recompute the angles"
    );
    assert_eq!(advance_fish_state(&mut fish), CollisionResponse::Ordinary);
    for _ in 0..30 {
        update_fish_swim(&mut fish);
    }
    assert_eq!(
        fish.outline.as_deref().unwrap(),
        &[(8, 47), (24, 44), (39, 47)]
    );
    assert_eq!(fish.outline_angles, angles[..3]);
}

#[test]
fn fish_update_wraps_intact_animation_and_freezes_after_twenty_broken_loops() {
    let mut fish = title()
        .objects
        .into_iter()
        .find(|o| o.definition == Def::Fish)
        .unwrap();
    fish.state = 10;
    fish.callback_timer = 0;
    update_fish_swim(&mut fish);
    assert_eq!(fish.state, 0);
    fish.state = 22;
    fish.callback_timer = 19;
    update_fish_swim(&mut fish);
    assert_eq!(fish.state, 14);
    assert_eq!(fish.callback_timer, 20);
    for _ in 0..30 {
        update_fish_swim(&mut fish);
    }
    assert_eq!(fish.state, 14);
    assert_eq!(fish.callback_timer, 20);
}

#[test]
fn live_fish_render_uses_simulation_state_without_mutation() {
    let root = crate::test_support::game_data_root();
    let resources = crate::resources::Resources::load(root).unwrap();
    let mut sim = MachineSimulation::load(&resources).unwrap();
    let index = sim
        .objects
        .iter()
        .position(|o| o.definition == Def::Fish)
        .unwrap();
    for state in 0..23 {
        sim.objects[index].state = state;
        let before = sim.objects.clone();
        let first = crate::machine::live_collection_frame(&resources, "TITLE.GKC", 0, &sim.objects)
            .unwrap();
        let second =
            crate::machine::live_collection_frame(&resources, "TITLE.GKC", 10, &sim.objects)
                .unwrap();
        assert_eq!(
            first.rgba, second.rgba,
            "render tick overwrote fish state {state}"
        );
        assert_eq!(sim.objects, before);
    }
}

#[test]
fn physical_collision_does_not_invoke_editor_flip_callbacks() {
    for definition in [Def::Fan, Def::Windmill, Def::Cannon, Def::Light] {
        let mut sim = title();
        let mut target = sim
            .objects
            .iter()
            .find(|o| o.definition == definition)
            .unwrap()
            .clone();
        target.list = ObjectList::Fixed;
        target.x = 0;
        target.y = 0;
        target.state = if definition == Def::Light { 2 } else { 0 };
        target.power_state = 0;
        target.callback_timer = 0;
        target.render_flags = crate::machine::RenderFlags::EMPTY;
        target.ropes = [None, None];
        target.belt = None;
        target.linked_objects = [None; 6];
        target.outline = crate::outline::vertices(&target);
        let mut ball = sim
            .objects
            .iter()
            .find(|o| o.definition == Def::BowlingBall)
            .unwrap()
            .clone();
        ball.list = ObjectList::Moving;
        ball.render_flags = crate::machine::RenderFlags::EMPTY;
        ball.ropes = [None, None];
        ball.belt = None;
        ball.linked_objects = [None; 6];
        ball.x = 10;
        ball.y = 10;
        ball.motion = crate::motion::Motion::at_pixels(10, 10);
        if definition == Def::Light {
            assert!(
                target.outline.is_none(),
                "native bulbs have no collision vertices"
            );
            assert!(convex_contact(&ball, &target).is_none());
        } else {
            assert!(
                convex_contact(&ball, &target).is_some(),
                "fixture must contact {definition:?}"
            );
        }
        let state = target.state;
        sim.objects = vec![target, ball];
        sim.step().unwrap();
        assert_eq!(
            sim.objects[0]
                .render_flags
                .contains(crate::machine::RenderFlag::FlipHorizontal),
            false,
            "collision flipped {definition:?}"
        );
        assert_eq!(
            sim.objects[0].state, state,
            "collision changed {definition:?}"
        );
    }
}

fn solar_fixture(source_definition: Def, source_x: i16) -> Vec<Object> {
    let root = crate::test_support::game_data_root();
    let resources = crate::resources::Resources::load(root).unwrap();
    let sim = MachineSimulation::load_collection(&resources, "L13.LEV").unwrap();
    let mut panel = sim
        .objects
        .into_iter()
        .find(|o| o.definition == Def::SolarPanel)
        .unwrap();
    panel.x = 0;
    panel.y = 0;
    panel.width = 32;
    panel.height = 32;
    panel.power_state = 0;
    panel.render_flags = crate::machine::RenderFlags::EMPTY;
    panel.linked_objects = [None, None, None, None, Some(2), Some(3)];
    let mut source = title()
        .objects
        .into_iter()
        .find(|o| o.definition == Def::Light)
        .unwrap();
    source.definition = source_definition;
    source.x = source_x;
    source.y = 0;
    source.width = 32;
    source.height = 32;
    source.power_state = 1;
    source.render_flags = crate::machine::RenderFlags::EMPTY;
    let mut output = source.clone();
    output.definition = Def::Baseball;
    output.power_state = -7;
    vec![panel, source, output.clone(), output]
}

#[test]
fn machine_dispatch_samples_solar_panel_on_fifth_update_from_reset() {
    let mut sim = title();
    sim.objects = solar_fixture(Def::Light, -40);
    sim.trace_tick = 0;
    for _ in 0..4 {
        sim.step().unwrap();
        assert_eq!(sim.objects[0].power_state, 0);
    }
    sim.step().unwrap();
    assert_eq!(sim.objects[0].power_state, 1);
    assert_eq!(sim.objects[2].power_state, 1);
    assert_eq!(sim.objects[3].power_state, 1);
}

#[test]
fn solar_panel_samples_on_phase_four_and_forwards_both_outputs_every_tick() {
    let mut objects = solar_fixture(Def::Light, -40);
    update_solar_panel_power(&mut objects, 0, 3).unwrap();
    assert_eq!(objects[0].power_state, 0);
    update_solar_panel_power(&mut objects, 0, 4).unwrap();
    assert_eq!(objects[0].power_state, 1);
    assert_eq!(objects[2].power_state, 1);
    assert_eq!(objects[3].power_state, 1);
    objects[1].power_state = 0;
    objects[2].power_state = 0;
    update_solar_panel_power(&mut objects, 0, 5).unwrap();
    assert_eq!(objects[0].power_state, 1);
    assert_eq!(objects[2].power_state, 1);
    update_solar_panel_power(&mut objects, 0, 12).unwrap();
    assert_eq!(objects[0].power_state, 0);
    assert_eq!(objects[2].power_state, 0);
    assert_eq!(objects[3].power_state, 0);
}

#[test]
fn solar_panel_accepts_only_native_light_sources_with_directional_flashlight() {
    for definition in [Def::Light, Def::Candle, Def::Explosion] {
        let mut objects = solar_fixture(definition, -40);
        update_solar_panel_power(&mut objects, 0, 4).unwrap();
        assert_eq!(objects[0].power_state, 1, "source {definition:?}");
    }
    for (x, flags, expected) in [(-40, 0, 1), (-40, 0x10, 0), (40, 0, 0), (40, 0x10, 1)] {
        let mut objects = solar_fixture(Def::Flashlight, x);
        objects[1].render_flags = flags.into();
        update_solar_panel_power(&mut objects, 0, 4).unwrap();
        assert_eq!(objects[0].power_state, expected);
    }
    let mut objects = solar_fixture(Def::Motor, -40);
    update_solar_panel_power(&mut objects, 0, 4).unwrap();
    assert_eq!(objects[0].power_state, 0);
}

#[test]
fn solar_panel_range_uses_native_frame_extents_at_both_edges() {
    // The panel's +0x44 width is 72 pixels even though its saved body size
    // is 32, so the right-side 26-pixel boundary lies at x=98.
    for (x, expected) in [(98, 1), (99, 0), (-58, 1), (-59, 0)] {
        let mut objects = solar_fixture(Def::Light, x);
        update_solar_panel_power(&mut objects, 0, 4).unwrap();
        assert_eq!(objects[0].power_state, expected);
    }
    let mut objects = solar_fixture(Def::Light, 0);
    objects[0].linked_objects[4] = Some(999);
    assert!(update_solar_panel_power(&mut objects, 0, 4).is_err());
}

#[test]
fn callback_errors_reach_the_simulation_caller_with_tick_and_object_context() {
    let mut sim = title();
    sim.objects = solar_fixture(Def::Light, 0);
    sim.objects[0].linked_objects[4] = Some(999);
    let error = sim.step().unwrap_err().to_string();
    assert!(error.contains("tick 0, object 0 (SolarPanel)"), "{error}");
    assert!(
        error.contains("Invalid solar-panel electrical link"),
        "{error}"
    );
}

#[test]
fn every_definition_dispatches_without_unsupported_reports() {
    // With every native update ported, no placed part is reported, and
    // reset still clears the list.
    let mut sim = title();
    let template = sim
        .objects
        .iter()
        .find(|o| o.definition == Def::BowlingBall)
        .unwrap()
        .clone();
    sim.objects = (0..Def::COUNT as u16)
        .map(|id| {
            let mut part = template.clone();
            part.definition = Def::try_from(id).unwrap();
            part.list = ObjectList::Fixed;
            part.render_flags = crate::machine::RenderFlags::EMPTY;
            part.linked_objects = [None; 6];
            part.belt = None;
            part.ropes = [None, None];
            part.x = id as i16 * 8;
            part
        })
        .collect();
    for _ in 0..3 {
        sim.step().unwrap();
    }
    assert!(
        sim.unsupported_updates.is_empty(),
        "{:?}",
        sim.unsupported_updates
    );
    sim.unsupported_updates.push(Def::BowlingBall);
    sim.reset();
    assert!(sim.unsupported_updates.is_empty());
}

#[test]
fn flashlight_activation_preserves_facing_power_and_builds_native_outline() {
    for facing in [0, 0x10] {
        let mut objects = solar_fixture(Def::Flashlight, -40);
        let flashlight = &mut objects[1];
        flashlight.state = 0;
        flashlight.power_state = 0;
        flashlight.render_flags = facing.into();
        update_flashlight_switch(flashlight);
        assert_eq!(flashlight.state, 0);
        flashlight.power_state = -1;
        update_flashlight_switch(flashlight);
        assert_eq!(flashlight.state, 1);
        assert_eq!(flashlight.power_state, -1);
        assert_eq!(
            flashlight
                .render_flags
                .contains(crate::machine::RenderFlag::FlipHorizontal),
            facing != 0
        );
        assert_eq!(flashlight.outline.as_ref().unwrap().len(), 6);
        assert_eq!(flashlight.outline_angles.len(), 6);
        let before = flashlight.clone();
        update_flashlight_switch(flashlight);
        assert_eq!(*flashlight, before);
    }
}

#[test]
fn powered_flashlight_dispatch_activates_and_supplies_solar_panel() {
    let mut sim = title();
    sim.objects = solar_fixture(Def::Flashlight, -40);
    sim.objects[1].state = 0;
    sim.trace_tick = 4;
    sim.step().unwrap();
    assert_eq!(sim.objects[1].state, 1);
    assert_eq!(sim.objects[0].power_state, 1);
    assert_eq!(sim.objects[2].power_state, 1);
    assert!(!sim.unsupported_updates.contains(&Def::Flashlight));
}

#[test]
fn candle_timer_threshold_and_flame_loop_preserve_power() {
    let mut objects = solar_fixture(Def::Candle, 0);
    objects[1].state = 0;
    objects[1].power_state = 0;
    objects[1].callback_timer = 20;
    update_candle_flame(&mut objects, 1);
    assert_eq!(objects[1].state, 0);
    objects[1].callback_timer = 21;
    update_candle_flame(&mut objects, 1);
    assert_eq!(objects[1].state, 1);
    assert_eq!(objects[1].power_state, 1);
    for expected in [2, 3, 4, 5, 1] {
        update_candle_flame(&mut objects, 1);
        assert_eq!(objects[1].state, expected);
    }
}

#[test]
fn candle_heat_uses_open_bounds_and_only_odd_flame_states() {
    let mut objects = solar_fixture(Def::Candle, 0);
    objects[0].definition = Def::Balloon;
    objects[0].list = ObjectList::Moving;
    objects[0].x = 9;
    objects[0].y = -5;
    objects[0].width = 8;
    objects[0].height = 8;
    objects[0].power_state = 0;
    objects[1].state = 1;
    update_candle_flame(&mut objects, 1); // state 2, no heat
    assert_eq!(objects[0].power_state, 0);
    update_candle_flame(&mut objects, 1); // state 3, heat
    assert_eq!(objects[0].power_state, 1);
    objects[0].power_state = 0;
    objects[0].x = 18;
    objects[1].state = 2;
    update_candle_flame(&mut objects, 1);
    assert_eq!(objects[0].power_state, 0); // touching right edge excluded
    objects[0].x = 9;
    objects[0].definition = Def::Cat;
    objects[0].state = 0;
    objects[1].state = 2;
    update_candle_flame(&mut objects, 1);
    assert_eq!(objects[0].state, 1);
    objects[0].state = 2;
    objects[1].state = 2;
    update_candle_flame(&mut objects, 1);
    assert_eq!(objects[0].state, 2);
}

#[test]
fn candle_ignition_uses_attachment_points_closed_bounds_and_combustible_flag() {
    let mut objects = solar_fixture(Def::Candle, 0);
    objects[1].state = 1; // next state even; ignition still runs
    objects[0].definition = Def::Cannon;
    objects[0].flags = 4;
    objects[0].x = 8;
    objects[0].y = -13; // anchor (1,3) -> (9,-10)
    objects[0].power_state = 0;
    update_candle_flame(&mut objects, 1);
    assert_eq!(objects[0].power_state, 1);
    objects[0].power_state = 0;
    objects[0].flags = 0;
    update_candle_flame(&mut objects, 1);
    assert_eq!(objects[0].power_state, 0);
    objects[0].flags = 4;
    objects[0].render_flags = crate::machine::RenderFlag::FlipHorizontal.mask();
    update_candle_flame(&mut objects, 1); // flipped anchor now outside
    assert_eq!(objects[0].power_state, 0);
    objects[0].x = -44;
    objects[0].y = 2; // (62,3) -> (18,5)
    update_candle_flame(&mut objects, 1);
    assert_eq!(objects[0].power_state, 1);
}

#[test]
fn turning_gear_pops_balloons_and_pushes_parts_by_edge() {
    let mut objects = solar_fixture(Def::Gear, 0);
    objects.truncate(2);
    objects[1].definition = Def::Gear;
    objects[1].previous_state = 0;
    objects[1].state = 0;
    objects[0].definition = Def::Balloon;
    objects[0].power_state = 0;
    gear_contact(&mut objects, 0, 1, 0);
    assert_eq!(objects[0].power_state, 0, "a still gear does nothing");
    // 3 -> 0 wraps to a forward turn.
    objects[1].previous_state = 3;
    gear_contact(&mut objects, 0, 1, 0);
    assert_eq!(objects[0].power_state, 1, "a turning gear pops the balloon");
    objects[0].definition = Def::BowlingBall;
    objects[0].motion.velocity_x = 0;
    objects[0].motion.velocity_y = 0;
    gear_contact(&mut objects, 0, 1, 2);
    assert_eq!(
        (objects[0].motion.velocity_x, objects[0].motion.velocity_y),
        (0, 0x1000)
    );
    // Backward (0 -> 3): edge 2 acts as edge 6.
    objects[1].previous_state = 0;
    objects[1].state = 3;
    objects[0].motion.velocity_y = 0;
    gear_contact(&mut objects, 0, 1, 2);
    assert_eq!(objects[0].motion.velocity_y, -0x1000);
}

#[test]
fn dynamite_fuse_spawns_native_explosion_and_disables_spent_part_once() {
    let mut sim = title();
    let mut parts = solar_fixture(Def::Dynamite, 100);
    let mut dynamite = parts.remove(1);
    dynamite.state = 0;
    dynamite.power_state = 0;
    dynamite.callback_timer = 20;
    dynamite.ropes = [None, None];
    dynamite.linked_objects = [None; 6];
    sim.objects = vec![dynamite];
    sim.step().unwrap();
    assert_eq!(sim.objects[0].state, 0);
    sim.objects[0].callback_timer = 21;
    for state in 1..=5 {
        sim.step().unwrap();
        assert_eq!(sim.objects[0].state, state);
    }
    sim.step().unwrap();
    assert_eq!(sim.objects.len(), 2);
    assert!(
        sim.objects[0]
            .render_flags
            .contains(crate::machine::RenderFlag::Hidden)
    );
    let explosion = &sim.objects[1];
    assert_eq!(explosion.definition, Def::Explosion);
    assert_eq!((explosion.x, explosion.y), (85, -19));
    assert_eq!(explosion.list, ObjectList::Fixed);
    assert_eq!((explosion.flags, explosion.update_flags), (8, 0x10));
    assert_eq!(explosion.effective_mass, 0);
    assert_eq!((explosion.width, explosion.height), (72, 61));
    sim.step().unwrap();
    assert_eq!(sim.objects.len(), 2);
}

#[test]
fn explosion_lifetime_and_frame_two_chain_reactions() {
    let mut objects = solar_fixture(Def::Dynamite, 0);
    objects[1].linked_objects = [None; 6];
    detonate_dynamite(&mut objects, 1);
    let effect = objects.len() - 1;
    objects[0].definition = Def::Balloon;
    objects[0].x = objects[effect].x;
    objects[0].y = objects[effect].y;
    objects[0].width = 10;
    objects[0].height = 10;
    objects[0].update_flags = 0x1000;
    objects[0].power_state = 0;
    update_explosion_expand(&mut objects, effect);
    assert_eq!(objects[effect].state, 1);
    assert_eq!((objects[effect].width, objects[effect].height), (72, 61));
    assert_eq!(objects[0].power_state, 0);
    update_explosion_expand(&mut objects, effect);
    assert_eq!(objects[0].power_state, 1);
    objects[0].power_state = 0;
    for state in 3..=5 {
        update_explosion_expand(&mut objects, effect);
        assert_eq!(objects[effect].state, state);
    }
    assert_eq!(objects[0].power_state, 0);
    update_explosion_expand(&mut objects, effect);
    assert!(
        objects[effect]
            .render_flags
            .contains(crate::machine::RenderFlag::Hidden)
    );
}

#[test]
fn blast_detonates_nearby_dynamite_and_excludes_disabled_targets() {
    let mut objects = solar_fixture(Def::Dynamite, 0);
    let mut target = objects[1].clone();
    target.update_flags = 0x1000;
    target.render_flags = crate::machine::RenderFlags::EMPTY;
    target.power_state = 0;
    target.x = -15;
    target.y = -19;
    detonate_dynamite(&mut objects, 1);
    let effect = objects.len() - 1;
    objects.push(target.clone());
    target.render_flags = crate::machine::RenderFlag::Hidden.mask();
    objects.push(target);
    let count = objects.len();
    objects[effect].state = 1;
    update_explosion_expand(&mut objects, effect);
    assert_eq!(objects.len(), count + 1);
    assert_eq!(objects[count - 2].state, 5);
    assert_eq!(objects[count - 1].state, objects[1].previous_state);
    assert_eq!(objects.last().unwrap().definition, Def::Explosion);
}

#[test]
fn blast_trims_walls_on_native_tile_grid_and_dispatches_fixed_parts() {
    let mut objects = solar_fixture(Def::Dynamite, 0);
    objects.truncate(2);
    objects[1].linked_objects = [None; 6];
    objects[1].x = 100;
    objects[1].y = 100;
    detonate_dynamite(&mut objects, 1);
    let effect = objects.len() - 1;
    // Explosion bounds advance with its sprite frame; frame 2 is 80x76.
    let mut wall = objects[0].clone();
    wall.definition = Def::BrickWall;
    wall.list = ObjectList::Fixed;
    wall.update_flags = 0x6040;
    wall.render_flags = crate::machine::RenderFlags::EMPTY;
    wall.x = 8;
    wall.y = 120;
    wall.width = 224;
    wall.height = 16;
    wall.alt_size = (224, 16);
    objects[0] = wall.clone();
    let mut tall = wall.clone();
    tall.x = 140;
    tall.y = 72;
    tall.width = 16;
    tall.height = 32;
    tall.alt_size = (16, 32);
    objects.push(tall);
    let mut fish = wall.clone();
    fish.definition = Def::Fish;
    fish.update_flags = 0;
    fish.state = 0;
    fish.x = 100;
    fish.y = 60;
    fish.width = 48;
    fish.height = 48;
    objects.push(fish);
    let count = objects.len();
    update_explosion_expand(&mut objects, effect);
    update_explosion_expand(&mut objects, effect);
    // Horizontal hole [((125-32)&!15)+8, ((125+24)&!15)+8) = [88, 152).
    assert_eq!(
        (objects[0].x, objects[0].width, objects[0].extent_override),
        (8, 224, Some((80, 16)))
    );
    assert_eq!(objects.len(), count + 1);
    let piece = objects.last().unwrap();
    assert_eq!(
        (
            piece.definition,
            piece.x,
            piece.y,
            piece.width,
            piece.height,
            piece.extent_override
        ),
        (Def::BrickWall, 152, 120, 224, 16, Some((80, 16)))
    );
    assert_eq!(
        piece.outline.as_deref(),
        Some(&[(0, 0), (79, 0), (79, 15), (0, 15)][..])
    );
    // Frame-2 extents put the native explosion centre at y=119; the
    // vertical interval cuts the 32-pixel wall to its first 16 pixels.
    assert_eq!(
        (
            objects[count - 2].y,
            objects[count - 2].height,
            objects[count - 2].extent_override
        ),
        (72, 32, Some((16, 16)))
    );
    assert_eq!(objects[count - 1].state, 11, "fixed fish jumps");
}

#[test]
fn credits_dynamite_blasts_concrete_and_throws_parts() {
    let root = crate::test_support::game_data_root();
    let resources = crate::resources::Resources::load(root).unwrap();
    let mut sim = MachineSimulation::load_collection(&resources, "CREDITS.GKC").unwrap();
    let walls = |sim: &MachineSimulation| {
        sim.objects
            .iter()
            .filter(|p| {
                p.definition == Def::BrickWall
                    && !p.render_flags.contains(crate::machine::RenderFlag::Hidden)
            })
            .map(|p| (p.x, p.y, p.width, p.height))
            .collect::<Vec<_>>()
    };
    let before = walls(&sim);
    let mut thrown = false;
    for _ in 0..339 {
        let exploding = sim
            .objects
            .iter()
            .any(|p| p.definition == Def::Explosion && p.state == 1);
        let velocities: Vec<_> = sim
            .objects
            .iter()
            .map(|p| (p.motion.velocity_x, p.motion.velocity_y))
            .collect();
        sim.step().unwrap();
        if exploding {
            thrown |= sim.objects.iter().zip(&velocities).any(|(p, &v)| {
                p.update_flags & 0x1000 != 0
                    && (p.motion.velocity_x, p.motion.velocity_y) != v
                    && p.motion.velocity_x.abs().max(p.motion.velocity_y.abs()) >= 0x400
            });
        }
    }
    assert!(
        sim.objects.iter().any(|p| p.definition == Def::Explosion),
        "dynamite never exploded"
    );
    assert_ne!(walls(&sim), before, "blast left the concrete intact");
    assert!(thrown, "blast did not throw any moving part");
}

#[test]
fn credits_falling_bucket_pulls_seesaw_rope_and_fires_gun() {
    let root = crate::test_support::game_data_root();
    let resources = crate::resources::Resources::load(root).unwrap();
    let mut sim = MachineSimulation::load_collection(&resources, "CREDITS.GKC").unwrap();
    let find =
        |sim: &MachineSimulation, d| sim.objects.iter().position(|p| p.definition == d).unwrap();
    let (bucket, gun) = (find(&sim, Def::Bucket), find(&sim, Def::Gun));
    let seesaw = sim.objects[bucket].linked_objects[0].unwrap();
    let start = (sim.objects[bucket].x, sim.objects[bucket].y);
    let mut fired = None;
    let mut tipped = false;
    for tick in 0..339 {
        sim.step().unwrap();
        tipped |= sim.objects[seesaw].state < 2;
        if tick < 200 {
            // A resting rope carries no whole-pixel stretch.
            assert_eq!(
                (sim.objects[bucket].x, sim.objects[bucket].y),
                start,
                "bucket crept at tick {tick}"
            );
            assert_eq!(
                sim.objects[gun].power_state, 0,
                "gun fired before the blast"
            );
        }
        if fired.is_none() && sim.objects.iter().any(|p| p.definition == Def::Bullet) {
            fired = Some(tick);
        }
    }
    assert!(
        fired.is_some_and(|t| t > 224),
        "gun never fired after the blast: {fired:?}"
    );
    assert!(tipped, "bucket did not tip the seesaw");
    assert_eq!(sim.objects[gun].state, 6);
}

#[test]
fn gun_trigger_accepts_pull_only_from_the_grip_side() {
    let mut gun = solar_fixture(Def::Dynamite, 0).remove(0);
    gun.definition = Def::Gun;
    gun.power_state = 0;
    gun.render_flags = crate::machine::RenderFlags::EMPTY;
    assert!(!gun_rope_contact(std::slice::from_mut(&mut gun), 0, 0x8008));
    assert_eq!(gun.power_state, 0, "probe must not fire");
    assert!(
        gun_rope_contact(std::slice::from_mut(&mut gun), 0, 0x10),
        "muzzle-side pull blocks"
    );
    assert!(!gun_rope_contact(std::slice::from_mut(&mut gun), 0, 8));
    assert_eq!(gun.power_state, 1);
    assert!(
        gun_rope_contact(std::slice::from_mut(&mut gun), 0, 8),
        "a fired gun blocks further pull"
    );
    gun.render_flags = crate::machine::RenderFlag::FlipHorizontal.mask();
    gun.power_state = 0;
    assert!(gun_rope_contact(std::slice::from_mut(&mut gun), 0, 8));
    assert!(!gun_rope_contact(std::slice::from_mut(&mut gun), 0, 0x10));
    assert_eq!(gun.power_state, 1);
}

#[test]
fn balloon_pop_replaces_rope_endpoint_and_disables_after_frame_six() {
    let mut objects = solar_fixture(Def::Balloon, 0);
    objects.truncate(2);
    objects.swap(0, 1);
    objects[0].state = 0;
    objects[0].power_state = 1;
    objects[0].rope_anchors[0] = (7, 9);
    objects[0].linked_objects[0] = Some(1);
    objects[1].linked_objects[0] = Some(0);
    let rope = crate::machine::RopeConnection {
        endpoints: [Some(0), Some(1)],
        attachment_slots: [0, 0],
        rest_length: 50.0,
        slack_count: 0,
    };
    objects[0].ropes = [Some(rope.clone()), None];
    objects[1].ropes = [Some(rope), None];
    update_balloon_pop(&mut objects, 0, 9695);
    assert_eq!(objects.len(), 3);
    assert_eq!(objects[2].definition, Def::RopeEnd);
    assert_eq!((objects[2].x, objects[2].y), (7, 9));
    assert_eq!(objects[1].linked_objects[0], Some(2));
    assert_eq!(
        objects[1].ropes[0].as_ref().unwrap().endpoints,
        [Some(2), Some(1)]
    );
    assert_eq!(
        objects[2].ropes[0].as_ref().unwrap().endpoints,
        [Some(2), Some(1)]
    );
    assert!(objects[0].ropes[0].is_none());
    for state in 2..=6 {
        update_balloon_pop(&mut objects, 0, 9695);
        assert_eq!(objects[0].state, state);
    }
    update_balloon_pop(&mut objects, 0, 9695);
    assert!(
        objects[0]
            .render_flags
            .contains(crate::machine::RenderFlag::Hidden)
    );
    assert_eq!(objects.len(), 3);
}

#[test]
fn balloon_hidden_by_callback_still_integrates_for_that_native_update() {
    let mut sim = title();
    let mut balloon = sim
        .objects
        .iter()
        .find(|part| part.definition == Def::Balloon)
        .unwrap()
        .clone();
    balloon.list = ObjectList::Moving;
    balloon.state = 6;
    balloon.power_state = 1;
    balloon
        .render_flags
        .remove(crate::machine::RenderFlag::Hidden);
    balloon.update_flags = 0x1000;
    balloon.ropes = [None, None];
    balloon.linked_objects = [None; 6];
    balloon.x = 40;
    balloon.y = 80;
    balloon.previous_position = (40, 80);
    balloon.motion.position_x = 40 << 9;
    balloon.motion.position_y = 80 << 9;
    balloon.motion.velocity_x = 90;
    sim.objects = vec![balloon];

    sim.simulate_object_update(0, 0).unwrap();

    let balloon = &sim.objects[0];
    assert!(
        balloon
            .render_flags
            .contains(crate::machine::RenderFlag::Hidden)
    );
    assert_eq!(balloon.motion.position_x, (40 << 9) + 90);
}

#[test]
fn boxing_glove_punch_direction_mass_and_strict_region() {
    for (flags, x, expected) in [(0, -10, -0x1800), (0x10, 60, 0x1800)] {
        let mut objects = solar_fixture(Def::BoxingGlove, 0);
        objects[1].state = 1;
        objects[1].render_flags = flags.into();
        objects[0].definition = Def::Baseball;
        objects[0].update_flags = 0x1000;
        objects[0].x = x;
        objects[0].y = 0;
        objects[0].width = 4;
        objects[0].height = 4;
        objects[0].motion.velocity_x = 99;
        update_boxing_glove_punch(&mut objects, 1);
        assert_eq!(objects[1].state, 2);
        assert_eq!(objects[0].motion.velocity_x, expected);
        assert_eq!(objects[1].outline.as_ref().unwrap().len(), 6);
    }
    let mut objects = solar_fixture(Def::BoxingGlove, 0);
    objects[1].state = 1;
    objects[0].definition = Def::Baseball;
    objects[0].update_flags = 0x1000;
    objects[0].x = 0;
    objects[0].motion.velocity_x = 99;
    update_boxing_glove_punch(&mut objects, 1);
    assert_eq!(objects[0].motion.velocity_x, 99);
    objects[1].state = 9;
    update_boxing_glove_punch(&mut objects, 1);
    assert_eq!(objects[1].state, 9);
}

#[test]
fn boxing_glove_breaks_intact_fish_bowl_once() {
    let mut objects = solar_fixture(Def::BoxingGlove, 0);
    objects[1].state = 1;
    objects[0].definition = Def::Fish;
    objects[0].update_flags = 0;
    objects[0].x = -10;
    objects[0].y = 0;
    objects[0].state = 0;
    update_boxing_glove_punch(&mut objects, 1);
    assert_eq!(objects[0].state, 11);
    assert_eq!(objects[0].outline, Some(vec![(8, 47), (24, 44), (39, 47)]));
    objects[0].state = 14;
    update_boxing_glove_punch(&mut objects, 1);
    assert_eq!(objects[0].state, 14);
}

#[test]
fn magnifying_glass_heats_opposite_combustible_point_until_candle_ignites() {
    let mut objects = solar_fixture(Def::Light, -40);
    objects[0].definition = Def::MagnifyingGlass;
    objects[0].width = 16;
    objects[0].linked_objects = [None; 6];
    objects[1].state = 1;
    objects[2].definition = Def::Candle;
    objects[2].flags = 4;
    objects[2].x = 20;
    objects[2].y = 0;
    objects[2].state = 0;
    objects[2].power_state = 0;
    objects[2].callback_timer = 0;
    objects[3].flags = 0;
    for _ in 0..21 {
        update_magnifying_glass_focus(&mut objects, 0);
    }
    assert_eq!(objects[0].linked_objects[4], Some(2));
    assert_eq!(objects[2].callback_timer, 21);
    update_candle_flame(&mut objects, 2);
    assert_eq!(objects[2].power_state, 1);
    objects[1].state = 2; // unlit bulb state
    update_magnifying_glass_focus(&mut objects, 0);
    assert_eq!(objects[0].linked_objects[4], None);
}

#[test]
fn cage_motor_signed_pulse_and_belt_output_use_separate_timer() {
    for (flags, power) in [(0, 1), (0x10, -1)] {
        let mut objects = solar_fixture(Def::CatPoweredDrive, 0);
        objects[1].render_flags = flags.into();
        objects[1].state = 0;
        objects[1].callback_timer = 17;
        objects[0].belt = Some([Some(1), Some(2)]);
        activate_contacted_object(&mut objects[1]);
        assert_eq!(objects[1].activation_timer, 100);
        for _ in 0..99 {
            update_cat_powered_drive(&mut objects, 1);
        }
        assert_eq!(objects[1].power_state, power);
        assert_eq!(objects[2].power_state, power);
        update_cat_powered_drive(&mut objects, 1);
        assert_eq!(objects[1].power_state, 0);
        assert_eq!(objects[2].power_state, power); // forwarded before expiry
        update_cat_powered_drive(&mut objects, 1);
        assert_eq!(objects[2].power_state, 0);
        assert_eq!(objects[1].callback_timer, 17);
    }
}

#[test]
fn monkey_rope_pull_pedals_both_ways_and_drives_its_belt() {
    for (flags, power) in [(0, 1), (0x10, -1)] {
        let mut objects = solar_fixture(Def::Monkey, 0);
        objects[1].render_flags = flags.into();
        objects[1].state = 0;
        objects[1].power_state = 0;
        objects[1].rope_anchors[0] = (0x4b, 0x2d);
        objects[2].power_state = 0;
        objects[0].belt = Some([Some(1), Some(2)]);
        // A slack or wrong-way rope does nothing; a pull starts the pedals.
        assert!(!monkey_rope_contact(&mut objects[1], 1));
        assert!(monkey_rope_contact(&mut objects[1], 2));
        assert_eq!(objects[1].power_state, 0);
        assert!(!monkey_rope_contact(&mut objects[1], 4));
        assert_eq!(objects[1].power_state, power);
        // Once pedalling, further pulls are absorbed.
        assert!(monkey_rope_contact(&mut objects[1], 4));
        let states: Vec<u16> = (0..6)
            .map(|_| {
                update_monkey_behavior(&mut objects, 1);
                objects[1].state
            })
            .collect();
        assert_eq!(states, [1, 2, 3, 4, 1, 2]);
        assert_eq!(objects[2].power_state, power);
        assert_eq!(objects[1].rope_anchors[0].1, 0x35);
    }
}

#[test]
fn monkey_startle_settles_when_idle_and_rides_off_when_riding() {
    let mut objects = solar_fixture(Def::Monkey, 0);
    objects[1].render_flags = crate::machine::RenderFlags::EMPTY;
    objects[1].state = 0;
    objects[1].power_state = 0;
    objects[1].activation_timer = 0;
    // Only the three upper outline edges startle it.
    startle_monkey(&mut objects[1], 3);
    assert_eq!(objects[1].activation_timer, 0);
    assert_eq!(
        startle_monkey(&mut objects[1], 0),
        CollisionResponse::Ordinary
    );
    assert_eq!((objects[1].state, objects[1].activation_timer), (5, 0x1c));
    let mut seen = Vec::new();
    for _ in 0..0x1b {
        update_monkey_behavior(&mut objects, 1);
        seen.push(objects[1].state);
    }
    assert_eq!(&seen[..5], &[6, 7, 8, 5, 6]);
    update_monkey_behavior(&mut objects, 1);
    assert_eq!((objects[1].state, objects[1].power_state), (0, 0));

    // Riding: a hit stops the pedals, then it rides off again.
    objects[1].power_state = 1;
    objects[1].state = 2;
    startle_monkey(&mut objects[1], 1);
    assert_eq!((objects[1].state, objects[1].power_state), (9, 0));
    for _ in 0..0x1b {
        update_monkey_behavior(&mut objects, 1);
        assert!((9..=12).contains(&objects[1].state));
    }
    update_monkey_behavior(&mut objects, 1);
    assert_eq!((objects[1].state, objects[1].power_state), (1, 1));

    // A rope pull during an idle startle turns it into a ride.
    objects[1].power_state = 0;
    objects[1].state = 0;
    startle_monkey(&mut objects[1], 0);
    update_monkey_behavior(&mut objects, 1);
    monkey_rope_contact(&mut objects[1], 4);
    assert!(objects[1].state > 8);
    for _ in 0..0x1b {
        update_monkey_behavior(&mut objects, 1);
    }
    assert_eq!(objects[1].power_state, 1);
}

#[test]
fn monkey_rope_stays_at_native_hand_through_pedaling_and_startle() {
    let root = crate::test_support::game_data_root();
    let resources = crate::resources::Resources::load(root).unwrap();
    for flipped in [false, true] {
        for riding in [false, true] {
            let mut objects = MachineSimulation::load_collection(&resources, "L62.LEV")
                .unwrap()
                .objects;
            let index = objects
                .iter()
                .position(|o| o.definition == Def::Monkey)
                .unwrap();
            let x = if flipped { 16 } else { 75 };
            objects[index]
                .render_flags
                .remove(crate::machine::RenderFlag::FlipHorizontal);
            if flipped {
                objects[index]
                    .render_flags
                    .insert(crate::machine::RenderFlag::FlipHorizontal);
            }
            objects[index].rope_anchors[0] = (x, 45);
            objects[index].state = 0;
            objects[index].activation_timer = 0;
            objects[index].power_state = if riding {
                monkey_drive(&objects[index])
            } else {
                0
            };
            let connection = crate::machine::RopeConnection {
                endpoints: [Some(index), Some(0)],
                attachment_slots: [0, 0],
                rest_length: 100.0,
                slack_count: 0,
            };
            objects[index].ropes[0] = Some(connection.clone());
            objects[index].linked_objects[0] = Some(0);
            objects[0].linked_objects[0] = Some(index);
            if riding {
                for _ in 0..12 {
                    update_monkey_behavior(&mut objects, index);
                }
            }
            startle_monkey(&mut objects[index], 0);
            for _ in 0..40 {
                update_monkey_behavior(&mut objects, index);
                let monkey = &objects[index];
                let y = if riding { 53 } else { 45 };
                assert_eq!(
                    crate::machine::draw_offset(monkey),
                    (0, 0),
                    "monkey has no native frame-offset table"
                );
                assert_eq!(
                    crate::machine::rope_point(monkey, 0),
                    (i32::from(monkey.x) + x as i32, i32::from(monkey.y) + y),
                    "hand detached in state {} flipped={flipped}",
                    monkey.state
                );
                assert_eq!(monkey.ropes[0], Some(connection.clone()));
                assert_eq!(monkey.linked_objects[0], Some(0));
                let spans = crate::machine::rope_spans(&objects, &connection).unwrap();
                assert_eq!(spans[0].start, crate::machine::rope_point(monkey, 0));
            }
        }
    }
}

#[test]
fn monkey_renders_every_animation_state_both_ways() {
    let root = crate::test_support::game_data_root();
    let resources = crate::resources::Resources::load(root).unwrap();
    let sim = MachineSimulation::load_collection(&resources, "L62.LEV").unwrap();
    let monkey = sim
        .objects
        .iter()
        .position(|o| o.definition == Def::Monkey)
        .unwrap();
    for flipped in [false, true] {
        let flip = if flipped { 0x10 } else { 0 };
        for state in 0..13 {
            let mut objects = sim.objects.clone();
            objects[monkey].state = state;
            objects[monkey]
                .render_flags
                .remove(crate::machine::RenderFlag::FlipHorizontal);
            if flipped {
                objects[monkey]
                    .render_flags
                    .insert(crate::machine::RenderFlag::FlipHorizontal);
            }
            objects[monkey].rope_anchors[0] = (
                if flip == 0 { 75 } else { 16 },
                if (1..=4).contains(&state) || state >= 9 {
                    53
                } else {
                    45
                },
            );
            let hand = objects[monkey].rope_anchors[0];
            assert_eq!(
                crate::machine::rope_point(&objects[monkey], 0),
                (
                    i32::from(objects[monkey].x) + i32::from(hand.0),
                    i32::from(objects[monkey].y) + i32::from(hand.1)
                )
            );
            crate::machine::live_collection_frame(&resources, "L62.LEV", 0, &objects)
                .unwrap_or_else(|e| panic!("state {state}: {e}"));
        }
    }
}

#[test]
fn belt_generator_wraps_signed_groups_and_forwards_zero_when_stalled() {
    let mut objects = solar_fixture(Def::ElectricalOutlet, 0);
    objects[1].linked_objects = [None, None, None, None, Some(2), Some(3)];
    objects[1].state = 7;
    update_electrical_outlet_rotation(&mut objects, 1).unwrap();
    assert_eq!(objects[1].state, 4);
    assert_eq!(objects[2].power_state, 1);
    objects[1].power_state = -1;
    update_electrical_outlet_rotation(&mut objects, 1).unwrap();
    assert_eq!(objects[1].state, 7);
    assert_eq!(objects[3].power_state, -1);
    objects[0].definition = Def::Gear;
    objects[0].previous_state = 2;
    objects[0].older_state = 2;
    objects[2].belt = Some([Some(1), Some(0)]);
    update_electrical_outlet_rotation(&mut objects, 1).unwrap();
    assert_eq!(objects[1].power_state, 0);
    assert_eq!(objects[1].state, 7);
    assert_eq!(objects[2].power_state, 0);
    assert_eq!(objects[3].power_state, 0);
}

#[test]
fn loaded_title_bulb_magnifier_heats_cannon_wick_and_spawns_projectile() {
    let mut sim = title();
    let cannon = sim
        .objects
        .iter()
        .position(|p| p.definition == Def::Cannon)
        .unwrap();
    let lens = sim
        .objects
        .iter()
        .position(|p| p.definition == Def::MagnifyingGlass)
        .unwrap();
    assert_ne!(
        sim.objects[cannon].flags & 4,
        0,
        "native combustible object flag must decode correctly"
    );
    assert_eq!(sim.objects[cannon].update_flags, 0x6400);
    let count = sim.objects.len();
    let mut heated = false;
    for _ in 0..180 {
        sim.step().unwrap();
        heated |= sim.objects[lens].linked_objects[4] == Some(cannon)
            && sim.objects[cannon].callback_timer > 0;
    }
    assert!(heated);
    assert_ne!(sim.objects[cannon].power_state, 0);
    assert!(
        sim.objects[count..]
            .iter()
            .any(|p| p.definition == Def::CannonProjectile)
    );
}

#[test]
fn title_activation_renders_light_live_composites_through_180_updates() {
    let root = crate::test_support::game_data_root();
    let resources = crate::resources::Resources::load(root).unwrap();
    let mut sim = MachineSimulation::load(&resources).unwrap();
    let light = sim
        .objects
        .iter()
        .position(|p| p.definition == Def::Light)
        .unwrap();
    let mut activated = false;
    for tick in 0..180 {
        sim.step().unwrap();
        crate::machine::live_collection_frame(&resources, "TITLE.GKC", tick, &sim.objects).unwrap();
        activated |= sim.objects[light].state == 3;
    }
    assert!(
        activated,
        "test must cover the formerly missing powered light composite"
    );
    for state in 0..4 {
        sim.objects[light].state = state;
        crate::machine::live_collection_frame(&resources, "TITLE.GKC", 0, &sim.objects).unwrap();
    }
}

#[test]
fn simple_collision_responses_distinguish_bullet_and_cannonball() {
    let (mut objects, _) = fan_fixture();
    objects[1].definition = Def::Balloon;
    objects[1].power_state = 0;
    for source in [Def::CannonProjectile, Def::BowlingBall, Def::Bullet] {
        objects[0].definition = source;
        assert_eq!(
            simple_target_contact(&mut objects, 0, 1, 0),
            CollisionResponse::Ordinary
        );
        assert_eq!(
            objects[1].power_state,
            if source == Def::Bullet { 1 } else { 0 }
        );
    }
    objects[1].definition = Def::Cat;
    objects[1].state = 0;
    objects[1].activation_timer = 30;
    simple_target_contact(&mut objects, 0, 1, 0);
    assert_eq!((objects[1].state, objects[1].activation_timer), (1, 0));
    objects[1].activation_timer = 12;
    simple_target_contact(&mut objects, 0, 1, 0);
    assert_eq!(objects[1].activation_timer, 12);
    objects[1].definition = Def::BoxingGlove;
    objects[1].power_state = 0;
    simple_target_contact(&mut objects, 0, 1, 1);
    assert_eq!(objects[1].power_state, 0);
    simple_target_contact(&mut objects, 0, 1, 2);
    assert_eq!(objects[1].power_state, 1);
}

#[test]
fn cannon_wick_composites_animate_and_projectile_uses_native_allocation() {
    let root = crate::test_support::game_data_root();
    let resources = crate::resources::Resources::load(root).unwrap();
    let mut sim = title();
    let cannon = sim
        .objects
        .iter()
        .position(|p| p.definition == Def::Cannon)
        .unwrap();
    let mut previous = None;
    for state in 0..6 {
        sim.objects[cannon].state = state;
        let frame = crate::machine::live_collection_frame(&resources, "TITLE.GKC", 0, &sim.objects)
            .unwrap();
        let mut wick = Vec::new();
        for y in 330..350 {
            wick.extend_from_slice(&frame.rgba[(y * 640 + 148) * 4..(y * 640 + 174) * 4]);
        }
        if let Some(old) = previous {
            assert_ne!(wick, old, "wick composite did not animate at state {state}");
        }
        previous = Some(wick);
    }
    sim.objects[cannon].state = 8;
    sim.objects[cannon].power_state = 1;
    let count = sim.objects.len();
    process_cannon(&mut sim.objects, cannon);
    let projectile = &sim.objects[count];
    assert_eq!((projectile.width, projectile.height), (24, 23));
    assert_eq!(projectile.update_flags, 0x0810);
    assert_eq!(projectile.flags, 8);
}

#[test]
fn focused_magnifier_render_adds_native_yellow_cone() {
    let root = crate::test_support::game_data_root();
    let resources = crate::resources::Resources::load(root).unwrap();
    let mut sim = title();
    let lens = sim
        .objects
        .iter()
        .position(|p| p.definition == Def::MagnifyingGlass)
        .unwrap();
    let cannon = sim
        .objects
        .iter()
        .position(|p| p.definition == Def::Cannon)
        .unwrap();
    let before =
        crate::machine::live_collection_frame(&resources, "TITLE.GKC", 0, &sim.objects).unwrap();
    sim.objects[lens].linked_objects[4] = Some(cannon);
    let after =
        crate::machine::live_collection_frame(&resources, "TITLE.GKC", 0, &sim.objects).unwrap();
    let hidden = crate::machine::live_collection_frame_with_magnifier_ray(
        &resources,
        "TITLE.GKC",
        0,
        &sim.objects,
        false,
    )
    .unwrap();
    let color = crate::resources::palette_color(&resources, "TIM.PAL", 14).unwrap();
    let count_color = |rgba: &[u8]| rgba.chunks_exact(4).filter(|pixel| *pixel == color).count();
    assert!(count_color(&after.rgba) > count_color(&before.rgba));
    assert_eq!(hidden.rgba, before.rgba);
}

#[test]
fn title_platforms_use_native_tile_flag_and_glove_has_collision_geometry() {
    let sim = title();
    let floors: Vec<_> = sim
        .objects
        .iter()
        .filter(|p| matches!(p.definition, Def::BrickWall | Def::WoodWall))
        .collect();
    assert!(!floors.is_empty());
    assert!(floors.iter().any(|p| p.update_flags & 0x40 != 0));
    let glove = sim
        .objects
        .iter()
        .find(|p| p.definition == Def::BoxingGlove)
        .unwrap();
    assert!(glove.outline.is_some());
}

#[test]
fn puzzle29_matches_recorded_dos_states_through_240_updates() {
    let root = crate::test_support::game_data_root();
    let resources = crate::resources::Resources::load(root).unwrap();
    let mut session = crate::puzzle::PuzzleSession::load(&resources, 29, 0).unwrap();
    session
        .restore(include_bytes!("../test_data/puzzle29/TITLE.GKC"))
        .unwrap();
    session.start();
    let sim = &mut session.run.as_mut().unwrap().simulation;
    // Frozen observations from the original DOS executable, not an
    // expected trace generated by this implementation.
    let mut lines = include_str!("../test_data/puzzle29/native.csv").lines();
    let columns: Vec<_> = lines.next().unwrap().split(',').collect();
    for tick in 0..=240 {
        let mut seen = std::collections::HashSet::new();
        for _ in 0..40 {
            let row: Vec<_> = lines.next().unwrap().split(',').collect();
            let value = |field: &str| row[columns.iter().position(|&c| c == field).unwrap()];
            assert_eq!(value("tick").parse::<usize>().unwrap(), tick);
            let id = value("id").parse::<usize>().unwrap();
            assert!(seen.insert(id));
            let o = &sim.objects[id];
            assert_eq!(value("list"), format!("{:?}", o.list));
            let fields = [
                ("definition", i64::from(o.definition.id())),
                ("x", i64::from(o.x)),
                ("y", i64::from(o.y)),
                ("position_x", i64::from(o.motion.position_x)),
                ("position_y", i64::from(o.motion.position_y)),
                ("velocity_x", i64::from(o.motion.velocity_x)),
                ("velocity_y", i64::from(o.motion.velocity_y)),
                ("state", i64::from(o.state)),
                ("power", i64::from(o.power_state)),
                ("activation_timer", i64::from(o.activation_timer)),
                ("callback_timer", i64::from(o.callback_timer)),
                ("update_flags", i64::from(o.update_flags)),
                ("render_flags", i64::from(o.render_flags.bits())),
                ("contact", o.contact.map_or(-1, |c| c.target as i64)),
                ("effective_mass", i64::from(o.effective_mass)),
                ("flags", i64::from(o.flags)),
                ("width", i64::from(o.width)),
                ("height", i64::from(o.height)),
            ];
            for (field, actual) in fields {
                assert_eq!(
                    actual,
                    value(field).parse::<i64>().unwrap(),
                    "DOS mismatch at tick {tick}, object {id} {:?}, {field}",
                    o.definition
                );
            }
        }
        if tick < 240 {
            sim.step().unwrap();
        }
    }
    assert!(lines.next().is_none());
}

#[test]
fn hidden_moving_parts_reset_bucket_load_without_accelerating() {
    let (objects, _) = fan_fixture();
    let mut dynamite = objects[0].clone();
    dynamite.definition = Def::Dynamite;
    dynamite.list = ObjectList::Moving;
    dynamite.state = 5;
    dynamite.render_flags = crate::machine::RenderFlag::Hidden.mask();
    dynamite.flags = PropertyFlag::Ignitable.mask()
        | PropertyFlag::FreePlacement.mask()
        | PropertyFlag::CarriedByBucket.mask();
    dynamite.effective_mass = 1234;
    dynamite.motion.velocity_y = 123;
    dynamite.collision_impulse = 456;
    dynamite.ropes = [None, None];
    dynamite.linked_objects = [None; 6];
    let mut fixed = dynamite.clone();
    fixed.list = ObjectList::Fixed;
    let initial_motion = dynamite.motion.clone();
    let mut sim =
        MachineSimulation::from_objects(vec![dynamite, fixed], crate::forces::initialize(0, 0));

    sim.step().unwrap();

    let dynamite = &sim.objects[0];
    assert_eq!(dynamite.flags, 0x0c);
    assert_eq!(dynamite.effective_mass, crate::forces::mass(Def::Dynamite));
    assert_eq!(dynamite.motion, initial_motion);
    assert_eq!(dynamite.collision_impulse, 456);
    assert_eq!(sim.objects[1].flags, 0x1c);
    assert_eq!(sim.objects[1].effective_mass, 1234);
}

#[test]
fn bucket_load_strict_bounds_mass_cap_and_velocity_transfer() {
    let (mut objects, _) = fan_fixture();
    objects[0].definition = Def::Bucket;
    objects[0].list = ObjectList::Moving;
    objects[0].x = 0;
    objects[0].y = 0;
    objects[0].height = 48;
    objects[0].previous_position = (0, 0);
    objects[0].extent_override = Some((36, 48));
    objects[0].effective_mass = 31900;
    objects[0].motion.velocity_y = 123;
    objects[1].x = 0;
    objects[1].y = 0;
    objects[1].width = 32;
    objects[1].height = 32;
    objects[1].previous_position = (0, 0);
    objects[1].extent_override = Some((32, 32));
    objects[1].list = ObjectList::Moving;
    objects[1].effective_mass = 200;
    let contents = collect_bucket_load(&mut objects, 0);
    assert_eq!(contents, vec![1]);
    accumulate_bucket_mass(&mut objects, 0, &contents);
    assert_eq!(objects[0].effective_mass, 32000);
    assert_eq!(objects[1].motion.velocity_y, 123);
    assert_ne!(objects[1].flags & 0x10, 0);
    objects[0].effective_mass = 300;
    collect_bucket_load(&mut objects, 0);
    assert_eq!(
        objects[0].effective_mass, 300,
        "recollection must not add mass"
    );
    objects[0].previous_position = (0, 0);
    objects[0].x = 5;
    objects[0].y = -3;
    move_bucket_contents(&mut objects, 0, &contents);
    assert_eq!((objects[1].x, objects[1].y), (5, -3));
    assert_eq!(objects[1].motion.pixels(), (5, -3));
    objects[0].x = 0;
    objects[0].y = 0;
    objects[0].effective_mass = 100;
    objects[1].flags &= !0x10;
    objects[1].x = -12; // center exactly 4
    objects[1].previous_position = (-12, 0);
    collect_bucket_load(&mut objects, 0);
    assert_eq!(objects[0].effective_mass, 100);
    assert_eq!(objects[1].flags & 0x10, 0);
    assert_eq!(add_effective_mass(-1, 2), 1);
    assert_eq!(add_effective_mass(32000, 100), 32000);
}

#[test]
fn priority_phase_dispatches_before_normal_pass_and_skips_processed_duplicates() {
    let mut sim = title();
    let mut part = solar_fixture(Def::Bellows, 0).remove(1);
    part.list = ObjectList::Fixed;
    part.state = 0;
    part.power_state = 1;
    part.render_flags = crate::machine::RenderFlag::PowerUpdateCallback.mask();
    part.ropes = [None, None];
    part.linked_objects = [None; 6];
    let mut objects = vec![part];
    dispatch_fixed_callbacks_with_priorities(&mut objects, &sim.forces, 0, &[(0, 200), (0, 100)])
        .unwrap();
    assert_eq!(objects[0].state, 1);
    assert!(
        objects[0]
            .render_flags
            .contains(crate::machine::RenderFlag::ProcessedThisUpdate)
    );
    sim.objects = objects;
    sim.impulse_priority.0 = vec![(0, 200)];
    sim.step().unwrap();
    assert_eq!(sim.objects[0].state, 2);
    assert!(sim.impulse_priority.0.is_empty());
}

#[test]
fn impulse_priority_rejects_ties_and_weaker_contacts_per_target() {
    let mut priority = ImpulsePriority::default();
    assert!(priority.accept(1, 100));
    assert!(!priority.accept(1, 100));
    assert!(!priority.accept(1, 99));
    assert!(priority.accept(2, -10));
    assert!(priority.accept(1, 101));
    assert_eq!(priority.0, vec![(1, 101), (1, 100), (2, -10)]);
    priority.0.clear();
    assert!(priority.accept(1, 99));
}

#[test]
fn seesaw_contact_edges_pivot_limits_and_blocked_state() {
    let mut sim = title();
    let part = sim
        .objects
        .iter_mut()
        .find(|p| p.definition == Def::Seesaw)
        .unwrap();
    part.x = 0;
    part.state = 1;
    part.render_flags = crate::machine::RenderFlags::EMPTY;
    for (center, direction) in [(36, Some(-1)), (37, None), (43, None), (44, Some(1))] {
        assert_eq!(seesaw_contact_direction(part, 0, center), direction);
    }
    assert_eq!(seesaw_contact_direction(part, 2, 40), Some(-1));
    assert_eq!(seesaw_contact_direction(part, 6, 40), Some(1));
    for edge in [1, 3, 4, 5, 7] {
        assert_eq!(seesaw_contact_direction(part, edge, 0), None);
    }
    part.state = 0;
    assert_eq!(seesaw_contact_direction(part, 2, 0), None);
    part.state = 2;
    assert_eq!(seesaw_contact_direction(part, 6, 80), None);
    part.state = 1;
    part.render_flags = crate::machine::RenderFlag::CallbackBlocked.mask();
    assert_eq!(seesaw_contact_direction(part, 0, 0), None);
}

#[test]
fn seesaw_throw_mass_thresholds_and_segment_boundaries() {
    for (mass, speed) in [
        (0, 0x1c00),
        (1, 0x1c00),
        (2, 0x1a00),
        (5, 0x1a00),
        (6, 0x1800),
        (9, 0x1800),
        (10, 0x1600),
        (20, 0x1600),
        (21, 0x1400),
        (120, 0x1400),
        (121, 0x1200),
        (150, 0x1200),
        (151, 0x1000),
        (32767, 0x1000),
        (-1, 0x1c00),
    ] {
        assert_eq!(seesaw_throw_speed_for_mass(mass), speed);
    }
    assert!(segments_cross((0, 0), (10, 0), (5, -1), (5, 1)));
    assert!(segments_cross((0, 0), (10, 0), (10, 0), (12, 2)));
    assert!(segments_cross((0, 0), (10, 0), (5, 0), (15, 0)));
    assert!(!segments_cross((0, 0), (10, 0), (11, 0), (15, 0)));
    assert!(!segments_cross((0, 0), (10, 0), (0, 1), (10, 1)));
}

#[test]
fn seesaw_throw_assigns_native_mass_speed_on_both_sides() {
    let (mut objects, _) = fan_fixture();
    objects[0].definition = Def::Seesaw;
    objects[0].x = 0;
    objects[0].y = 0;
    objects[0].width = 80;
    objects[0].state = 1;
    objects[1].x = 4;
    objects[1].y = 0;
    objects[1].list = ObjectList::Moving;
    objects[1].outline = Some(vec![(0, 0), (20, 0), (20, 32), (0, 32)]);
    let speed = seesaw_throw_speed(objects[1].definition);
    for (x, power, vy) in [
        (4, -1, speed),
        (4, 1, -speed),
        (50, -1, -speed),
        (50, 1, speed),
    ] {
        objects[1].x = x;
        throw_seesaw_crossing_parts(&mut objects, 0, power);
        assert_eq!(objects[1].motion.velocity_y, vy);
        assert_eq!(
            objects[1].motion.velocity_x,
            if x < 40 { -(speed >> 2) } else { speed >> 2 }
        );
    }
    objects[1].y = 40;
    objects[1].motion.velocity_y = 123;
    throw_seesaw_crossing_parts(&mut objects, 0, 1);
    assert_eq!(objects[1].motion.velocity_y, 123);
}

#[test]
fn seesaw_uses_native_attachment_table_after_each_tilt() {
    assert_eq!(crate::outline::seesaw_anchors(0), Some([(5, 27), (73, 3)]));
    assert_eq!(crate::outline::seesaw_anchors(1), Some([(4, 2), (75, 2)]));
    assert_eq!(crate::outline::seesaw_anchors(2), Some([(6, 3), (74, 27)]));
    let mut sim = title();
    let i = sim
        .objects
        .iter()
        .position(|p| p.definition == Def::Seesaw)
        .unwrap();
    sim.objects[i]
        .render_flags
        .insert(crate::machine::RenderFlag::RopeProbeBypassed); // Native bypass of rope probe.
    for (state, power, next) in [(2, -1, 1), (1, -1, 0), (0, 1, 1), (1, 1, 2)] {
        sim.objects[i].state = state;
        sim.objects[i].power_state = power;
        apply_seesaw_tilt_and_throw_parts(&mut sim.objects, i);
        assert_eq!(sim.objects[i].state, next);
        assert_eq!(
            sim.objects[i].rope_anchors,
            crate::outline::seesaw_anchors(next).unwrap()
        );
    }
}

#[test]
fn title_one_pulley_route_powers_light_and_blocks_a_second_pull() {
    let mut sim = title();
    let seesaw = sim
        .objects
        .iter()
        .position(|p| p.definition == Def::Seesaw)
        .unwrap();
    let light = sim
        .objects
        .iter()
        .position(|p| p.definition == Def::Light)
        .unwrap();
    sim.objects[seesaw].power_state = -1;
    assert!(!DispatchConnectedRopeContact(
        &mut sim.objects,
        seesaw,
        0x8000
    ));
    assert_eq!(sim.objects[light].power_state, 0);
    apply_seesaw_tilt_and_throw_parts(&mut sim.objects, seesaw);
    assert_eq!(sim.objects[light].power_state, 1);
    let state = sim.objects[seesaw].state;
    sim.objects[seesaw].power_state = -1;
    apply_seesaw_tilt_and_throw_parts(&mut sim.objects, seesaw);
    assert_eq!(sim.objects[seesaw].state, state);
    assert_eq!(sim.objects[seesaw].power_state, 0);
}

#[test]
fn rope_ends_follow_the_drawn_frame_of_a_flat_seesaw() {
    // Puzzle 10: the flat seesaw frame is drawn 12 pixels down, and the
    // knots (rebuild_rope_endpoint_coordinates) are measured from it.
    let root = crate::test_support::game_data_root();
    let resources = crate::resources::Resources::load(root).unwrap();
    let mut seesaw = crate::machine::collection_objects(&resources, "L10.LEV")
        .unwrap()
        .into_iter()
        .find(|o| o.definition == Def::Seesaw)
        .unwrap();
    seesaw.state = 1;
    seesaw.rope_anchors = crate::outline::seesaw_anchors(1).unwrap();
    let (x, y) = (i32::from(seesaw.x), i32::from(seesaw.y));
    assert_eq!(crate::machine::rope_point(&seesaw, 0), (x + 4, y + 12 + 2));
    assert_eq!(crate::machine::rope_point(&seesaw, 1), (x + 75, y + 12 + 2));
    // Tilted frames are drawn in place.
    seesaw.state = 0;
    seesaw.rope_anchors = crate::outline::seesaw_anchors(0).unwrap();
    assert_eq!(crate::machine::rope_point(&seesaw, 0), (x + 5, y + 27));
}

#[test]
fn a_popped_balloon_on_a_separate_rope_leaves_a_falling_rope_end() {
    // Puzzle 10's balloon is tied to an eye hook by a Rope part; the
    // balloon itself holds no rope record.
    let root = crate::test_support::game_data_root();
    let resources = crate::resources::Resources::load(root).unwrap();
    let mut sim = MachineSimulation::load_collection(&resources, "L10.LEV").unwrap();
    let balloon = sim
        .objects
        .iter()
        .position(|o| o.definition == Def::Balloon)
        .unwrap();
    let rope = sim
        .objects
        .iter()
        .position(|o| {
            o.ropes[0]
                .as_ref()
                .is_some_and(|c| c.endpoints.contains(&Some(balloon)))
        })
        .unwrap();
    assert!(sim.objects[balloon].ropes[0].is_none());
    sim.objects[balloon].power_state = 1;
    sim.step().unwrap();
    let end = sim
        .objects
        .iter()
        .position(|o| o.definition == Def::RopeEnd)
        .expect("no rope end");
    let connection = sim.objects[rope].ropes[0].clone().unwrap();
    assert!(connection.endpoints.contains(&Some(end)));
    assert!(!connection.endpoints.contains(&Some(balloon)));
    // The end holds no second copy of the rope, and it falls.
    assert!(sim.objects[end].ropes[0].is_none());
    let y = sim.objects[end].y;
    for _ in 0..20 {
        sim.step().unwrap();
    }
    assert!(sim.objects[end].y > y, "rope end did not fall");
}

#[test]
fn bellows_push_follows_the_native_shift_and_never_flips_light_parts() {
    // Puzzle 8: a baseball (mass 9) on the handle, one pixel into the
    // blast, used to overflow the 16-bit velocity and fly backwards.
    for (strength, expected) in [
        (0x800, (0x800 * 255 >> 8) / 9),
        (-0x800, (-0x800 * 255 >> 8) / 9),
    ] {
        let impulse = air_impulse(strength, 1, Def::Baseball);
        assert_eq!(impulse, expected);
        assert_eq!(impulse.signum(), strength.signum());
    }
    for definition in (0..Def::COUNT as u16).map(|id| Def::try_from(id).unwrap()) {
        for distance in [0, 1, 128, 255] {
            let impulse = air_impulse(0x1000, distance, definition);
            assert!(
                (0..=i32::from(i16::MAX)).contains(&impulse),
                "{definition:?} {distance}"
            );
        }
    }
}

#[test]
fn bellows_contact_selects_only_native_facing_edges_and_keeps_bounce() {
    let mut part = solar_fixture(Def::Bellows, 0).remove(1);
    for flipped in [false, true] {
        part.render_flags = if flipped {
            crate::machine::RenderFlag::FlipHorizontal.mask()
        } else {
            RenderFlags::EMPTY
        };
        for edge in 0..6 {
            part.power_state = -1;
            assert_eq!(
                bellows_contact(&mut part, edge),
                CollisionResponse::Ordinary
            );
            let active = if flipped {
                edge == 1 || edge == 3
            } else {
                edge == 0 || edge == 4
            };
            assert_eq!(part.power_state, if active { 1 } else { -1 });
        }
    }
}

#[test]
fn bellows_pressure_compression_return_and_facing() {
    for flipped in [false, true] {
        let (mut objects, forces) = fan_fixture();
        objects[0].definition = Def::Bellows;
        objects[0].render_flags = if flipped {
            crate::machine::RenderFlag::FlipHorizontal.mask()
        } else {
            RenderFlags::EMPTY
        };
        objects[0].state = 0;
        objects[0].previous_state = 0;
        objects[1].x = if flipped { -132 } else { 132 };
        objects[1].motion = crate::motion::Motion::at_pixels(objects[1].x, 0);
        objects[1].update_flags = 0x1000;
        process_bellows(&mut objects, 0, &forces);
        assert_eq!(objects[0].state, 1);
        assert_eq!(objects[0].outline.as_ref().unwrap().len(), 6);
        assert_eq!(
            objects[1].motion.velocity_x.signum(),
            if flipped { -1 } else { 1 }
        );
        objects[1].definition = Def::Windmill;
        objects[1].update_flags = 0x2000;
        process_bellows(&mut objects, 0, &forces);
        assert_eq!(objects[0].state, 2);
        assert_eq!(objects[1].callback_timer, 20);
        objects[1].callback_timer = 0;
        process_bellows(&mut objects, 0, &forces);
        assert_eq!(objects[1].callback_timer, 0);
        objects[0].power_state = -1;
        process_bellows(&mut objects, 0, &forces);
        assert_eq!(objects[0].state, 1);
        process_bellows(&mut objects, 0, &forces);
        assert_eq!(objects[0].state, 0);
        process_bellows(&mut objects, 0, &forces);
        assert_eq!(objects[0].state, 0);
        objects[0].power_state = 1;
        objects[1].definition = Def::Candle;
        objects[1].update_flags = 0x1000;
        objects[1].state = 4;
        objects[1].power_state = 1;
        objects[1].callback_timer = 30;
        process_bellows(&mut objects, 0, &forces);
        assert_eq!(
            (
                objects[1].state,
                objects[1].power_state,
                objects[1].callback_timer
            ),
            (0, 0, 0)
        );
    }
}

#[test]
fn machine_collision_dispatches_trampoline_launch_and_ignores_disabled_spring() {
    for disabled in [false, true] {
        let mut sim = title();
        let mut spring = solar_fixture(Def::Trampoline, 0).remove(1);
        spring.list = ObjectList::Fixed;
        spring.width = 48;
        spring.height = 28;
        spring.state = 2;
        spring.power_state = 1;
        spring.render_flags = if disabled {
            crate::machine::RenderFlag::Hidden.mask()
        } else {
            RenderFlags::EMPTY
        };
        spring.update_flags = 0;
        spring.linked_objects = [None; 6];
        spring.ropes = [None, None];
        spring.belt = None;
        spring.outline = crate::outline::vertices(&spring);
        spring.outline_angles = crate::outline::edge_angles(spring.outline.as_deref().unwrap());
        let mut ball = sim
            .objects
            .iter()
            .find(|o| o.definition == Def::BowlingBall)
            .unwrap()
            .clone();
        ball.list = ObjectList::Moving;
        ball.render_flags = crate::machine::RenderFlags::EMPTY;
        ball.width = 32;
        ball.height = 32;
        ball.x = 8;
        ball.y = -20;
        ball.outline = crate::outline::vertices(&ball);
        ball.motion = crate::motion::Motion::at_pixels(8, -20);
        ball.motion.velocity_y = 512;
        ball.ropes = [None, None];
        ball.belt = None;
        ball.linked_objects = [None; 6];
        let force = &mut sim.forces[Def::BowlingBall.id() as usize];
        force.acceleration = 0;
        force.speed_limit = 0x3000;
        sim.objects = vec![spring, ball];
        sim.step().unwrap();
        assert_eq!(
            sim.objects[1].motion.velocity_y,
            if disabled { 512 } else { -1536 }
        );
        assert_eq!(sim.objects[1].contact, None);
        assert_eq!(sim.objects[0].state, if disabled { 2 } else { 3 });
    }
}

#[test]
fn trampoline_contact_strict_center_edge_and_launch() {
    let mut objects = solar_fixture(Def::Trampoline, 0);
    objects[0].x = 0;
    objects[0].width = 32;
    objects[0].motion = crate::motion::Motion::at_pixels(0, 0);
    objects[1].x = -8;
    objects[1].width = 48;
    // `1000:3ebf` compares object centers using frame extents (+0x44),
    // which differ from these fixture objects' persistent body sizes.
    for (gap, accepted) in [(-35, false), (-34, true), (-6, true), (-5, false)] {
        objects[0].motion.position_x = gap * 512;
        objects[0].x = gap as i16;
        objects[1].power_state = 0;
        assert_eq!(
            apply_trampoline_bounce(&mut objects, 0, 1, 0, 0x3000),
            accepted
        );
        assert_eq!(objects[1].power_state, i16::from(accepted));
    }
    objects[0].motion.position_x = 0;
    objects[0].x = 0;
    objects[1].x = 12;
    objects[1].motion = crate::motion::Motion::at_pixels(12, 0);
    assert!(!apply_trampoline_bounce(&mut objects, 0, 1, 1, 0x3000));
    objects[1].state = 3;
    objects[0].contact = Some(crate::machine::ContactRecord {
        target: 1,
        outline_edge: 0,
        angle: 0,
        flag_a: true,
        flag_b: true,
    });
    objects[0].update_flags = 0xa501;
    objects[0].motion.velocity_x = -1025;
    objects[0].motion.velocity_y = -512;
    assert!(apply_trampoline_bounce(&mut objects, 0, 1, 0, 0x3000));
    assert_eq!(objects[0].contact, None);
    assert_eq!(objects[0].update_flags, 0xa500);
    assert_eq!(objects[0].motion.velocity_x, -513);
    assert_eq!(objects[0].motion.velocity_y, -1536);
    objects[0].motion.velocity_x = 1023;
    objects[0].motion.velocity_y = 3000;
    apply_trampoline_bounce(&mut objects, 0, 1, 0, 3500);
    assert_eq!(objects[0].motion.velocity_x, 1023);
    assert_eq!(objects[0].motion.velocity_y, -3500);
}

#[test]
fn trampoline_spring_cycle_resets_and_idle_state_does_not_advance() {
    let mut objects = solar_fixture(Def::Trampoline, 0);
    objects[1].state = 0;
    objects[1].power_state = 0;
    update_trampoline_bounce(&mut objects[1]);
    assert_eq!(objects[1].state, 0);
    objects[1].power_state = 1;
    for state in [1, 2, 3, 4, 0] {
        update_trampoline_bounce(&mut objects[1]);
        assert_eq!(objects[1].state, state);
    }
    assert_eq!(objects[1].power_state, 0);
    assert_eq!(
        crate::outline::vertices(&objects[1]),
        Some(vec![(0, 11), (47, 11), (47, 27), (0, 27)])
    );
}

fn fan_fixture() -> (Vec<Object>, [crate::forces::PartForce; 58]) {
    let sim = title();
    let mut fan = sim
        .objects
        .iter()
        .find(|o| o.definition == Def::Fan)
        .unwrap()
        .clone();
    fan.x = 0;
    fan.y = 0;
    fan.width = 32;
    fan.height = 32;
    fan.power_state = 1;
    fan.state = 0;
    fan.render_flags = crate::machine::RenderFlags::EMPTY;
    let mut ball = sim
        .objects
        .iter()
        .find(|o| o.definition == Def::BowlingBall)
        .unwrap()
        .clone();
    ball.x = 132;
    ball.y = 0;
    ball.width = 32;
    ball.height = 32;
    ball.motion = crate::motion::Motion::at_pixels(ball.x, ball.y);
    ball.render_flags = crate::machine::RenderFlags::EMPTY;
    ball.update_flags = 0;
    (vec![fan, ball], sim.forces)
}

#[test]
fn fan_uses_distance_and_mass_and_only_reaches_its_facing_region() {
    let (mut objects, forces) = fan_fixture();
    let mut behind = objects[1].clone();
    behind.x = -100;
    let mut below = objects[1].clone();
    below.y = 33;
    let mut distant = objects[1].clone();
    distant.x = 289;
    let mut disabled = objects[1].clone();
    disabled.render_flags = crate::machine::RenderFlag::Hidden.mask();
    let mut inventory = objects[1].clone();
    inventory.list = ObjectList::Inventory;
    objects.extend([behind, below, distant, disabled, inventory]);
    update_fan_airflow(&mut objects, 0, &forces);
    assert_eq!(
        objects[1].motion.velocity_x,
        ((0x1000 * 156 >> 8) / 200) as i16
    );
    for object in &objects[2..] {
        assert_eq!(object.motion.velocity_x, 0);
    }
    assert_eq!(objects[0].state, 1);
}

#[test]
fn flipped_fan_pushes_left_and_caps_velocity() {
    let (mut objects, forces) = fan_fixture();
    objects[0].render_flags = crate::machine::RenderFlag::FlipHorizontal.mask();
    objects[0].state = 3;
    objects[1].x = -132; // right edge is 100 pixels left of fan origin
    objects[1].definition = Def::Balloon; // mass 1, saturates the native limit
    update_fan_airflow(&mut objects, 0, &forces);
    let expected = (-0x1000_i32 * 156 >> 8) as i16;
    assert_eq!(
        objects[1].motion.velocity_x,
        expected.clamp(-forces[4].speed_limit, forces[4].speed_limit)
    );
    assert_eq!(objects[0].state, 0);
}

#[test]
fn fan_windmill_activation_has_strict_200_pixel_boundary() {
    let (mut objects, forces) = fan_fixture();
    objects[1].definition = Def::Windmill;
    objects[1].list = ObjectList::Fixed;
    objects[1].update_flags = 0x2000;
    objects[1].x = 231; // nearest gap 199
    objects[1].power_state = 0;
    let mut outside = objects[1].clone();
    outside.x = 232;
    objects.push(outside);
    update_fan_airflow(&mut objects, 0, &forces);
    assert_eq!(objects[1].power_state, 1);
    assert_eq!(objects[1].callback_timer, 20);
    assert_eq!(objects[2].power_state, 0);
    assert_eq!(objects[1].motion.velocity_x, 0);
}

#[test]
fn unpowered_fan_leaves_animation_and_targets_unchanged() {
    let (mut objects, forces) = fan_fixture();
    objects[0].power_state = 0;
    update_fan_airflow(&mut objects, 0, &forces);
    assert_eq!(objects[0].state, 0);
    assert_eq!(objects[1].motion.velocity_x, 0);
}

#[test]
fn fan_query_includes_exact_range_edges_and_preserves_overlap_gap() {
    let (mut objects, _) = fan_fixture();
    objects[1].x = 288;
    objects[1].y = -42;
    let contacts = crate::spatial::nearby_parts(&objects, 0, (0, 256), (-10, 0));
    assert_eq!(contacts.len(), 1);
    assert_eq!(contacts[0].horizontal_gap, 256);
    assert_eq!(contacts[0].vertical_gap, -10);
    objects[1].y = -43;
    assert!(crate::spatial::nearby_parts(&objects, 0, (0, 256), (-10, 0)).is_empty());
    objects[1].x = 0;
    objects[1].y = 0;
    let contacts = crate::spatial::nearby_parts(&objects, 0, (0, 256), (-10, 0));
    assert_eq!(contacts[0].horizontal_gap, -1);
    assert_eq!(contacts[0].vertical_gap, -1);
}

#[test]
fn scissors_activation_preserves_saved_facing() {
    let sim = title();
    let template = sim
        .objects
        .iter()
        .find(|o| o.definition == Def::Scissors)
        .unwrap();
    for facing in [0, 0x10] {
        let mut objects = vec![template.clone()];
        objects[0].state = 0;
        objects[0].power_state = 1;
        objects[0].render_flags = facing.into();
        update_scissors_snap(&mut objects, 0).unwrap();
        assert_eq!(objects[0].state, 1);
        assert_eq!(
            objects[0]
                .render_flags
                .contains(crate::machine::RenderFlag::FlipHorizontal),
            facing != 0
        );
        update_scissors_snap(&mut objects, 0).unwrap();
        assert_eq!(objects[0].state, 1);
    }
}

#[test]
fn powered_pass_uses_the_same_native_update_dispatcher() {
    let mut sim = title();
    let index = sim
        .objects
        .iter()
        .position(|o| o.definition == Def::Motor)
        .unwrap();
    sim.objects[index].state = 0;
    sim.objects[index].power_state = 1;
    sim.objects[index].belt = None;
    // Remove belt records to isolate the motor's powered-pass animation.
    for object in &mut sim.objects {
        object.belt = None;
    }
    sim.objects[index].render_flags = crate::machine::RenderFlag::PowerUpdateCallback.mask();
    dispatch_fixed_callbacks(&mut sim.objects, &sim.forces, 0).unwrap();
    assert_eq!(sim.objects[index].state, 1);
}

#[test]
fn fixed_callbacks_skip_disabled_parts() {
    let sim = title();
    let first = sim
        .objects
        .iter()
        .find(|o| o.definition == Def::Seesaw)
        .unwrap()
        .clone();
    let mut second = first.clone();
    let mut first = first;
    // A disabled fixed part retains its state and impulse.
    first.ropes = [None, None];
    first.state = 0;
    first.power_state = 1;
    first.render_flags = crate::machine::RenderFlag::RopeProbeBypassed.mask();
    second.ropes = [None, None];
    second.state = 0;
    second.power_state = 1;
    second.render_flags = crate::machine::RenderFlag::RopeProbeBypassed.mask()
        | crate::machine::RenderFlag::Hidden.mask();
    let mut objects = vec![first, second];
    dispatch_fixed_callbacks(&mut objects, &sim.forces, 0).unwrap();
    assert_eq!(objects[0].state, 1);
    assert_eq!(objects[1].state, 0);
    assert_eq!(objects[1].power_state, 1);
}

#[test]
fn light_probe_is_read_only_and_real_contact_lights_saved_state() {
    let mut sim = title();
    let light = sim
        .objects
        .iter_mut()
        .find(|o| o.definition == Def::Light)
        .unwrap();
    let origin = (light.x, light.y);
    light.state = 2;
    light.power_state = 0;
    SetLightRopeAnchorForState(light);
    assert!(!process_light_rope_contact(light, 0x8004));
    assert_eq!(light.power_state, 0);
    assert_eq!(light.state, 2);
    assert!(!process_light_rope_contact(light, 4));
    assert_eq!(light.power_state, 1);
    illuminate_light_on_power(std::slice::from_mut(light), 0);
    assert_eq!(light.state, 3);
    assert_eq!(light.rope_anchors[0], (28, 35));
    assert_eq!((light.x, light.y), origin);
    assert!(process_light_rope_contact(light, 4));
}

#[test]
fn blocked_rope_callback_prevents_seesaw_tilt_and_consumes_impulse() {
    let mut sim = title();
    let index = sim
        .objects
        .iter()
        .position(|o| o.definition == Def::Seesaw)
        .unwrap();
    sim.objects[index].state = 1;
    sim.objects[index].power_state = 1;
    sim.objects[index]
        .render_flags
        .insert(crate::machine::RenderFlag::CallbackBlocked);
    apply_seesaw_tilt_and_throw_parts(&mut sim.objects, index);
    assert_eq!(sim.objects[index].state, 1);
    assert_eq!(sim.objects[index].power_state, 0);
}

#[test]
fn simulation_step_dispatches_seesaw_and_consumes_middle_state_impulse() {
    let mut sim = title();
    let index = sim
        .objects
        .iter()
        .position(|o| o.definition == Def::Seesaw)
        .unwrap();
    sim.objects[index].state = 0;
    sim.objects[index].power_state = 1;
    sim.objects[index]
        .render_flags
        .insert(crate::machine::RenderFlag::RopeProbeBypassed);
    for object in &mut sim.objects {
        if object.list == ObjectList::Moving {
            object
                .render_flags
                .insert(crate::machine::RenderFlag::Hidden);
        }
    }
    let index = sim
        .objects
        .iter()
        .position(|o| o.definition == Def::Seesaw)
        .unwrap();
    sim.step().unwrap();
    assert_eq!(sim.objects[index].state, 1);
    assert_eq!(sim.objects[index].power_state, 0);
    assert!(
        sim.objects[index]
            .render_flags
            .contains(crate::machine::RenderFlag::ProcessedThisUpdate)
    );
}
