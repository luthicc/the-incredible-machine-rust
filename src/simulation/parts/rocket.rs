use super::*;
use crate::sound::SoundEffect;

const FRAME_LAUNCH_SOUND: u16 = 6;
const FRAME_FIRST_FLIGHT: u16 = 7;
const FRAME_FLIGHT_CYCLE_END: u16 = 10;
const HEAT_FRAME_BIT: u16 = 1;
const THRUST_PER_TICK: i16 = 1024;
const EXHAUST_BOUNDS: (i16, i16, i16, i16) = (-4, 18, 48, 81);

/// update_rocket_flight, Ghidra 272c:3635. The magnifier and other native
/// ignition sources accumulate callback_timer; the rocket consumes it after
/// 20, launches through states 7..9, and propagates its flame to nearby parts.
pub(super) fn update_rocket_flight(
    objects: &mut [Object],
    index: usize,
    forces: &[crate::forces::PartForce; 58],
) {
    objects[index].contact = None;
    if objects[index].power_state == 0 && objects[index].callback_timer > IGNITION_HEAT_THRESHOLD {
        objects[index].power_state = 1;
    }
    if objects[index].power_state == 0 {
        return;
    }
    objects[index].state = objects[index].state.wrapping_add(1);
    if objects[index].state == FRAME_FLIGHT_CYCLE_END {
        objects[index].state = FRAME_FIRST_FLIGHT;
    }
    if objects[index].state == FRAME_LAUNCH_SOUND {
        crate::sound::play_effect(SoundEffect::RocketLaunch);
    }
    if objects[index].state > FRAME_LAUNCH_SOUND {
        objects[index].motion.velocity_y = objects[index]
            .motion
            .velocity_y
            .wrapping_sub(THRUST_PER_TICK);
        objects[index]
            .motion
            .limit_velocity(forces[Def::Rocket.id() as usize].speed_limit);

        for target in crate::spatial::ignition_targets(objects, index, EXHAUST_BOUNDS) {
            if objects[target].power_state == 0 {
                objects[target].power_state = 1;
            }
        }

        if objects[index].state & HEAT_FRAME_BIT != 0 {
            for target in crate::spatial::collect_parts_overlapping_rectangle(
                objects,
                index,
                UpdateFlag::ActiveMoving.mask(),
                EXHAUST_BOUNDS,
            ) {
                match objects[target].definition {
                    Def::Balloon => objects[target].power_state = 1,
                    Def::Cat if objects[target].state == CAT_IDLE_FRAME => {
                        objects[target].state = CAT_STARTLED_FRAME;
                        objects[target].activation_timer = 0;
                        crate::sound::play_effect(SoundEffect::CatStartled);
                    }
                    _ => {}
                }
            }
        }
    }
}
