use super::*;
use crate::sound::SoundEffect;

const FRAME_FIRE_SOUND: u16 = 2;
const FRAME_FIRE_BULLET: u16 = 3;
const FRAME_SPENT: u16 = 6;
const LEFT_MUZZLE_X_OFFSET: i16 = -32;
const RIGHT_MUZZLE_X_OFFSET: i16 = 36;
const MUZZLE_Y_OFFSET: i16 = 3;
const LAUNCH_SWEEP_PIXELS: i16 = 24;
const BULLET_WIDTH: u16 = 40;
const BULLET_HEIGHT: u16 = 7;

/// compute_unknown27_contact_impulse, Ghidra 272c:2451: the gun's trigger
/// fires only when the rope pulls from the side facing away from the muzzle.
/// Flag value 1 denotes a loosening direction rather than a trigger pull.
pub(super) fn gun_rope_contact(
    objects: &mut [Object],
    gun_index: usize,
    contact_flags: u16,
) -> bool {
    if contact_flags == RopeContactFlag::LocalDirectionMatch.mask() {
        // compute_unknown27_contact_impulse (272c:2451) increments the
        // attached type-10 record's +0x0e counter on a loosening event.
        if let Some(rope) = objects
            .iter_mut()
            .flat_map(|object| object.ropes.iter_mut().flatten())
            .find(|rope| rope.endpoints.contains(&Some(gun_index)))
        {
            rope.slack_count = rope.slack_count.wrapping_add(1);
        }
        return false;
    }
    let gun = &mut objects[gun_index];
    let side = contact_flags & RopeContactFlag::HORIZONTAL_SIDE_MASK;
    let (pulling, blocking) = if !gun
        .render_flags
        .contains(crate::machine::RenderFlag::FlipHorizontal)
    {
        (
            RopeContactFlag::RouteLeftOfTarget.mask(),
            RopeContactFlag::RouteAtOrRightOfTarget.mask(),
        )
    } else {
        (
            RopeContactFlag::RouteAtOrRightOfTarget.mask(),
            RopeContactFlag::RouteLeftOfTarget.mask(),
        )
    };
    if side == blocking || (side == pulling && gun.power_state != 0) {
        return true;
    }
    if contact_flags & RopeContactFlag::PROBE_SIDE_MASK == pulling && gun.power_state == 0 {
        gun.power_state = 1;
    }
    false
}

/// update_gun_launch, Ghidra 272c:22ae: the powered gun animates to
/// state 6 and fires a definition-20 bullet on state 3.
pub(super) fn update_gun_launch(
    objects: &mut Vec<Object>,
    index: usize,
    forces: &[crate::forces::PartForce; 58],
) {
    let gun = &mut objects[index];
    if gun.power_state == 0 || gun.state == FRAME_SPENT {
        return;
    }
    gun.state += 1;
    if gun.state == FRAME_FIRE_SOUND {
        crate::sound::play_effect(SoundEffect::Gunshot);
    }
    if gun.state != FRAME_FIRE_BULLET {
        return;
    }
    let flipped = gun
        .render_flags
        .contains(crate::machine::RenderFlag::FlipHorizontal);
    let (x, previous_x, velocity_x) = if flipped {
        (
            gun.x + LEFT_MUZZLE_X_OFFSET,
            gun.x + LEFT_MUZZLE_X_OFFSET + LAUNCH_SWEEP_PIXELS,
            -BULLET_LAUNCH_SPEED,
        )
    } else {
        (
            gun.x + RIGHT_MUZZLE_X_OFFSET,
            gun.x + RIGHT_MUZZLE_X_OFFSET - LAUNCH_SWEEP_PIXELS,
            BULLET_LAUNCH_SPEED,
        )
    };
    let y = gun.y + MUZZLE_Y_OFFSET;
    // Allocator defaults DS 2966 + 20*16: update 0x0800, flags 8, 40x7; +0x10 on insert.
    let mut bullet = Object {
        list: ObjectList::Moving,
        update_flags: UpdateFlag::InInventory.mask() | UpdateFlag::CreatedDuringSimulation.mask(),
        contact: None,
        definition: Def::Bullet,
        effective_mass: crate::forces::mass(Def::Bullet),
        collision_impulse: 0,
        motion: crate::motion::Motion::at_pixels(x, y),
        saved_position: (previous_x, y),
        previous_position: (previous_x, y),
        older_position: (previous_x, y),
        saved_state: 0,
        outline: None,
        outline_angles: Vec::new(),
        state: 0,
        previous_state: 0,
        older_state: 0,
        power_state: 0,
        callback_timer: 0,
        pending_light_power: 0,
        activation_timer: 0,
        belt_anchor: (0, 0),
        belt_size: 0,
        rope_anchors: [(0, 0); 2],
        belt: None,
        ropes: [None, None],
        linked_objects: [None; 6],
        next_part: None,
        pulley_connection: None,
        flags: PropertyFlag::FreePlacement.mask(),
        render_flags: if flipped {
            RenderFlag::FlipHorizontal.mask()
        } else {
            RenderFlags::EMPTY
        },
        x,
        y,
        width: BULLET_WIDTH,
        height: BULLET_HEIGHT,
        extent_override: None,
        alt_size: (0, 0),
    };
    bullet.motion.velocity_x = velocity_x;
    bullet
        .motion
        .limit_velocity(forces[Def::Bullet.id() as usize].speed_limit);
    update_object_outline(&mut bullet);
    objects.push(bullet);
}
