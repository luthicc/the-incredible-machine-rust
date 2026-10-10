use super::*;
use crate::sound::SoundEffect;

const FRAME_REST: u16 = 0;
const FRAME_LAUNCH: u16 = 3;
const FRAME_CYCLE_END: u16 = 5;
const HIT_HALF_WIDTH_PIXELS: i16 = 15;
const HORIZONTAL_DAMPING_THRESHOLD: i16 = 1023;
const LAUNCH_BOOST: i16 = 1024;

/// Native 272c:3ebf. Contact edge zero is the trampoline's top edge.
/// Returns true when the native callback suppresses the ordinary bounce.
pub(super) fn apply_trampoline_bounce(
    objects: &mut [Object],
    moving: usize,
    trampoline: usize,
    edge: u16,
    speed_limit: i16,
) -> bool {
    let center = objects[moving]
        .x
        .wrapping_add((crate::spatial::native_object_extents(&objects[moving]).0 >> 1) as i16);
    let spring_center = objects[trampoline]
        .x
        .wrapping_add((crate::spatial::native_object_extents(&objects[trampoline]).0 >> 1) as i16);
    let gap = center.wrapping_sub(spring_center);
    if edge != 0 || gap <= -HIT_HALF_WIDTH_PIXELS || gap >= HIT_HALF_WIDTH_PIXELS {
        return false;
    }
    objects[trampoline].power_state = 1;
    objects[moving].contact = None;
    objects[moving].update_flags &= !UpdateFlag::Supported.mask();
    let launch = objects[trampoline].state == FRAME_LAUNCH;
    let y = objects[moving].y;
    let motion = &mut objects[moving].motion;
    if motion.velocity_x.wrapping_abs() > HORIZONTAL_DAMPING_THRESHOLD {
        motion.velocity_x >>= 1;
    }
    motion.position_y = i32::from(y) << crate::motion::POSITION_FRACTION_BITS;
    if launch {
        motion.velocity_y = motion
            .velocity_y
            .wrapping_abs()
            .wrapping_neg()
            .wrapping_sub(LAUNCH_BOOST);
        motion.limit_velocity(speed_limit);
    }
    true
}

pub(super) fn update_trampoline_bounce(part: &mut Object) {
    if part.power_state != 0 {
        part.state = part.state.wrapping_add(1);
        if part.state == FRAME_LAUNCH {
            crate::sound::play_effect(SoundEffect::SpringRelease);
        }
        if part.state == FRAME_CYCLE_END {
            part.state = FRAME_REST;
            part.power_state = 0;
        }
    }
}
