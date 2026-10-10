//! Native nearby-part query from 1000:3566, using saved integer bounds.
mod facing;
pub use facing::Facing;
mod nearby_contact;
pub use nearby_contact::NearbyContact;

use crate::machine::{Object, ObjectList, RenderFlag, RenderFlags};
use crate::machine::{PropertyFlag, UpdateFlag};
#[path = "../tables/fish_dimensions.rs"]
mod fish_dimensions;
#[path = "../tables/native_extents.rs"]
mod native_extents;

pub fn collision_dimensions(part: &Object) -> (u16, u16) {
    if let Some(extents) = part.extent_override {
        return extents;
    }
    if part.definition == crate::definition::PartDefinition::Fish {
        fish_dimensions::DIMENSIONS
            .get(part.state as usize)
            .copied()
            .unwrap_or((part.width, part.height))
    } else {
        (part.width, part.height)
    }
}

/// `update_object_collision_bounds` (1000:02dd) uses the frame extent fields
/// at +0x44/+0x46 rather than the persistent +0x40/+0x42 body size.
pub fn native_object_extents(part: &Object) -> (u16, u16) {
    if let Some(extents) = part.extent_override {
        return extents;
    }
    if matches!(
        part.definition,
        crate::definition::PartDefinition::Belt | crate::definition::PartDefinition::Rope
    ) {
        return (0, 0);
    }
    if part.update_flags & UpdateFlag::UseAlternateSize.mask() != 0 {
        return part.alt_size;
    }
    if let Some((width, height)) = native_extents::get(
        part.definition.id(),
        part.state,
        part.render_flags.contains(RenderFlag::FlipHorizontal),
    ) {
        return (width as u16, height as u16);
    }
    if part.definition == crate::definition::PartDefinition::Fish {
        return fish_dimensions::DIMENSIONS
            .get(part.state as usize)
            .copied()
            .unwrap_or((part.width, part.height));
    }
    crate::part_extents::FRAMES[part.definition.id() as usize]
        .get(part.state as usize)
        .map(|&(width, height)| (width as u16, height as u16))
        .unwrap_or((part.width, part.height))
}

/// collect_parts_overlapping_rectangle, 1000:36de, strict native bounds.
pub fn collect_parts_overlapping_rectangle(
    objects: &[Object],
    source: usize,
    mask: u16,
    bounds: (i16, i16, i16, i16),
) -> Vec<usize> {
    let a = &objects[source];
    let (left, right, top, bottom) = bounds;
    let mut result = Vec::new();
    for (index, b) in objects.iter().enumerate() {
        if index == source || b.render_flags.contains(crate::machine::RenderFlag::Hidden) {
            continue;
        }
        if !match b.list {
            ObjectList::Moving => mask & 0x1000 != 0,
            ObjectList::Fixed => mask & 0x2000 != 0,
            ObjectList::Inventory => false,
        } {
            continue;
        }
        let (w, h) = native_object_extents(b);
        if i32::from(b.x) < i32::from(a.x) + i32::from(right)
            && i32::from(a.x) + i32::from(left) < i32::from(b.x) + i32::from(w)
            && i32::from(b.y) < i32::from(a.y) + i32::from(bottom)
            && i32::from(a.y) + i32::from(top) < i32::from(b.y) + i32::from(h)
        {
            result.push(index);
        }
    }
    result.reverse();
    result
}

// Native overlapping bounds retain a signed +/-1 gap, rather than zero.
fn nearest_gap(far: i16, near: i16) -> i16 {
    let far_gap = if far >= 0 { -1 } else { far };
    let near_gap = if near <= 0 { 1 } else { near };
    if i32::from(near).abs() < i32::from(far).abs() {
        near_gap
    } else {
        far_gap
    }
}

/// 0x3000 selects fixed and moving lists. Inventory and disabled objects are
/// excluded. Results follow the native prepend order for the supplied traversal.
pub fn nearby_parts(
    objects: &[Object],
    source: usize,
    horizontal: (i16, i16),
    vertical: (i16, i16),
) -> Vec<NearbyContact> {
    let origin = &objects[source];
    let mut contacts = Vec::new();
    for (index, part) in objects.iter().enumerate() {
        if index == source
            || part.list == ObjectList::Inventory
            || part
                .render_flags
                .contains(crate::machine::RenderFlag::Hidden)
        {
            continue;
        }
        let (width, height) = native_object_extents(part);
        let (origin_width, origin_height) = native_object_extents(origin);
        let far_x = part.x.wrapping_add(width as i16).wrapping_sub(origin.x);
        let near_x = part
            .x
            .wrapping_sub(origin.x.wrapping_add(origin_width as i16));
        let far_y = part.y.wrapping_add(height as i16).wrapping_sub(origin.y);
        let near_y = part
            .y
            .wrapping_sub(origin.y.wrapping_add(origin_height as i16));
        if far_x >= horizontal.0
            && near_x <= horizontal.1
            && far_y >= vertical.0
            && near_y <= vertical.1
        {
            contacts.push(NearbyContact {
                index,
                horizontal_gap: nearest_gap(far_x, near_x),
                vertical_gap: nearest_gap(far_y, near_y),
            });
        }
    }
    contacts.reverse();
    contacts
}

/// 1000:38b9 selects combustible ignition points in a closed local rectangle.
pub fn ignition_targets(
    objects: &[Object],
    source: usize,
    bounds: (i16, i16, i16, i16),
) -> Vec<usize> {
    let origin = &objects[source];
    let mut targets = Vec::new();
    for (index, part) in objects.iter().enumerate() {
        if index == source
            || part.list == ObjectList::Inventory
            || part
                .render_flags
                .contains(crate::machine::RenderFlag::Hidden)
            || part.flags & PropertyFlag::Ignitable.mask() == 0
        {
            continue;
        }
        let Some((ax, ay)) = part.ignition_anchor() else {
            continue;
        };
        let x = part.x.wrapping_add(i16::from(ax));
        let y = part.y.wrapping_add(i16::from(ay));
        if origin.x.wrapping_add(bounds.0) <= x
            && x <= origin.x.wrapping_add(bounds.1)
            && origin.y.wrapping_add(bounds.2) <= y
            && y <= origin.y.wrapping_add(bounds.3)
        {
            targets.push(index);
        }
    }
    targets.reverse();
    targets
}
