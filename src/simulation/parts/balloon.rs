use super::*;
use crate::sound::SoundEffect;

const FRAME_INTACT: u16 = 0;
const FRAME_HIDDEN: u16 = 6;
const INITIAL_POP_POWER: i16 = 1;

/// 272c:018e replaces the first attached rope endpoint before pop animation.
pub(super) fn update_balloon_pop(
    objects: &mut Vec<Object>,
    index: usize,
    _rope_end_speed_limit: i16,
) {
    if objects[index].power_state == 0 {
        return;
    }
    objects[index]
        .render_flags
        .insert(crate::machine::RenderFlag::ProcessedThisUpdate);
    if objects[index].state == FRAME_HIDDEN {
        objects[index]
            .render_flags
            .insert(crate::machine::RenderFlag::Hidden);
        return;
    }
    if objects[index].power_state == INITIAL_POP_POWER {
        // The native balloon points at its rope record; here the rope may
        // instead belong to a separate Rope part with the balloon as an end.
        let owned = objects[index].ropes[0].clone();
        let tied = owned.is_some()
            || objects.iter().any(|o| {
                o.ropes
                    .iter()
                    .flatten()
                    .any(|c| c.endpoints.contains(&Some(index)))
            });
        if tied {
            let new_index = objects.len();
            // The loose end starts where the knot is drawn.
            let (x, y) = crate::machine::rope_point(&objects[index], 0);
            let (x, y) = (x as i16, y as i16);
            let neighbor = objects[index].linked_objects[0];
            let endpoint = Object {
                contact: None,
                list: ObjectList::Moving,
                update_flags: UpdateFlag::InInventory.mask()
                    | UpdateFlag::CreatedDuringSimulation.mask(),
                definition: Def::RopeEnd,
                // create_object_instance_from_definition leaves the new
                // endpoint's +0x3a load field at zero for this update.
                effective_mass: 0,
                collision_impulse: 0,
                motion: crate::motion::Motion::at_pixels(x, y),
                saved_position: (x, y),
                // The native allocator zeroes the new PartRecord and the
                // balloon callback writes its pixel/anchor coordinates only.
                // update_velocity_from_displacement later observes this zero
                // previous pixel if the loose end is corrected this tick.
                previous_position: (0, 0),
                older_position: (0, 0),
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
                ropes: [owned, None],
                linked_objects: [neighbor, None, None, None, None, None],
                next_part: None,
                pulley_connection: None,
                flags: PropertyFlag::FreePlacement.mask(),
                render_flags: RenderFlag::RopeAttachable.mask(),
                x,
                y,
                width: 0,
                height: 0,
                extent_override: None,
                alt_size: (0, 0),
            };
            objects.push(endpoint);
            for object in objects.iter_mut() {
                for connection in object.ropes.iter_mut().flatten() {
                    for side in 0..2 {
                        if connection.endpoints[side] == Some(index) {
                            connection.endpoints[side] = Some(new_index);
                            connection.attachment_slots[side] = 0;
                        }
                    }
                }
                for link in &mut object.linked_objects[..2] {
                    if *link == Some(index) {
                        *link = Some(new_index);
                    }
                }
            }
            objects[index].ropes[0] = None;
            objects[index].linked_objects[0] = None;
        }
    }
    if objects[index].state == FRAME_INTACT {
        crate::sound::play_effect(SoundEffect::BalloonPop);
    }
    objects[index].state = objects[index].state.wrapping_add(1);
}
