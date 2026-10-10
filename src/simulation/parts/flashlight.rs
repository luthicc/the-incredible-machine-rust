use super::*;
use crate::sound::SoundEffect;

const FRAME_OFF: u16 = 0;
const FRAME_ON: u16 = 1;

/// activate_flashlight_contact, 272c:1d07: a horizontal top contact powers the switch.
pub(super) fn activate_flashlight_contact(
    objects: &mut [Object],
    _moving: usize,
    target: usize,
    edge: u16,
) -> CollisionResponse {
    if objects[_moving]
        .contact
        .map_or(objects[target].outline_angles[edge as usize], |c| c.angle)
        == 0
    {
        objects[target].power_state = 1;
    }
    CollisionResponse::Ordinary
}

/// 272c:1d78 activates state 0 once, without consuming power or changing facing.
pub(super) fn update_flashlight_switch(flashlight: &mut Object) {
    if flashlight.power_state != 0 && flashlight.state == FRAME_OFF {
        flashlight.state = FRAME_ON;
        crate::sound::play_effect(SoundEffect::SwitchToggle);
        flashlight.outline = crate::outline::vertices(flashlight);
        flashlight.outline_angles = flashlight
            .outline
            .as_deref()
            .map(crate::outline::edge_angles)
            .unwrap_or_default();
    }
}
