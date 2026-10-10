use super::*;

const FRAME_FIRE_EXPLOSION: u16 = 1;
const FRAME_PRESSED: u16 = 2;
const HEAD_HEIGHT_PIXELS: i16 = 12;
const CONTACT_ANGLE_BIAS: u16 = 2048;
const CONTACT_ANGLE_LIMIT: i16 = 4096;
const EXPLOSION_X_OFFSET: i16 = -16;
const FLIPPED_EXPLOSION_X_OFFSET: i16 = 96;
const ROPE_ANCHOR_X: u8 = 114;
const FLIPPED_ROPE_ANCHOR_X: u8 = 11;
const ROPE_ANCHOR_FRAME_SPACING: u16 = 5;

/// trigger_detonator_state, 272c:323f. Edge 0 always presses the plunger;
/// other edges require a descending contact above its twelve-pixel head.
/// 272c:3272/3282 use signed comparisons after 16-bit additions.
pub(super) fn trigger_detonator_contact(
    objects: &mut [Object],
    source: usize,
    target: usize,
    edge: u16,
    angle: u16,
) -> CollisionResponse {
    let impact = &objects[source];
    let bottom = impact.y.wrapping_add(impact.height as i16);
    let head = objects[target].y.wrapping_add(HEAD_HEIGHT_PIXELS);
    if edge == 0
        || (impact.motion.velocity_y > 0
            && (angle.wrapping_add(CONTACT_ANGLE_BIAS) as i16) < CONTACT_ANGLE_LIMIT
            && bottom < head)
    {
        objects[target].power_state = 1;
    }
    CollisionResponse::Ordinary
}

/// update_detonator_trigger, Ghidra 272c:332a. This is the impact/cable
/// switch animation that creates a short-lived explosion at its nose.
pub(super) fn update_detonator_trigger(objects: &mut Vec<Object>, index: usize) {
    if objects[index].power_state == 0 {
        return;
    }
    if objects[index].state == FRAME_FIRE_EXPLOSION {
        let flipped = objects[index]
            .render_flags
            .contains(crate::machine::RenderFlag::FlipHorizontal);
        let x = objects[index]
            .x
            .wrapping_add(EXPLOSION_X_OFFSET)
            .wrapping_add(if flipped {
                FLIPPED_EXPLOSION_X_OFFSET
            } else {
                0
            });
        let y = objects[index].y;
        spawn_explosion_at(objects, x, y);
    }
    if objects[index].state != FRAME_PRESSED {
        objects[index].state = objects[index].state.wrapping_add(1);
        let anchor_x = if !objects[index]
            .render_flags
            .contains(crate::machine::RenderFlag::FlipHorizontal)
        {
            ROPE_ANCHOR_X
        } else {
            FLIPPED_ROPE_ANCHOR_X
        };
        objects[index].rope_anchors[0] = (
            anchor_x,
            (objects[index].state.min(FRAME_PRESSED) * ROPE_ANCHOR_FRAME_SPACING) as u8,
        );
        update_object_outline(&mut objects[index]);
    }
}

/// 272c:341d detonator rope contact callback. A pulling contact presses the
/// detonator; an already-triggered plunger blocks the same directions as DOS.
pub(super) fn detonator_rope_contact(
    objects: &mut [Object],
    detonator_index: usize,
    contact_flags: u16,
) -> bool {
    if contact_flags == RopeContactFlag::LocalDirectionMatch.mask() {
        if let Some(connected) = objects[detonator_index].linked_objects[0] {
            objects[connected].state = objects[connected].state.wrapping_add(1);
        }
        return false;
    }
    let detonator = &mut objects[detonator_index];
    if (contact_flags & RopeContactFlag::REMOTE_DIRECTION_MASK)
        == RopeContactFlag::RemoteDirectionMismatch.mask()
        || ((contact_flags & RopeContactFlag::REMOTE_DIRECTION_MASK)
            == RopeContactFlag::RemoteDirectionMatch.mask()
            && detonator.state == FRAME_PRESSED)
    {
        return true;
    }
    if (contact_flags & RopeContactFlag::PROBE_REMOTE_MASK)
        == RopeContactFlag::RemoteDirectionMatch.mask()
        && detonator.power_state == 0
    {
        detonator.power_state = 1;
    }
    false
}
