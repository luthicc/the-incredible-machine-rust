use super::*;
use crate::sound::SoundEffect;

pub(super) const CAT_IDLE_FRAME: u16 = 0;
// Native frame 1 is also the vertically flipped falling pose.
pub(super) const CAT_STARTLED_FRAME: u16 = 1;
const FRAME_FIRST_STEP: u16 = 2;
const FRAME_STEP_CYCLE_END: u16 = 10;
const STARTLE_DELAY_TICKS: i16 = 12;
const FALL_DELAY_TICKS: i16 = 5;
const FALL_DISTANCE_PIXELS: i32 = 2;
const STEP_PIXELS: i16 = 32;
const SEARCH_REACH_PIXELS: i16 = 240;
const INTACT_FISH_REACH: i32 = 96;
const LOOSE_FISH_REACH: i32 = 292;
const MOVING_MOUSE_REACH: i32 = 192;
const IDLE_MOUSE_REACH: i32 = 128;
const MOUSE_CENTER_OFFSET: i16 = 16;
const MOUSE_CATCH_WIDTH: i16 = 55;
const MOUSE_CATCH_HEIGHT: i16 = 39;
const DRIVE_CONTACT_RANGE: (i16, i16) = (-16, 16);
const DRIVE_FRAME_TOGGLE_BIT: u16 = 1;
pub(super) const CAT_DRIVE_RUN_TICKS: i16 = 100;

/// Cat step probe uses native detect_object_collision, 1000:3e23.
pub(super) fn cat_step_is_blocked(objects: &[Object], index: usize) -> bool {
    crate::contact::detect_object_collision(objects, index)
}

pub(super) fn try_cat_step(objects: &mut [Object], index: usize, delta: i16) -> bool {
    let old = objects[index].x;
    objects[index].x = old.wrapping_add(delta);
    objects[index].motion.position_x =
        i32::from(objects[index].x) << crate::motion::POSITION_FRACTION_BITS;
    if cat_step_is_blocked(objects, index) {
        objects[index].x = old;
        objects[index].motion.position_x = i32::from(old) << crate::motion::POSITION_FRACTION_BITS;
        false
    } else {
        objects[index].state = FRAME_FIRST_STEP;
        true
    }
}

