use super::*;
use crate::sound::SoundEffect;

const FRAME_FIRST_SWIM: u16 = 0;
pub(super) const FISH_BOWL_BREAK_FRAME: u16 = 11;
const FRAME_FIRST_FLOP: u16 = 14;
const FRAME_FLOP_CYCLE_END: u16 = 23;
const FLOP_CYCLE_LIMIT: i16 = 20;
const GLASS_BASE_VERTEX_COUNT: usize = 3;

/// make_fish_jump, 272c:1c9e: break an intact bowl and retain its glass base.
pub(super) fn make_fish_jump(fish: &mut Object) {
    if fish.state < FISH_BOWL_BREAK_FRAME {
        fish.state = FISH_BOWL_BREAK_FRAME;
        crate::sound::play_effect(SoundEffect::FishBowlBreak);
        fish.outline = crate::outline::vertices(fish);
        // 272c:1c9e changes only the three vertices, retaining the old
        // contact angles. Recomputing slopes here launches the title text.
        fish.outline_angles.truncate(GLASS_BASE_VERTEX_COUNT);
    }
}

/// advance_fish_state, 272c:1c39: the first impact breaks the bowl and
/// suppresses reflection so the impacting part can continue through the glass.
pub(super) fn advance_fish_state(fish: &mut Object) -> CollisionResponse {
    if fish.state < FISH_BOWL_BREAK_FRAME {
        make_fish_jump(fish);
        CollisionResponse::SuppressBounce
    } else {
        CollisionResponse::Ordinary
    }
}

/// 272c:1c5f: intact animation wraps at 11; broken animation loops
/// from 23 to 14 twenty times before freezing. Rendering never drives state.
pub(super) fn update_fish_swim(fish: &mut Object) {
    if fish.callback_timer < FLOP_CYCLE_LIMIT {
        fish.state = fish.state.wrapping_add(1);
    }
    if (fish.state as i16) < FRAME_FLOP_CYCLE_END as i16 {
        if fish.state == FISH_BOWL_BREAK_FRAME {
            fish.state = FRAME_FIRST_SWIM;
        }
    } else {
        fish.state = FRAME_FIRST_FLOP;
        fish.callback_timer = fish.callback_timer.wrapping_add(1);
    }
}
