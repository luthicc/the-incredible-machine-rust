use super::*;

native_part_state!(MouseFrame {
    First = 0,
    Second = 1,
});

const FLEE_TICKS: i16 = 5;
const SHORT_STRIDE_PIXELS: i16 = 3;
const LONG_STRIDE_PIXELS: i16 = 4;
const FRAME_TOGGLE_BIT: u16 = 1;
const CAT_SEARCH_HORIZONTAL: (i16, i16) = (-128, 128);
const CAT_SEARCH_VERTICAL: (i16, i16) = (-8, 8);

/// update_mouse_behavior, 272c:34d0. Supported mice flee the nearest cat
/// for five updates, alternating the original three/four pixel strides.
pub(super) fn update_mouse_behavior(objects: &mut [Object], index: usize) {
    if objects[index].activation_timer == 0 {
        if objects[index].update_flags & UpdateFlag::Supported.mask() != 0 {
            let threat = crate::spatial::nearby_parts(
                objects,
                index,
                CAT_SEARCH_HORIZONTAL,
                CAT_SEARCH_VERTICAL,
            )
            .into_iter()
            .filter(|c| {
                objects[c.index].list == ObjectList::Moving
                    && objects[c.index].definition == Def::Cat
            })
            .min_by_key(|c| i32::from(c.horizontal_gap).abs());
            if let Some(threat) = threat {
                let mouse = &mut objects[index];
                mouse.state = MouseFrame::Second.raw();
                mouse.activation_timer = FLEE_TICKS;
                if threat.horizontal_gap < 1 {
                    mouse
                        .render_flags
                        .insert(crate::machine::RenderFlag::FlipHorizontal);
                    mouse.x = mouse.x.wrapping_add(SHORT_STRIDE_PIXELS);
                } else {
                    mouse
                        .render_flags
                        .remove(crate::machine::RenderFlag::FlipHorizontal);
                    mouse.x = mouse.x.wrapping_sub(SHORT_STRIDE_PIXELS);
                }
            }
        }
    } else {
        let mouse = &mut objects[index];
        mouse.activation_timer = mouse.activation_timer.wrapping_sub(1);
        mouse.state ^= FRAME_TOGGLE_BIT;
        let frame = MouseFrame::from_raw(mouse.state);
        if let Some(frame) = frame {
            mouse.state = frame.raw();
        }
        let stride = match frame {
            Some(MouseFrame::First) => SHORT_STRIDE_PIXELS,
            Some(MouseFrame::Second) | None => LONG_STRIDE_PIXELS,
        };
        mouse.x = mouse.x.wrapping_add(
            if !mouse
                .render_flags
                .contains(crate::machine::RenderFlag::FlipHorizontal)
            {
                -stride
            } else {
                stride
            },
        );
    }
    if objects[index].state != objects[index].previous_state {
        objects[index].motion.position_x =
            i32::from(objects[index].x) << crate::motion::POSITION_FRACTION_BITS;
    }
}
