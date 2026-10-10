//! Original mass thresholds and fixed-point blast projection.
use crate::{forces, machine::Object};
#[path = "../tables/trig_table.rs"]
mod table;
pub fn magnitude(mass: i16) -> i16 {
    match mass {
        ..=1 => 0x1800,
        2..=5 => 0x1600,
        6..=9 => 0x1400,
        10..=20 => 0x1200,
        21..=120 => 0x1000,
        121..=150 => 0xe00,
        151..=200 => 0xc00,
        201..=1800 => 0xa00,
        _ => 0x800,
    }
}
fn component(angle: u16) -> i16 {
    let mut index = usize::from(angle >> 4);
    if index & 0x800 != 0 {
        index = 0x1000 - index;
    }
    table::COSINE[index]
}
fn object_extent(object: &Object) -> (i16, i16) {
    // FUN_1000_3da5 derives explosion direction from the live +0x44/+0x46
    // collision extents. Animation frame tables are only the initializer for
    // those fields; some callbacks deliberately leave them at an older frame.
    let (width, height) = crate::spatial::native_object_extents(object);
    (width as i16, height as i16)
}
/// 1000:3da5 uses object centers; 1000:7223 replaces velocity and shifts
/// signed products by 14 (CL=0x0e), preserving native table rounding.
pub fn impulse(source: &Object, target: &Object) -> (i16, i16) {
    let (source_width, source_height) = object_extent(source);
    let (target_width, target_height) = object_extent(target);
    let dx = source
        .x
        .wrapping_add(source_width >> 1)
        .wrapping_sub(target.x.wrapping_add(target_width >> 1));
    let dy = target
        .y
        .wrapping_add(target_height >> 1)
        .wrapping_sub(source.y.wrapping_add(source_height >> 1));
    let angle = crate::outline::vector_angle(dx, dy);
    let speed = i32::from(magnitude(forces::mass(target.definition)));
    (
        (speed * i32::from(component(angle.wrapping_add(0xc000))) >> 14) as i16,
        (speed * i32::from(component(angle)) >> 14) as i16,
    )
}
#[cfg(test)]
mod tests;
