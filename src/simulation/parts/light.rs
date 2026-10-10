use super::*;
use crate::sound::SoundEffect;

native_part_state!(LightFrame {
    Unlit = 0,
    Lit = 1,
    AlternateUnlit = 2,
    AlternateLit = 3,
});

const UNLIT_ROPE_ANCHOR: (u8, u8) = (21, 51);
const LIT_ROPE_ANCHOR: (u8, u8) = (29, 79);
const ALTERNATE_UNLIT_ROPE_ANCHOR: (u8, u8) = (20, 25);
const ALTERNATE_LIT_ROPE_ANCHOR: (u8, u8) = (28, 35);

/// Ghidra 272c:2b99.
#[allow(non_snake_case)]
pub(super) fn illuminate_light_on_power(objects: &mut [Object], index: usize) {
    let light = &mut objects[index];
    let frame = LightFrame::from_raw(light.state);
    if light.power_state != 0
        && matches!(frame, Some(LightFrame::Unlit | LightFrame::AlternateUnlit))
    {
        light.state = match frame.expect("unlit light state was decoded") {
            LightFrame::Unlit => LightFrame::Lit,
            LightFrame::AlternateUnlit => LightFrame::AlternateLit,
            _ => unreachable!("only unlit frames are advanced"),
        }
        .raw();
        SetLightRopeAnchorForState(light);
    }
}

/// Editor/flip action 272c:2bc5; never called by physical collision response.
#[allow(non_snake_case)]
pub fn toggle_light_state(light: &mut Object) {
    light.state = if LightFrame::from_raw(light.state) == Some(LightFrame::Unlit) {
        LightFrame::AlternateUnlit.raw()
    } else {
        LightFrame::Unlit.raw()
    };
    SetLightRopeAnchorForState(light);
}

/// Ghidra 272c:2b58 selects the saved local rope anchor using the current state.
#[allow(non_snake_case)]
pub(super) fn SetLightRopeAnchorForState(light: &mut Object) {
    light.rope_anchors[0] = match LightFrame::from_raw(light.state) {
        Some(LightFrame::Unlit) => UNLIT_ROPE_ANCHOR,
        Some(LightFrame::Lit) => LIT_ROPE_ANCHOR,
        Some(LightFrame::AlternateUnlit) => ALTERNATE_UNLIT_ROPE_ANCHOR,
        Some(LightFrame::AlternateLit) => ALTERNATE_LIT_ROPE_ANCHOR,
        None => light.rope_anchors[0],
    };
    light.outline = crate::outline::vertices(light);
}

/// Ghidra 272c:2c19. The contact record is the light in this callback-table
/// dispatch. Other low flag bits short-circuit propagation; flag 4 powers it.
#[allow(non_snake_case)]
pub(super) fn process_light_rope_contact(light: &mut Object, contact_flags: u16) -> bool {
    if (contact_flags & RopeContactFlag::REMOTE_DIRECTION_MASK)
        == RopeContactFlag::RemoteDirectionMismatch.mask()
        || ((contact_flags & RopeContactFlag::REMOTE_DIRECTION_MASK)
            == RopeContactFlag::RemoteDirectionMatch.mask()
            && light.power_state != 0)
    {
        return true;
    }
    if (contact_flags & RopeContactFlag::PROBE_REMOTE_MASK)
        == RopeContactFlag::RemoteDirectionMatch.mask()
        && light.power_state == 0
    {
        crate::sound::play_effect(SoundEffect::SwitchToggle);
        light.power_state = 1;
    }
    false
}
