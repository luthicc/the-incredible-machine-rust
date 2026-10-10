use super::*;
use crate::sound::SoundEffect;

const FRAME_FIRST: u16 = 0;
const FRAME_CYCLE_END: u16 = 4;
const AIR_STRENGTH: i32 = 4096;
const AIR_REACH_PIXELS: i16 = 256;
const AIR_VERTICAL_RANGE: (i16, i16) = (-10, 0);
const WINDMILL_REACH_PIXELS: i32 = 200;

/// 272c:1a82 uses the nearby query 1000:3566 and mass at 3d3c:0ea8.
pub(super) fn update_fan_airflow(
    objects: &mut [Object],
    index: usize,
    forces: &[crate::forces::PartForce; 58],
) {
    use crate::spatial::{Facing, nearby_parts};
    if objects[index].power_state == 0 {
        return;
    }
    crate::sound::keep_effect_looping(SoundEffect::FanRunning);
    if objects[index].state == objects[index].previous_state {
        crate::sound::play_effect(SoundEffect::FanRunning);
    }
    let facing = Facing::from_render_flags(objects[index].render_flags);
    objects[index].state = objects[index].state.wrapping_add(1);
    if objects[index].state == FRAME_CYCLE_END {
        objects[index].state = FRAME_FIRST;
    }
    let horizontal = match facing {
        Facing::Right => (0, AIR_REACH_PIXELS),
        Facing::Left => (-AIR_REACH_PIXELS, 0),
    };
    let contacts = nearby_parts(objects, index, horizontal, AIR_VERTICAL_RANGE);
    for contact in contacts {
        let part = &mut objects[contact.index];
        let distance = i32::from(contact.horizontal_gap).abs();
        // This branch tests update flags (+6), not the render flags (+8).
        if part.update_flags & UpdateFlag::ActiveFixed.mask() == 0 {
            let impulse = air_impulse(facing.sign() * AIR_STRENGTH, distance, part.definition);
            part.motion.velocity_x = part.motion.velocity_x.wrapping_add(impulse as i16);
            part.motion
                .limit_velocity(forces[part.definition.id() as usize].speed_limit);
            if part.definition == Def::Candle {
                part.callback_timer = 0;
                part.power_state = 0;
                part.state = CANDLE_UNLIT_FRAME;
            }
        } else if part.definition == Def::Windmill && distance < WINDMILL_REACH_PIXELS {
            part.power_state = 1;
            part.callback_timer = WINDMILL_COAST_TICKS;
        }
    }
}
