//! Moving rope response from TIM 1000:72c7, called by 1000:1216.
//! Rope owners retain native end allowances at +0x96/+0x9c. Pulley middle
//! spans are routing geometry, not part of a Euclidean spring constraint.
use super::{ImpulsePriority, rope_pull_response, vertical_direction};
use crate::{
    definition::PartDefinition as Def,
    forces::PartForce,
    machine::{Object, ObjectList, RopeConnection, UpdateFlag, linked_point_distance, rope_spans},
};

fn allowance(objects: &[Object], owner: usize, side: usize) -> i32 {
    i32::from(if side == 0 {
        objects[owner].activation_timer
    } else {
        objects[owner].callback_timer
    })
}
fn set_allowance(objects: &mut [Object], owner: usize, side: usize, length: i32) {
    if side == 0 {
        objects[owner].activation_timer = length as i16;
    } else {
        objects[owner].callback_timer = length as i16;
    }
}

/// compute_rope_segment_delta (1000:7947): endpoint minus its adjacent
/// route tangent, with native octagonal length and integer pixel anchors.
fn segment(objects: &[Object], rope: &RopeConnection, side: usize) -> Option<((i32, i32), i32)> {
    let spans = rope_spans(objects, rope).ok()?;
    let span = if side == 0 {
        spans.first()?
    } else {
        spans.last()?
    };
    let delta = if side == 0 {
        (span.start.0 - span.end.0, span.start.1 - span.end.1)
    } else {
        (span.end.0 - span.start.0, span.end.1 - span.start.1)
    };
    Some((delta, linked_point_distance(span.start, span.end)))
}

/// Native `compute_linked_point_distance` reads the type-10 attachment's
/// cached endpoint coordinates during pulley unwrapping. Those coordinates
/// still describe the prior pixel positions; the geometry queue catches up
/// after the current object update. Rebuild that view from previous positions
/// when calculating the replacement allowance.
fn segment_at_previous_positions(
    objects: &[Object],
    rope: &RopeConnection,
    side: usize,
) -> Option<((i32, i32), i32)> {
    let mut previous = objects.to_vec();
    for object in &mut previous {
        object.x = object.previous_position.0;
        object.y = object.previous_position.1;
    }
    segment(&previous, rope, side)
}

/// 1000:72c7 rewires the nearest pulley when the loose end's allowance
/// goes negative. Live slots 0/1 change; saved reset slots 2/3 remain intact.
fn unwrap_pulley(objects: &mut [Object], rope: &RopeConnection, side: usize) -> bool {
    let Some(end) = rope.endpoints[side] else {
        return false;
    };
    let slot = rope.attachment_slots[side] as usize;
    let Some(pulley) = objects[end].linked_objects[slot] else {
        return false;
    };
    if objects[pulley].definition != Def::Pulley {
        return false;
    }
    let onward_slot = if side == 0 { 0 } else { 1 };
    let Some(next) = objects[pulley].linked_objects[onward_slot] else {
        return false;
    };
    let Some(back_slot) = objects[next].linked_objects[..2]
        .iter()
        .position(|&p| p == Some(pulley))
    else {
        return false;
    };
    objects[end].linked_objects[slot] = Some(next);
    objects[next].linked_objects[back_slot] = Some(end);
    objects[pulley].linked_objects[0] = None;
    objects[pulley].linked_objects[1] = None;
    objects[pulley].ropes[0] = None;
    objects[pulley].pulley_connection = None;
    true
}

/// Native position projection and update_velocity_from_displacement
/// (1000:7283, axis mask 1). Only horizontal velocity is recomputed;
/// vertical velocity is zeroed when horizontal displacement is zero.
fn project(
    objects: &mut [Object],
    moving: usize,
    delta: (i32, i32),
    length: i32,
    allowed: i32,
    forces: &[PartForce; 58],
) {
    if length <= 0 {
        return;
    }
    let p = &mut objects[moving];
    p.x =
        p.x.wrapping_add((delta.0 * allowed / length - delta.0) as i16);
    p.y =
        p.y.wrapping_add((delta.1 * allowed / length - delta.1) as i16);
    p.motion.position_x = i32::from(p.x) << 9;
    p.motion.position_y = i32::from(p.y) << 9;
    p.motion.velocity_x = p.x.wrapping_sub(p.previous_position.0).wrapping_shl(9);
    p.motion
        .limit_velocity(forces[p.definition.id() as usize].speed_limit);
    if p.x == p.previous_position.0 {
        p.motion.velocity_y = 0;
    }
}

