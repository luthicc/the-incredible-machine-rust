use super::*;

const FRAME_FUSE_END: u16 = 5;
const EXPLOSION_X_OFFSET: i16 = -15;
const EXPLOSION_Y_OFFSET: i16 = -19;

/// 272c:12c2 and 1328; definition-41 allocation defaults at 3d3c:2bf6.
pub(super) fn update_dynamite_fuse(objects: &mut Vec<Object>, index: usize) {
    if objects[index].power_state == 0 && objects[index].callback_timer > IGNITION_HEAT_THRESHOLD {
        objects[index].power_state = 1;
    }
    if objects[index].power_state == 0 {
        return;
    }
    if objects[index].state != FRAME_FUSE_END {
        objects[index].state = objects[index].state.wrapping_add(1);
        return;
    }
    detonate_dynamite(objects, index);
}

pub(super) fn detonate_dynamite(objects: &mut Vec<Object>, index: usize) {
    // FUN_272c_1328 sets state 5 and hides the dynamite without calling
    // update_object_anchor_offsets (1000:5be4). The native +0x44/+0x46
    // collision extents therefore stay at the last frame that was loaded.
    // Preserve those live extents before state 5 would select a different
    // bitmap frame in native_object_extents().
    let live_extents = crate::spatial::native_object_extents(&objects[index]);
    objects[index].extent_override = Some(live_extents);
    objects[index].state = FRAME_FUSE_END;
    let x = objects[index].x.wrapping_add(EXPLOSION_X_OFFSET);
    let y = objects[index].y.wrapping_add(EXPLOSION_Y_OFFSET);
    spawn_explosion_at(objects, x, y);
    objects[index]
        .render_flags
        .insert(crate::machine::RenderFlag::Hidden);
}
