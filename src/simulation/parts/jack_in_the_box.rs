use super::*;
use crate::sound::{self, SoundEffect};

// (Inclusive upper mass, launch speed in native velocity units.)
const THROW_SPEED_BY_MASS: &[(i16, i16)] = &[
    (1, 6144),
    (5, 5632),
    (9, 5120),
    (20, 4608),
    (120, 4096),
    (150, 3584),
    (200, 3072),
    (1800, 2560),
    (i16::MAX, 2048),
];
const THROW_RIGHT_BOUND_PIXELS: i16 = 31;

/// Native animation boundaries at 272c:27e2. Intermediate values are sprite
/// frames: eight crank frames, followed by eleven pop/open frames.
#[repr(u16)]
enum JackInTheBoxFrame {
    FirstCrank = 0,
    LastCrank = 7,
    FirstPop = 8,
    LastThrow = 10,
    FullyOpen = 18,
}

impl JackInTheBoxFrame {
    const fn raw(self) -> u16 {
        self as u16
    }
}

// Native +0x96 (activation_timer) counts completed crank turns for this part.
const CRANK_TURNS_BEFORE_POP: i16 = 6;
const REVERSE_CRANK_WRAP: u16 = u16::MAX;

/// compute_jack_in_the_box_velocity, 272c:29c6: original mass thresholds.
pub(super) fn compute_jack_in_the_box_velocity(part: &Object) -> i16 {
    launch_speed_for_mass(crate::forces::mass(part.definition), THROW_SPEED_BY_MASS)
}

/// update_jack_in_the_box, 272c:27e2: six crank turns, pop and throw.
pub(super) fn update_jack_in_the_box(objects: &mut [Object], index: usize) {
    use JackInTheBoxFrame as Frame;
    let part = &mut objects[index];
    // Preserve the native signed comparison even for wrapped state values.
    if (part.state as i16) < Frame::FirstPop.raw() as i16 {
        if part.power_state != 0 {
            let drive = if !part
                .render_flags
                .contains(crate::machine::RenderFlag::FlipHorizontal)
            {
                part.power_state
            } else {
                part.power_state.wrapping_neg()
            };
            part.state = part.state.wrapping_add(drive as u16);
            if part.state == Frame::FirstPop.raw() {
                part.state = Frame::FirstCrank.raw();
                part.activation_timer = part.activation_timer.wrapping_add(1);
            } else if part.state == REVERSE_CRANK_WRAP {
                part.state = Frame::LastCrank.raw();
                part.activation_timer = part.activation_timer.wrapping_add(1);
            }
            if part.activation_timer == CRANK_TURNS_BEFORE_POP {
                sound::play_effect(SoundEffect::SpringRelease);
                part.state = Frame::FirstPop.raw();
            }
        }
    } else if part.state != Frame::FullyOpen.raw() {
        part.state = part.state.wrapping_add(1);
    }
    if !(Frame::FirstPop.raw()..=Frame::LastThrow.raw()).contains(&part.state) {
        return;
    }
    let top = crate::jack_tables::POP_TOP[(part.state - Frame::FirstPop.raw()) as usize];
    let center = part.x.wrapping_add((part.width >> 1) as i16);
    let flipped = part
        .render_flags
        .contains(crate::machine::RenderFlag::FlipHorizontal);
    let targets = crate::spatial::collect_parts_overlapping_rectangle(
        objects,
        index,
        UpdateFlag::ACTIVE_LIST_MASK,
        (0, THROW_RIGHT_BOUND_PIXELS, top, 0),
    );
    for target in targets {
        if objects[target].update_flags & UpdateFlag::ActiveMoving.mask() != 0 {
            let speed = compute_jack_in_the_box_velocity(&objects[target]);
            objects[target].motion.velocity_x = if flipped { speed } else { -speed };
            objects[target].motion.velocity_y = -speed;
        } else {
            // Jump table is in CS:2945/2951, not the data segment.
            match objects[target].definition {
                Def::Fish => make_fish_jump(&mut objects[target]),
                Def::CatPoweredDrive => activate_contacted_object(&mut objects[target]),
                Def::Seesaw => update_proximity_power_state(&mut objects[target], center),
                Def::Bellows => update_proximity_power_state_wide(&mut objects[target], center),
                Def::PowerDistributor => {
                    power_distributor_contact_trigger(&mut objects[target], center)
                }
                Def::Scissors => update_proximity_power_state_narrow(&mut objects[target], center),
                _ => {}
            }
        }
    }
}

#[cfg(test)]
#[path = "jack_in_the_box_tests.rs"]
mod mass_curve_tests;