fn process_rope_contact_response(
    objects: &mut [Object],
    owner: usize,
    side: usize,
    forces: &[PartForce; 58],
    priority: &mut ImpulsePriority,
) -> bool {
    let Some(rope) = objects[owner].ropes[0].clone() else {
        return false;
    };
    let (Some(moving), Some(peer)) = (rope.endpoints[side], rope.endpoints[1 - side]) else {
        return false;
    };
    let Some((delta, length)) = segment(objects, &rope, side) else {
        return false;
    };
    let Some((_, peer_length)) = segment(objects, &rope, 1 - side) else {
        return false;
    };
    let rising = vertical_direction(&objects[moving]);
    let loose = objects[moving].definition == Def::RopeEnd;
    let peer_loose = objects[peer].definition == Def::RopeEnd;
    let mut allowed = allowance(objects, owner, side);
    let mut peer_allowed = allowance(objects, owner, 1 - side);
    let mut stretch = length - allowed;

    // First use the other end's slack, including slack at a fixed anchor.
    // Conservation is between the two end allowances, not a sum of chords.
    if !loose && stretch > 0 && peer_length < peer_allowed {
        let transfer = stretch.min(peer_allowed - peer_length);
        allowed += transfer;
        peer_allowed -= transfer;
        stretch -= transfer;
        set_allowance(objects, owner, side, allowed);
        set_allowance(objects, owner, 1 - side, peer_allowed);
    }

    // A heavier endpoint can shorten the lighter moving end's allowance.
    // Recursion strictly decreases effective mass. Contacts may block that
    // displacement, so give back any transfer the lighter end could not take.
    let mass = i32::from(objects[moving].effective_mass);
    let peer_mass = i32::from(objects[peer].effective_mass);
    if stretch > 0
        && objects[peer].list == ObjectList::Moving
        && !loose
        && !peer_loose
        && mass > peer_mass
        && mass > 0
    {
        let mut transfer = ((stretch * (mass - peer_mass) + mass) / mass)
            .max(1)
            .min(peer_allowed.abs());
        if transfer != 0 {
            peer_allowed -= transfer;
            set_allowance(objects, owner, 1 - side, peer_allowed);
            process_rope_contact_response(objects, owner, 1 - side, forces, priority);
            objects[peer].update_flags &= !UpdateFlag::CONTACT_RESPONSE_MASK;
            crate::contact::update_object_contacts(objects, peer);
            if let Some((_, actual)) = segment(objects, &rope, 1 - side) {
                let blocked = actual - peer_allowed;
                peer_allowed += blocked;
                transfer -= blocked;
                set_allowance(objects, owner, 1 - side, peer_allowed);
            }
            allowed += transfer;
            stretch = length - allowed;
            set_allowance(objects, owner, side, allowed);
        }
    }

    if stretch <= 0 {
        return false;
    }
    if peer_loose && !loose {
        // A cut tail provides rope without acting as a load. Once the tail
        // is consumed at a pulley, bypass that pulley in both graph directions.
        let slot = rope.attachment_slots[side] as usize;
        if objects[moving].linked_objects[slot]
            .is_some_and(|i| objects[i].definition == Def::Pulley)
        {
            peer_allowed -= stretch;
            if peer_allowed < 0 {
                stretch += peer_allowed;
                if unwrap_pulley(objects, &rope, 1 - side) {
                    peer_allowed =
                        segment_at_previous_positions(objects, &rope, 1 - side).map_or(0, |s| s.1);
                }
            }
            set_allowance(objects, owner, 1 - side, peer_allowed);
            set_allowance(objects, owner, side, allowed + stretch);
        }
        return false;
    }

    project(objects, moving, delta, length, allowed, forces);
    if !loose {
        rope_pull_response(objects, &rope, side, rising, priority);
    }
    true
}

/// simulate_object_update (1000:1216): response and contact rescan happen
/// before the next moving object, rather than in a later global solver pass.
pub(super) fn update_moving_rope(
    objects: &mut [Object],
    moving: usize,
    forces: &[PartForce; 58],
    priority: &mut ImpulsePriority,
) {
    let owners: Vec<_> = objects
        .iter()
        .enumerate()
        .filter_map(|(i, o)| {
            if o.definition != Def::Rope
                || o.render_flags.contains(crate::machine::RenderFlag::Hidden)
            {
                return None;
            }
            let rope = o.ropes[0].as_ref()?;
            let side = rope.endpoints.iter().position(|&p| p == Some(moving))?;
            Some((i, side))
        })
        .collect();
    for (owner, side) in owners {
        let strained = process_rope_contact_response(objects, owner, side, forces, priority);
        let prior = if strained {
            objects[moving].update_flags &= !UpdateFlag::CONTACT_RESPONSE_MASK;
            None
        } else {
            objects[moving].contact.take()
        };
        crate::contact::rescan_object_contacts(objects, moving);
        if objects[moving].contact.is_none() && !strained {
            objects[moving].contact = prior;
        }
    }
}

#[cfg(test)]
mod tests;
