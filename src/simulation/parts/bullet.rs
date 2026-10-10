use super::*;
use crate::sound::SoundEffect;

const FRAME_FLYING: u16 = 0;
const FRAME_IMPACT: u16 = 1;
const FRAME_SPENT: u16 = 2;
pub(super) const BULLET_LAUNCH_SPEED: i16 = 12288;

/// update_bullet_flight, Ghidra 272c:08f1: any change from launch speed means
/// an impact; DOS clears the vertex count but retains the non-null outline
/// pointer through the final hidden update.
pub(super) fn update_bullet_flight(bullet: &mut Object) {
    match bullet.state {
        FRAME_SPENT => {
            bullet
                .render_flags
                .insert(crate::machine::RenderFlag::Hidden);
            bullet.contact = None;
        }
        FRAME_FLYING => {
            if bullet.motion.velocity_x != BULLET_LAUNCH_SPEED
                && bullet.motion.velocity_x != -BULLET_LAUNCH_SPEED
            {
                bullet.state = FRAME_IMPACT;
                crate::sound::play_effect(SoundEffect::Gunshot);
            }
        }
        _ => bullet.state += 1,
    }
}
