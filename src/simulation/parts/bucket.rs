use super::*;

const EFFECTIVE_MASS_CAP: i32 = 32000;
const MOUTH_LEFT_OFFSET: i16 = 4;
const MOUTH_RIGHT_OFFSET: i16 = 32;
const CONTENTS_TOP_OFFSET: i16 = 20;

/// Native 1000:7c5b adds signed effective masses and caps positive overflow.
pub(super) fn add_effective_mass(a: i16, b: i16) -> i16 {
    let sum = i32::from(a) + i32::from(b);
    if sum > EFFECTIVE_MASS_CAP {
        EFFECTIVE_MASS_CAP as i16
    } else {
        sum as i16
    }
}

/// check_bucket_contact_height (272c:0763): a falling part strictly inside
/// the mouth is carried instead of bounced. Native +0x22 is previous pixel X.
pub(super) fn check_bucket_contact_height(object: &Object, bucket: &Object) -> bool {
    let center = object
        .previous_position
        .0
        .wrapping_add((crate::spatial::collision_dimensions(object).0 >> 1) as i16);
    let left = bucket.previous_position.0;
    object.motion.velocity_y <= 0
        || center <= left.wrapping_add(MOUTH_LEFT_OFFSET)
        || left.wrapping_add(MOUTH_RIGHT_OFFSET) <= center
}

/// First bucket phase from 1000:3972 / 7c3a. Geometry uses saved pixel
/// positions (+22/+24); contained parts inherit bucket velocity.
pub(super) fn collect_bucket_load(objects: &mut [Object], bucket: usize) -> Vec<usize> {
    objects[bucket].next_part = None;
    // The query runs after the moving-list update, but reads each saved
    // previous position from the native object record.
    let (bx, by) = objects[bucket].previous_position;
    let left = bx.wrapping_add(MOUTH_LEFT_OFFSET);
    let right = bx.wrapping_add(MOUTH_RIGHT_OFFSET);
    // FUN_1000_3972 reads runtime collision extents (+0x44/+0x46), which can
    // differ from the persistent sprite size for animated/trimmed objects.
    let bucket_height = crate::spatial::native_object_extents(&objects[bucket]).1;
    let bottom = by.wrapping_add(bucket_height as i16);
    let velocity = (
        objects[bucket].motion.velocity_x,
        objects[bucket].motion.velocity_y,
    );
    let mut contents = Vec::new();
    for (i, part) in objects.iter().enumerate() {
        if i == bucket
            || part.list != ObjectList::Moving
            || part
                .render_flags
                .contains(crate::machine::RenderFlag::Hidden)
            || part.definition == Def::BirdCage
        {
            continue;
        }
        let center = part
            .previous_position
            .0
            .wrapping_add((part.width >> 1) as i16);
        if !(left < center && center < right) {
            continue;
        }
        let supported =
            part.contact.is_some_and(|c| c.target == bucket) && part.motion.velocity_y > 0;
        let part_height = crate::spatial::native_object_extents(part).1;
        let foot = part.previous_position.1.wrapping_add(part_height as i16);
        if supported || (by.wrapping_add(CONTENTS_TOP_OFFSET) < foot && foot < bottom) {
            contents.push(i);
        }
    }
    // Native query prepends. Collection sets the carried list and velocity;
    // accumulate_linked_mass is a separate, once-per-update phase.
    contents.reverse();
    // is_object_linked (1000:3a61) walks this chain when scanning the
    // bucket's contacts, so its own load cannot block a rope correction.
    for &i in contents.iter().rev() {
        objects[i].next_part = objects[bucket].next_part;
        objects[bucket].next_part = Some(i);
        objects[i].flags |= PropertyFlag::CarriedByBucket.mask();
        objects[i].motion.velocity_x = velocity.0;
        objects[i].motion.velocity_y = velocity.1;
    }
    contents
}

/// accumulate_linked_mass / add_linked_mass (1000:7c3a / 7c5b).
pub(super) fn accumulate_bucket_mass(objects: &mut [Object], bucket: usize, contents: &[usize]) {
    for &i in contents {
        objects[bucket].effective_mass =
            add_effective_mass(objects[bucket].effective_mass, objects[i].effective_mass);
    }
}

/// move_linked_parts_with_parent (1000:3a8d): apply the bucket's corrected
/// pixel displacement to its contents after rope response and integration.
pub(super) fn move_bucket_contents(objects: &mut [Object], bucket: usize, contents: &[usize]) {
    let dx = objects[bucket]
        .x
        .wrapping_sub(objects[bucket].previous_position.0);
    let dy = objects[bucket]
        .y
        .wrapping_sub(objects[bucket].previous_position.1);
    if dx == 0 && dy == 0 {
        return;
    }
    for &i in contents {
        let part = &mut objects[i];
        part.x = part.x.wrapping_add(dx);
        part.y = part.y.wrapping_add(dy);
        part.motion.position_x = i32::from(part.x) << crate::motion::POSITION_FRACTION_BITS;
        part.motion.position_y = i32::from(part.y) << crate::motion::POSITION_FRACTION_BITS;
    }
}
