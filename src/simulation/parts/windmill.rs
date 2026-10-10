use super::*;

native_part_state!(WindmillFrame {
    First = 0,
    Second = 1,
    Third = 2,
    Fourth = 3,
});

pub(super) const WINDMILL_COAST_TICKS: i16 = 20;
const ROTATION_FRAME_MASK: u16 = WindmillFrame::Fourth.raw();

pub(super) fn update_windmill_rotation(objects: &mut [Object], index: usize) {
    let (power, direction) = {
        let object = &mut objects[index];
        object.power_state = 0;
        if object.callback_timer > 0 {
            object.callback_timer -= 1;
            if object.callback_timer > 0 {
                object.power_state = 1;
            }
        }
        if object.power_state != 0 {
            let next = (object.state + 1) & ROTATION_FRAME_MASK;
            object.state = WindmillFrame::from_raw(next)
                .expect("windmill rotation selects one of four native frames")
                .raw();
        }
        (
            object.power_state,
            object
                .render_flags
                .contains(crate::machine::RenderFlag::FlipHorizontal),
        )
    };
    if let Some(partner) = belt_partner(objects, index) {
        if !objects[partner]
            .render_flags
            .contains(crate::machine::RenderFlag::PowerUpdateCallback)
        {
            objects[partner].power_state = if direction { -power } else { power };
        }
    }
}
