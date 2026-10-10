use super::*;
use crate::sound::SoundEffect;

pub(super) const BELLOWS_EXPANDED_FRAME: u16 = 0;
const FRAME_COMPRESSED: u16 = 2;
const COMPRESS_POWER: i16 = 1;
const RELEASE_POWER: i16 = -1;
const AIR_STRENGTH: i32 = 2048;
const AIR_REACH_PIXELS: i16 = 128;
const AIR_VERTICAL_RANGE: (i16, i16) = (-10, 0);

const COMPRESSION_EDGES: [u16; 2] = [0, 4];
const FLIPPED_COMPRESSION_EDGES: [u16; 2] = [1, 3];

/// Native 272c:0332; contact activates the facing-specific compression edges.
pub(super) fn bellows_contact(part: &mut Object, edge: u16) -> CollisionResponse {
    let active = if !part
        .render_flags
        .contains(crate::machine::RenderFlag::FlipHorizontal)
    {
        COMPRESSION_EDGES.contains(&edge)
    } else {
        FLIPPED_COMPRESSION_EDGES.contains(&edge)
    };
    if active {
        part.power_state = COMPRESS_POWER;
    }
    CollisionResponse::Ordinary
}

pub(super) fn process_bellows(
    objects: &mut [Object],
    index: usize,
    forces: &[crate::forces::PartForce; 58],
) {
    objects[index]
        .render_flags
        .insert(crate::machine::RenderFlag::ProcessedThisUpdate);
    if objects[index].power_state == COMPRESS_POWER && objects[index].state != FRAME_COMPRESSED {
        // update_bellows_wind (272c:0405) queries nearby parts after it
        // advances the animation state, but +0x44/+0x46 extents are rebuilt
        // only after that query. Capture with the previous frame's extents.
        let flipped = objects[index]
            .render_flags
            .contains(crate::machine::RenderFlag::FlipHorizontal);
        let range = if flipped {
            (-AIR_REACH_PIXELS, 0)
        } else {
            (0, AIR_REACH_PIXELS)
        };
        let contacts = crate::spatial::nearby_parts(objects, index, range, AIR_VERTICAL_RANGE);
        objects[index].state = objects[index].state.wrapping_add(1);
        for contact in contacts {
            let target = &mut objects[contact.index];
            if target.update_flags & UpdateFlag::ACTIVE_LIST_MASK == 0 {
                continue;
            }
            if target.update_flags & UpdateFlag::ActiveMoving.mask() == 0 {
                if target.definition == Def::Windmill {
                    target.power_state = 1;
                    target.callback_timer = WINDMILL_COAST_TICKS;
                }
            } else {
                let distance = i32::from(contact.horizontal_gap).abs();
                let impulse = air_impulse(
                    if flipped { -AIR_STRENGTH } else { AIR_STRENGTH },
                    distance,
                    target.definition,
                );
                target.motion.velocity_x = target.motion.velocity_x.wrapping_add(impulse as i16);
                target
                    .motion
                    .limit_velocity(forces[target.definition.id() as usize].speed_limit);
                if target.definition == Def::Candle {
                    target.callback_timer = 0;
                    target.power_state = 0;
                    target.state = CANDLE_UNLIT_FRAME;
                }
            }
        }
    } else if objects[index].power_state == RELEASE_POWER
        && objects[index].state != BELLOWS_EXPANDED_FRAME
    {
        objects[index].state = objects[index].state.wrapping_sub(1);
    }
    if objects[index].state != objects[index].previous_state {
        objects[index].outline = crate::outline::vertices(&objects[index]);
        objects[index].outline_angles =
            crate::outline::edge_angles(objects[index].outline.as_deref().unwrap_or(&[]));
        if matches!(
            objects[index].previous_state,
            BELLOWS_EXPANDED_FRAME | FRAME_COMPRESSED
        ) {
            crate::sound::play_effect(SoundEffect::MechanismMovement);
        }
    }
}
