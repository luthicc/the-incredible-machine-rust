use super::*;
use crate::sound::SoundEffect;

// Lower state bits retain the socket configuration; bit 2 selects the strip.
const SWITCH_POSITION_BIT: u16 = 4;
const PRIMARY_STRIP_LAST_STATE: u16 = SWITCH_POSITION_BIT - 1;
const CONTACT_QUARTER_TURN: u16 = 16384;
const CONTACT_HALF_TURN_BIT: u16 = 32768;
const TRIGGER_LEFT_OFFSET: i16 = -2;
const TRIGGER_RIGHT_OFFSET: i16 = 20;

/// adjust_power_distributor_level, 272c:14d3: contacts from below/above
/// flip the active strip and compare against the saved switch state.
pub(super) fn adjust_power_distributor_level(
    objects: &mut [Object],
    moving: usize,
    target: usize,
    angle: u16,
) -> CollisionResponse {
    let upper = angle.wrapping_add(CONTACT_QUARTER_TURN) & CONTACT_HALF_TURN_BIT == 0;
    let state = objects[target].state;
    if state < SWITCH_POSITION_BIT && upper {
        objects[target].state = state.wrapping_add(SWITCH_POSITION_BIT);
        crate::sound::play_effect(SoundEffect::SwitchToggle);
    } else if state >= SWITCH_POSITION_BIT && !upper {
        objects[target].state = state.wrapping_sub(SWITCH_POSITION_BIT);
        crate::sound::play_effect(SoundEffect::SwitchToggle);
    }
    let part = &mut objects[target];
    part.power_state = i16::from(part.state != part.saved_state);
    part.outline = crate::outline::vertices(part);
    part.outline_angles = part
        .outline
        .as_deref()
        .map(crate::outline::edge_angles)
        .unwrap_or_default();
    if objects[moving].definition == Def::Bullet {
        objects[moving].motion.velocity_x = objects[moving].motion.velocity_x.wrapping_sub(1);
    }
    CollisionResponse::SuppressBounce
}

/// power_distributor_contact_trigger, 272c:2acb.
pub(super) fn power_distributor_contact_trigger(part: &mut Object, x: i16) {
    if part.state > PRIMARY_STRIP_LAST_STATE
        && x > part.x.wrapping_add(TRIGGER_LEFT_OFFSET)
        && x < part.x.wrapping_add(TRIGGER_RIGHT_OFFSET)
    {
        part.state -= SWITCH_POSITION_BIT;
        crate::sound::play_effect(SoundEffect::SwitchToggle);
        part.power_state = i16::from(part.state != part.saved_state);
        part.outline = crate::outline::vertices(part);
        part.outline_angles = part
            .outline
            .as_deref()
            .map(crate::outline::edge_angles)
            .unwrap_or_default();
    }
}
