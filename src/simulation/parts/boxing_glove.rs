use super::*;
use crate::sound::SoundEffect;

const FRAME_RETRACTED: u16 = 0;
const FRAME_PUNCH_NEAR: u16 = 2;
const FRAME_PUNCH_FAR: u16 = 3;
const FRAME_EXTENDED: u16 = 9;
const PUNCH_HEIGHT_PIXELS: i16 = 31;
const RIGHT_PUNCH_START: i16 = 48;
const RIGHT_PUNCH_NEAR_END: i16 = 80;
const RIGHT_PUNCH_FAR_END: i16 = 130;
const LEFT_PUNCH_NEAR_START: i16 = -32;
const LEFT_PUNCH_FAR_START: i16 = -82;
// (Inclusive upper mass, horizontal launch speed in native velocity units.)
const PUNCH_SPEED_BY_MASS: &[(i16, i16)] = &[
    (5, 6656),
    (9, 6144),
    (20, 5632),
    (120, 5120),
    (150, 4608),
    (200, 4096),
    (1800, 3584),
    (i16::MAX, 3072),
];

/// 272c:057e/06f9; exact punch bounds and mass-dependent signed impulse.
/// update_boxing_glove_punch, Ghidra 272c:057e.
pub(super) fn update_boxing_glove_punch(objects: &mut [Object], index: usize) {
    if objects[index].power_state != 0 && objects[index].state != FRAME_EXTENDED {
        if objects[index].state == FRAME_RETRACTED {
            crate::sound::play_effect(SoundEffect::SpringRelease);
        }
        objects[index].state = objects[index].state.wrapping_add(1);
        objects[index].outline = crate::outline::vertices(&objects[index]);
        objects[index].outline_angles = objects[index]
            .outline
            .as_deref()
            .map(crate::outline::edge_angles)
            .unwrap_or_default();
    }
    let state = objects[index].state;
    if !matches!(state, FRAME_PUNCH_NEAR | FRAME_PUNCH_FAR) {
        return;
    }
    let flipped = objects[index]
        .render_flags
        .contains(crate::machine::RenderFlag::FlipHorizontal);
    let (left, right) = if flipped {
        (
            RIGHT_PUNCH_START,
            if state == FRAME_PUNCH_NEAR {
                RIGHT_PUNCH_NEAR_END
            } else {
                RIGHT_PUNCH_FAR_END
            },
        )
    } else {
        (
            if state == FRAME_PUNCH_NEAR {
                LEFT_PUNCH_NEAR_START
            } else {
                LEFT_PUNCH_FAR_START
            },
            0,
        )
    };
    let x = objects[index].x;
    let y = objects[index].y;
    for (target, part) in objects.iter_mut().enumerate() {
        if target == index
            || part.list == ObjectList::Inventory
            || part
                .render_flags
                .contains(crate::machine::RenderFlag::Hidden)
        {
            continue;
        }
        if part.x >= x.wrapping_add(right)
            || part.x.wrapping_add(part.width as i16) <= x.wrapping_add(left)
            || part.y >= y.wrapping_add(PUNCH_HEIGHT_PIXELS)
            || part.y.wrapping_add(part.height as i16) <= y
        {
            continue;
        }
        if part.update_flags & UpdateFlag::ActiveMoving.mask() != 0 {
            let mass = crate::forces::mass(part.definition);
            let speed = launch_speed_for_mass(mass, PUNCH_SPEED_BY_MASS);
            part.motion.velocity_x = if flipped { speed } else { -speed };
        } else if part.definition == Def::CatPoweredDrive {
            activate_contacted_object(part);
        } else if part.definition == Def::Fish {
            make_fish_jump(part);
        }
    }
}

#[cfg(test)]
#[path = "boxing_glove_tests.rs"]
mod mass_curve_tests;
