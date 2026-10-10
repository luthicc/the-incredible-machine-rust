use super::*;
use crate::sound::SoundEffect;

const FRAME_FUSE_PAUSE: u16 = 7;
const FRAME_FIRE_SOUND: u16 = 8;
const FRAME_FIRE_PROJECTILE: u16 = 9;
const FRAME_SPENT: u16 = 11;
const FUSE_PAUSE_TICKS: i16 = 3;
const RIGHT_MUZZLE_OFFSET: i16 = 97;
const LEFT_MUZZLE_OFFSET: i16 = -48;
const MUZZLE_Y_OFFSET: i16 = -7;
const LAUNCH_SWEEP_X_PIXELS: i16 = 24;
const LAUNCH_SWEEP_Y_PIXELS: i16 = 8;
const PROJECTILE_SIZE: (u16, u16) = (24, 23);
const PROJECTILE_HORIZONTAL_SPEED: i16 = 12288;
const PROJECTILE_VERTICAL_SPEED: i16 = -4096;

/// Scene rope trigger for the Cannon. The native cannon update callback
/// consumes power_state and runs advance_cannon_and_fire_projectile (272c:0a5d).
pub(super) fn activate_cannon_from_rope(cannon: &mut Object, contact_flags: u16) {
    if contact_flags & RopeContactFlag::ProbeOnly.mask() == 0
        && contact_flags & RopeContactFlag::REMOTE_DIRECTION_MASK
            != RopeContactFlag::RemoteDirectionMismatch.mask()
    {
        cannon.power_state = 1;
    }
}

/// 272c:0a5d starts and advances the cannon sequence, then creates part 43.
pub(super) fn process_cannon(objects: &mut Vec<Object>, index: usize) {
    let cannon = &mut objects[index];
    if cannon.power_state == 0 && cannon.callback_timer > IGNITION_HEAT_THRESHOLD {
        cannon.power_state = 1;
    }
    if cannon.power_state == 0 || cannon.state == FRAME_SPENT {
        return;
    }
    if cannon.state == FRAME_FUSE_PAUSE {
        cannon.callback_timer = cannon.callback_timer.saturating_add(1);
        if cannon.callback_timer > FUSE_PAUSE_TICKS {
            cannon.state += 1;
        }
    } else {
        cannon.state += 1;
    }
    if cannon.state == FRAME_FIRE_SOUND {
        crate::sound::play_effect(SoundEffect::CannonFire);
    }
    if cannon.state == FRAME_FIRE_PROJECTILE {
        let right_facing = !cannon
            .render_flags
            .contains(crate::machine::RenderFlag::FlipHorizontal);
        let x = cannon.x
            + if right_facing {
                RIGHT_MUZZLE_OFFSET
            } else {
                LEFT_MUZZLE_OFFSET
            };
        let y = cannon.y + MUZZLE_Y_OFFSET;
        // 272c:0a5d seeds both prior position pairs at the projectile's
        // muzzle: current x - 24 when firing right, +24 when firing left,
        // and current y + 8. The first 1000:0556 sweep uses that path.
        let launch_x = x + if right_facing {
            -LAUNCH_SWEEP_X_PIXELS
        } else {
            LAUNCH_SWEEP_X_PIXELS
        };
        let launch_y = y + LAUNCH_SWEEP_Y_PIXELS;
        // Native allocator table DS 2966 + 43*16, followed by update bit 0x10.
        let (width, height) = PROJECTILE_SIZE;
        let vx = if right_facing {
            PROJECTILE_HORIZONTAL_SPEED
        } else {
            -PROJECTILE_HORIZONTAL_SPEED
        };
        let mut projectile = Object {
            list: ObjectList::Moving,
            update_flags: UpdateFlag::InInventory.mask()
                | UpdateFlag::CreatedDuringSimulation.mask(),
            contact: None,
            definition: Def::CannonProjectile,
            effective_mass: crate::forces::mass(Def::CannonProjectile),
            collision_impulse: 0,
            motion: crate::motion::Motion::at_pixels(x, y),
            saved_position: (x, y),
            previous_position: (launch_x, launch_y),
            older_position: (launch_x, launch_y),
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
            render_flags: RenderFlags::EMPTY,
            x,
            y,
            width,
            height,
            extent_override: None,
            alt_size: (0, 0),
        };
        projectile.motion.velocity_x = vx;
        projectile.motion.velocity_y = PROJECTILE_VERTICAL_SPEED;
        projectile.outline = crate::outline::vertices(&projectile);
        projectile.outline_angles = projectile
            .outline
            .as_deref()
            .map(crate::outline::edge_angles)
            .unwrap_or_default();
        objects.push(projectile);
    }
}