/// update_cat_behavior, 272c:0ca3: contact delay, falling/landing states,
/// fish pursuit, mouse pursuit/consumption and collision-checked 32-pixel steps.
pub(super) fn update_cat_behavior(objects: &mut [Object], index: usize) {
    let fallen = i32::from(
        objects[index]
            .y
            .wrapping_sub(objects[index].older_position.1),
    )
    .abs();
    if !objects[index]
        .render_flags
        .contains(crate::machine::RenderFlag::FlipVertical)
    {
        if fallen < FALL_DISTANCE_PIXELS || objects[index].state > CAT_STARTLED_FRAME {
            if objects[index].state == CAT_STARTLED_FRAME {
                objects[index].activation_timer = objects[index].activation_timer.wrapping_add(1);
                if objects[index].activation_timer > STARTLE_DELAY_TICKS {
                    let delta = if !objects[index]
                        .render_flags
                        .contains(crate::machine::RenderFlag::FlipHorizontal)
                    {
                        -STEP_PIXELS
                    } else {
                        STEP_PIXELS
                    };
                    objects[index].activation_timer = 0;
                    if !try_cat_step(objects, index, delta) {
                        if try_cat_step(objects, index, -delta) {
                            objects[index]
                                .render_flags
                                .toggle(crate::machine::RenderFlag::FlipHorizontal);
                        } else {
                            objects[index].state = CAT_IDLE_FRAME;
                        }
                    }
                }
            } else {
                let was_moving = objects[index].state != CAT_IDLE_FRAME;
                if was_moving {
                    objects[index].state = objects[index].state.wrapping_add(1);
                    if objects[index].state == FRAME_STEP_CYCLE_END {
                        objects[index].state = CAT_IDLE_FRAME;
                    }
                }
                if objects[index].state == CAT_IDLE_FRAME {
                    let bounds = if !objects[index]
                        .render_flags
                        .contains(crate::machine::RenderFlag::FlipHorizontal)
                    {
                        (-SEARCH_REACH_PIXELS, 0)
                    } else {
                        (0, SEARCH_REACH_PIXELS)
                    };
                    let contacts = crate::spatial::nearby_parts(objects, index, bounds, (0, 0));
                    for contact in contacts {
                        let target = contact.index;
                        let limit = match objects[target].definition {
                            Def::Fish => {
                                if objects[target].state < FISH_BOWL_BREAK_FRAME {
                                    INTACT_FISH_REACH
                                } else {
                                    LOOSE_FISH_REACH
                                }
                            }
                            Def::Mouse => {
                                let dx = objects[target]
                                    .x
                                    .wrapping_sub(objects[index].x)
                                    .wrapping_add(MOUSE_CENTER_OFFSET);
                                let dy = objects[target].y.wrapping_sub(objects[index].y);
                                if dx > 0
                                    && dx <= MOUSE_CATCH_WIDTH
                                    && dy > 0
                                    && dy <= MOUSE_CATCH_HEIGHT
                                {
                                    objects[target]
                                        .render_flags
                                        .insert(crate::machine::RenderFlag::Hidden);
                                    crate::sound::play_effect(SoundEffect::CatPoweredDrive);
                                    -1
                                } else if was_moving {
                                    MOVING_MOUSE_REACH
                                } else {
                                    IDLE_MOUSE_REACH
                                }
                            }
                            _ => -1,
                        };
                        if i32::from(contact.horizontal_gap).abs() < limit {
                            let delta = if !objects[index]
                                .render_flags
                                .contains(crate::machine::RenderFlag::FlipHorizontal)
                            {
                                -STEP_PIXELS
                            } else {
                                STEP_PIXELS
                            };
                            objects[index].activation_timer = 0;
                            if !try_cat_step(objects, index, delta) {
                                objects[index].state = CAT_IDLE_FRAME;
                            }
                            break;
                        }
                    }
                }
            }
        } else if objects[index].activation_timer < FALL_DELAY_TICKS {
            objects[index].activation_timer = objects[index].activation_timer.wrapping_add(1);
        } else {
            objects[index]
                .render_flags
                .insert(crate::machine::RenderFlag::FlipVertical);
            objects[index].state = CAT_STARTLED_FRAME;
            objects[index].activation_timer = 0;
        }
    } else if objects[index].update_flags & UpdateFlag::SurfaceContact.mask() != 0 {
        objects[index]
            .render_flags
            .remove(crate::machine::RenderFlag::FlipVertical);
        objects[index].state = CAT_IDLE_FRAME;
    }
    objects[index].outline = crate::outline::vertices(&objects[index]);
    objects[index].outline_angles = objects[index]
        .outline
        .as_deref()
        .map(crate::outline::edge_angles)
        .unwrap_or_default();
}

/// 272c:2f3e: nearby cats can start drive; output precedes timer decrement.
pub(super) fn update_cat_powered_drive(objects: &mut [Object], index: usize) {
    if objects[index].power_state == 0 {
        let contacts = crate::spatial::nearby_parts(objects, index, DRIVE_CONTACT_RANGE, (0, 0));
        if contacts.iter().any(|c| {
            objects[c.index].list == ObjectList::Moving && objects[c.index].definition == Def::Cat
        }) {
            objects[index].power_state = 1;
        }
    }
    if let Some(partner) = belt_partner(objects, index) {
        if !objects[partner]
            .render_flags
            .contains(crate::machine::RenderFlag::PowerUpdateCallback)
        {
            objects[partner].power_state = objects[index].power_state;
        }
    }
    if objects[index].power_state != 0 {
        objects[index].state ^= DRIVE_FRAME_TOGGLE_BIT;
        objects[index].activation_timer = objects[index].activation_timer.wrapping_sub(1);
        if objects[index].activation_timer == 0 {
            objects[index].power_state = 0;
        }
    }
}
