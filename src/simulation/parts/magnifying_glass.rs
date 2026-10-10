use super::*;

const FOCUS_TARGET_LINK: usize = 4;
const INITIAL_NEAREST_DISTANCE: i32 = 400;
const LIGHT_SEARCH_RANGE: (i16, i16) = (-32, 32);
const LENS_Y_OFFSET_PIXELS: i16 = 8;
const FOCUS_REACH_PIXELS: i32 = 47;
// All three eligible sources have frame 0 unlit; combustibles start at frame 0.
const UNLIT_SOURCE_FRAME: u16 = 0;
const IGNITION_READY_FRAME: u16 = 0;

/// 272c:3035/31dc: retain the current focus when still eligible, otherwise
/// select the nearest combustible point opposite an active light source.
pub(super) fn update_magnifying_glass_focus(objects: &mut [Object], index: usize) {
    let flipped = objects[index]
        .render_flags
        .contains(crate::machine::RenderFlag::FlipHorizontal);
    let previous = objects[index].linked_objects[FOCUS_TARGET_LINK];
    let mut target = None;
    let mut nearest = INITIAL_NEAREST_DISTANCE;
    let mut illuminated = false;
    let mut retained = false;
    let contacts = crate::spatial::nearby_parts(objects, index, LIGHT_SEARCH_RANGE, (0, 0));
    for contact in contacts {
        let part = &objects[contact.index];
        if matches!(part.definition, Def::Light | Def::Flashlight | Def::Candle)
            && part.state != UNLIT_SOURCE_FRAME
        {
            if (!flipped && contact.horizontal_gap < 0) || (flipped && contact.horizontal_gap > 0) {
                illuminated = true;
            }
            if part.definition == Def::Flashlight
                && part
                    .render_flags
                    .contains(crate::machine::RenderFlag::FlipHorizontal)
                    != objects[index]
                        .render_flags
                        .contains(crate::machine::RenderFlag::FlipHorizontal)
            {
                illuminated = false;
            }
            if part.definition == Def::Light && part.state == LightFrame::AlternateUnlit.raw() {
                illuminated = false;
            }
        } else if part.flags & PropertyFlag::Ignitable.mask() != 0
            && part.state == IGNITION_READY_FRAME
            && !retained
        {
            if let Some((ax, ay)) = part.ignition_anchor() {
                let lens_x = objects[index].x.wrapping_add(if flipped {
                    0
                } else {
                    objects[index].width as i16
                });
                // compute_part_position_delta, 272c:31dc, returns absolute
                // point deltas; collect_nearby_parts supplies the signed side.
                let dx = i32::from(lens_x.wrapping_sub(part.x.wrapping_add(i16::from(ax)))).abs();
                let dy = i32::from(
                    objects[index]
                        .y
                        .wrapping_add(LENS_Y_OFFSET_PIXELS)
                        .wrapping_sub(part.y.wrapping_add(i16::from(ay))),
                )
                .abs();
                let target_on_focused_side = if flipped {
                    contact.horizontal_gap < 0
                } else {
                    contact.horizontal_gap > 0
                };
                if target_on_focused_side && dx <= FOCUS_REACH_PIXELS && dx >= dy {
                    if previous == Some(contact.index) {
                        retained = true;
                        target = Some(contact.index);
                    } else if i32::from(contact.horizontal_gap).abs() < nearest {
                        nearest = i32::from(contact.horizontal_gap).abs();
                        target = Some(contact.index);
                    }
                }
            }
        }
        if illuminated && retained {
            break;
        }
    }
    if !illuminated {
        target = None;
    }
    objects[index].linked_objects[FOCUS_TARGET_LINK] = target;
    if let Some(target) = target {
        objects[target].callback_timer = objects[target].callback_timer.wrapping_add(1);
    }
}
