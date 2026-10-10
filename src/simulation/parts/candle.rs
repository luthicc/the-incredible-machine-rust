use super::*;
use crate::sound::SoundEffect;

pub(super) const CANDLE_UNLIT_FRAME: u16 = 0;
pub(super) const CANDLE_FIRST_FLAME_FRAME: u16 = 1;
const FRAME_LAST_FLAME: u16 = 5;
const HEAT_FRAME_BIT: u16 = 1;
const FLAME_BOUNDS: (i16, i16, i16, i16) = (9, 18, -10, 5);

/// 272c:098a flame animation, ignition-point propagation, and heat response.
pub(super) fn update_candle_flame(objects: &mut [Object], index: usize) {
    if objects[index].power_state == 0 && objects[index].callback_timer > IGNITION_HEAT_THRESHOLD {
        objects[index].power_state = 1;
    }
    if objects[index].power_state == 0 {
        return;
    }
    objects[index].state = if objects[index].state == FRAME_LAST_FLAME {
        CANDLE_FIRST_FLAME_FRAME
    } else {
        objects[index].state.wrapping_add(1)
    };
    for target in crate::spatial::ignition_targets(objects, index, FLAME_BOUNDS) {
        if objects[target].power_state == 0 {
            objects[target].power_state = 1;
        }
    }
    if objects[index].state & HEAT_FRAME_BIT == 0 {
        return;
    }
    let origin_x = objects[index].x;
    let origin_y = objects[index].y;
    // 1000:36de uses an open rectangle relative to origin, unlike nearby_parts.
    for (other, part) in objects.iter_mut().enumerate() {
        if other == index
            || part.list != ObjectList::Moving
            || part
                .render_flags
                .contains(crate::machine::RenderFlag::Hidden)
        {
            continue;
        }
        let left = origin_x.wrapping_add(FLAME_BOUNDS.0);
        let right = origin_x.wrapping_add(FLAME_BOUNDS.1);
        let top = origin_y.wrapping_add(FLAME_BOUNDS.2);
        let bottom = origin_y.wrapping_add(FLAME_BOUNDS.3);
        if part.x < right
            && left < part.x.wrapping_add(part.width as i16)
            && part.y < bottom
            && top < part.y.wrapping_add(part.height as i16)
        {
            match part.definition {
                Def::Balloon => part.power_state = 1,
                Def::Cat if part.state == CAT_IDLE_FRAME => {
                    part.state = CAT_STARTLED_FRAME;
                    part.activation_timer = 0;
                    crate::sound::play_effect(SoundEffect::CatStartled);
                }
                _ => {}
            }
        }
    }
}
