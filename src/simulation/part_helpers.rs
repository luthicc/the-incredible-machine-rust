use super::*;
use crate::sound::SoundEffect;

pub(super) const IGNITION_HEAT_THRESHOLD: i16 = 20;
pub(super) const ELECTRICAL_OUTPUT_LINKS: [usize; 2] = [4, 5];
const LETTERING_REST_FRAME: u16 = 1;
const LETTERING_MOVING_FRAME: u16 = 3;
const SEESAW_LEFT_TRIGGER_OFFSET: i16 = 36;
const SEESAW_CENTER_TRIGGER_OFFSET: i16 = 40;
const SEESAW_RIGHT_TRIGGER_OFFSET: i16 = 44;
const BELLOWS_TRIGGER_OFFSET: i16 = 44;
const FLIPPED_BELLOWS_TRIGGER_OFFSET: i16 = 12;
const SCISSORS_TRIGGER_OFFSET: i16 = 24;
const FLIPPED_SCISSORS_TRIGGER_OFFSET: i16 = 18;

/// Tables must be sorted by inclusive upper mass, ending at i16::MAX.
pub(super) fn launch_speed_for_mass(mass: i16, bands: &[(i16, i16)]) -> i16 {
    bands
        .iter()
        .find(|&&(upper_mass, _)| mass <= upper_mass)
        .expect("launch speed table covers every signed native mass")
        .1
}

/// start_part_horizontal_motion, 272c:11a6: credit lettering changes frame when set in motion.
pub(super) fn start_part_horizontal_motion(part: &mut Object) {
    if part.state == LETTERING_REST_FRAME && part.motion.velocity_x != 0 {
        part.state = LETTERING_MOVING_FRAME;
        crate::sound::play_effect(SoundEffect::SpringRelease);
    }
}

/// update_proximity_power_state, 272c:2a3a.
pub(super) fn update_proximity_power_state(part: &mut Object, x: i16) {
    match SeesawPosition::from_raw(part.state) {
        Some(SeesawPosition::Left) if x < part.x.wrapping_add(SEESAW_LEFT_TRIGGER_OFFSET) => {
            part.power_state = 1
        }
        Some(SeesawPosition::Level) => {
            part.power_state = if x < part.x.wrapping_add(SEESAW_CENTER_TRIGGER_OFFSET) {
                1
            } else {
                -1
            }
        }
        Some(SeesawPosition::Right) if x > part.x.wrapping_add(SEESAW_RIGHT_TRIGGER_OFFSET) => {
            part.power_state = -1
        }
        _ => {}
    }
}

/// update_proximity_power_state_wide, 272c:2a91.
pub(super) fn update_proximity_power_state_wide(part: &mut Object, x: i16) {
    if part.state == BELLOWS_EXPANDED_FRAME
        && ((!part
            .render_flags
            .contains(crate::machine::RenderFlag::FlipHorizontal)
            && x < part.x.wrapping_add(BELLOWS_TRIGGER_OFFSET))
            || (part
                .render_flags
                .contains(crate::machine::RenderFlag::FlipHorizontal)
                && x > part.x.wrapping_add(FLIPPED_BELLOWS_TRIGGER_OFFSET)))
    {
        part.power_state = 1;
    }
}

/// update_proximity_power_state_narrow, 272c:2b1e.
pub(super) fn update_proximity_power_state_narrow(part: &mut Object, x: i16) {
    if part.state == ScissorsSnap::Open.raw()
        && ((!part
            .render_flags
            .contains(crate::machine::RenderFlag::FlipHorizontal)
            && x < part.x.wrapping_add(SCISSORS_TRIGGER_OFFSET))
            || (part
                .render_flags
                .contains(crate::machine::RenderFlag::FlipHorizontal)
                && x > part.x.wrapping_add(FLIPPED_SCISSORS_TRIGGER_OFFSET)))
    {
        part.power_state = 1;
    }
}

/// 272c:2ffd; orientation selects signed drive and restarts its pulse.
pub(super) fn activate_contacted_object(part: &mut Object) {
    if part.power_state == 0 {
        crate::sound::play_effect(SoundEffect::CatPoweredDrive);
    }
    part.power_state = if !part
        .render_flags
        .contains(crate::machine::RenderFlag::FlipHorizontal)
    {
        1
    } else {
        -1
    };
    part.activation_timer = CAT_DRIVE_RUN_TICKS;
}

pub(super) fn update_object_outline(object: &mut Object) {
    object.outline = crate::outline::vertices(object);
    object.outline_angles = object
        .outline
        .as_deref()
        .map(crate::outline::edge_angles)
        .unwrap_or_default();
}
