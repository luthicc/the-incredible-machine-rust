use super::*;
use crate::sound::SoundEffect;

const FRAME_IDLE: u16 = 0;
const FRAME_FIRST_PEDAL: u16 = 1;
const FRAME_LAST_PEDAL: u16 = 4;
const FRAME_FIRST_IDLE_STARTLE: u16 = 5;
const FRAME_LAST_IDLE_STARTLE: u16 = 8;
const FRAME_FIRST_RIDING_STARTLE: u16 = 9;
const FRAME_RIDING_STARTLE_END: u16 = 13;
const STARTLE_TICKS: i16 = 28;
const UPPER_EDGE_COUNT: u16 = 3;
const RIDING_ROPE_ANCHOR_Y: u8 = 53;
const RIDING_STARTLE_FRAME_OFFSET: u16 = 4;

/// Pedal direction for a monkey facing its saved way.
pub(super) fn monkey_drive(monkey: &Object) -> i16 {
    if !monkey
        .render_flags
        .contains(crate::machine::RenderFlag::FlipHorizontal)
    {
        1
    } else {
        -1
    }
}

/// startle_monkey_on_contact, 272c:2c83: a part striking the monkey's upper
/// edges startles it for 28 updates (states 5..8 when idle, 9..12 while
/// riding) and stops the pedals. The striking part still bounces.
pub(super) fn startle_monkey(monkey: &mut Object, edge: u16) -> CollisionResponse {
    if monkey.activation_timer == 0 && edge < UPPER_EDGE_COUNT {
        monkey.activation_timer = STARTLE_TICKS;
        monkey.power_state = 0;
        monkey.state = if monkey.state == FRAME_IDLE {
            FRAME_FIRST_IDLE_STARTLE
        } else {
            FRAME_FIRST_RIDING_STARTLE
        };
    }
    CollisionResponse::Ordinary
}

/// update_monkey_behavior, 272c:2d40: the belt partner follows the
/// pedals; a running monkey cycles states 1..4. When a startle ends, an
/// idle monkey settles back to 0 and a riding one starts pedalling again.
pub(super) fn update_monkey_behavior(objects: &mut [Object], index: usize) {
    if let Some(partner) = belt_partner(objects, index) {
        if !objects[partner]
            .render_flags
            .contains(crate::machine::RenderFlag::PowerUpdateCallback)
        {
            objects[partner].power_state = objects[index].power_state;
        }
    }
    let monkey = &mut objects[index];
    if monkey.activation_timer == 0 {
        if monkey.power_state != 0 {
            monkey.rope_anchors[0].1 = RIDING_ROPE_ANCHOR_Y;
            monkey.state = if monkey.state == FRAME_LAST_PEDAL {
                FRAME_FIRST_PEDAL
            } else {
                monkey.state + 1
            };
        }
        return;
    }
    monkey.activation_timer -= 1;
    if monkey.activation_timer == 0 {
        if monkey.state < FRAME_FIRST_RIDING_STARTLE {
            monkey.state = FRAME_IDLE;
        } else {
            crate::sound::play_effect(SoundEffect::Monkey);
            monkey.power_state = monkey_drive(monkey);
            monkey.state = FRAME_FIRST_PEDAL;
        }
        return;
    }
    if monkey.state > FRAME_LAST_IDLE_STARTLE {
        monkey.rope_anchors[0].1 = RIDING_ROPE_ANCHOR_Y;
    }
    monkey.state += 1;
    if monkey.state == FRAME_FIRST_RIDING_STARTLE {
        monkey.state = FRAME_FIRST_IDLE_STARTLE;
    } else if monkey.state == FRAME_RIDING_STARTLE_END {
        monkey.state = FRAME_FIRST_RIDING_STARTLE;
    }
}

/// process_monkey_rope_contact, 272c:2e4b: a rope pull starts the
/// pedals, or during a startle makes the monkey ride off once it ends.
/// Flag value 1 denotes a loosening direction; the pedaling callback ignores it.
pub(super) fn monkey_rope_contact(monkey: &mut Object, contact_flags: u16) -> bool {
    if contact_flags == RopeContactFlag::LocalDirectionMatch.mask() {
        return false;
    }
    if (contact_flags & RopeContactFlag::REMOTE_DIRECTION_MASK)
        == RopeContactFlag::RemoteDirectionMismatch.mask()
        || ((contact_flags & RopeContactFlag::REMOTE_DIRECTION_MASK)
            == RopeContactFlag::RemoteDirectionMatch.mask()
            && (monkey.power_state != 0 || monkey.state > FRAME_LAST_IDLE_STARTLE))
    {
        return true;
    }
    if (contact_flags & RopeContactFlag::PROBE_REMOTE_MASK)
        == RopeContactFlag::RemoteDirectionMatch.mask()
        && monkey.power_state == 0
    {
        if monkey.state < FRAME_FIRST_IDLE_STARTLE || monkey.state > FRAME_LAST_IDLE_STARTLE {
            crate::sound::play_effect(SoundEffect::Monkey);
            monkey.power_state = monkey_drive(monkey);
        } else {
            monkey.state += RIDING_STARTLE_FRAME_OFFSET;
        }
    }
    false
}
