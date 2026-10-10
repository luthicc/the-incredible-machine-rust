use super::*;
use crate::sound::SoundEffect;

native_part_state!(SeesawPosition {
    Left = 0,
    Level = 1,
    Right = 2,
});

const REMOTE_MATCH: u16 = RopeContactFlag::RemoteDirectionMatch.mask();
const REMOTE_MISMATCH: u16 = RopeContactFlag::RemoteDirectionMismatch.mask();
const PIVOT_LEFT_THRESHOLD_PIXELS: i16 = 37;
const PIVOT_RIGHT_THRESHOLD_PIXELS: i16 = 44;
const CONTACT_PROBE_PIXELS: i16 = 16;
const LAST_PIXEL_FRACTION: i32 = (1 << crate::motion::POSITION_FRACTION_BITS) - 1;
const HORIZONTAL_THROW_SHIFT: u32 = 2;
const LEFT_TILT_QUERY_LINE: ((i32, i32), (i32, i32)) = ((0, 32), (79, 3));
const LEVEL_QUERY_LINE: ((i32, i32), (i32, i32)) = ((0, 17), (79, 17));
const RIGHT_TILT_QUERY_LINE: ((i32, i32), (i32, i32)) = ((0, 3), (79, 32));
const RIGHT_LOWER_NEIGHBOR_BOUNDS: (i16, i16, i16, i16) = (74, 79, 32, 36);
const LEFT_UPPER_NEIGHBOR_BOUNDS: (i16, i16, i16, i16) = (0, 6, -2, 2);
const RIGHT_UPPER_NEIGHBOR_BOUNDS: (i16, i16, i16, i16) = (74, 79, -2, 2);
const LEFT_LOWER_NEIGHBOR_BOUNDS: (i16, i16, i16, i16) = (0, 6, 32, 36);
// (Inclusive upper mass, launch speed in native velocity units.)
const THROW_SPEED_BY_MASS: &[(i16, i16)] = &[
    (1, 7168),
    (5, 6656),
    (9, 6144),
    (20, 5632),
    (120, 5120),
    (150, 4608),
    (i16::MAX, 4096),
];

/// Native 272c:44fe: pass an endpoint tilt through the other attached rope,
/// excluding the caller. Probe impulses are restored before the real event.
pub(super) fn seesaw_rope_contact(
    objects: &mut [Object],
    target: usize,
    caller: usize,
    slot: u8,
    flags: u16,
    path: &mut Vec<usize>,
) -> bool {
    let connection = objects
        .iter()
        .flat_map(|o| o.ropes.iter().flatten())
        .find(|r| {
            r.endpoints
                .iter()
                .enumerate()
                .any(|(side, &end)| end == Some(target) && r.attachment_slots[side] == slot)
        })
        .cloned();
    let slack = connection.as_ref().map_or(0, |r| r.slack_count);
    let adjust_slack = |objects: &mut [Object], delta: i16| {
        if let Some(connection) = &connection {
            for r in objects
                .iter_mut()
                .flat_map(|o| o.ropes.iter_mut().flatten())
            {
                if r.endpoints == connection.endpoints
                    && r.attachment_slots == connection.attachment_slots
                {
                    r.slack_count = r.slack_count.wrapping_add(delta);
                }
            }
        }
    };
    if flags & RopeContactFlag::DIRECTION_MASK != RopeContactFlag::LocalDirectionMatch.mask()
        && slack != 0
    {
        if flags & RopeContactFlag::ProbeOnly.mask() == 0 {
            adjust_slack(objects, -1);
        }
        return false;
    }
    if flags & RopeContactFlag::DIRECTION_MASK == RopeContactFlag::LocalDirectionMatch.mask() {
        if objects[target]
            .render_flags
            .contains(crate::machine::RenderFlag::CallbackBlocked)
        {
            return true;
        }
        if flags & RopeContactFlag::ProbeOnly.mask() == 0 {
            adjust_slack(objects, 1);
        }
        return false;
    }
    let state = objects[target].state;
    let direction = match (flags & RopeContactFlag::DIRECTION_MASK, slot) {
        (REMOTE_MATCH, 0) | (REMOTE_MISMATCH, 1) => {
            (state != SeesawPosition::Left.raw()).then_some(-1)
        }
        (REMOTE_MISMATCH, 0) | (REMOTE_MATCH, 1) => {
            (state != SeesawPosition::Right.raw()).then_some(1)
        }
        _ => None,
    };
    let blocked = if let Some(direction) = direction {
        let old = objects[target].power_state;
        objects[target].power_state = direction;
        let blocked = dispatch_rope_contact(
            objects,
            target,
            Some(caller),
            flags & RopeContactFlag::ProbeOnly.mask(),
            path,
        );
        if flags & RopeContactFlag::ProbeOnly.mask() != 0 {
            objects[target].power_state = old;
        } else if !blocked {
            objects[target]
                .render_flags
                .insert(crate::machine::RenderFlag::RopeProbeBypassed);
        }
        blocked
    } else {
        true
    };
    if blocked {
        objects[target]
            .render_flags
            .insert(crate::machine::RenderFlag::CallbackBlocked);
    }
    objects[target]
        .render_flags
        .contains(crate::machine::RenderFlag::CallbackBlocked)
}

/// Native 272c:3fe8 edge/pivot/state eligibility. Native 1000:7b6f
/// impulse arbitration must additionally accept before the original commits.
pub(super) fn seesaw_contact_direction(
    seesaw: &Object,
    edge: u16,
    moving_center: i16,
) -> Option<i16> {
    if seesaw
        .render_flags
        .contains(crate::machine::RenderFlag::CallbackBlocked)
    {
        return None;
    }
    match edge {
        0 => {
            let center = moving_center.wrapping_sub(seesaw.x);
            if center < PIVOT_LEFT_THRESHOLD_PIXELS && seesaw.state != SeesawPosition::Left.raw() {
                Some(-1)
            } else if center >= PIVOT_RIGHT_THRESHOLD_PIXELS
                && seesaw.state != SeesawPosition::Right.raw()
            {
                Some(1)
            } else {
                None
            }
        }
        2 if seesaw.state != SeesawPosition::Left.raw() => Some(-1),
        6 if seesaw.state != SeesawPosition::Right.raw() => Some(1),
        _ => None,
    }
}

/// Native 272c:471f mass thresholds; these are assigned velocities.
pub(super) fn seesaw_throw_speed(definition: Def) -> i16 {
    seesaw_throw_speed_for_mass(crate::forces::mass(definition))
}

pub(super) fn seesaw_throw_speed_for_mass(mass: i16) -> i16 {
    launch_speed_for_mass(mass, THROW_SPEED_BY_MASS)
}

/// Segment crossing adapter for 1000:3782's outline query. The native 3ba9
/// routine computes and truncates the integer intersection before checking
/// inclusive bounds; exact cross-product tests miss contacts that truncate
/// onto an outline vertex.
pub(super) fn segments_cross(a: (i32, i32), b: (i32, i32), c: (i32, i32), d: (i32, i32)) -> bool {
    crate::contact::intersect_line_segments((a, b), (c, d)).is_some()
}

pub(super) fn throw_seesaw_crossing_parts(objects: &mut [Object], index: usize, power: i16) {
    let mut thrown = Vec::new();
    // DS 3542: the native query line differs from the rope attachment table.
    let (a, b) = match SeesawPosition::from_raw(objects[index].state) {
        Some(SeesawPosition::Left) => LEFT_TILT_QUERY_LINE,
        Some(SeesawPosition::Level) => LEVEL_QUERY_LINE,
        Some(SeesawPosition::Right) => RIGHT_TILT_QUERY_LINE,
        _ => return,
    };
    let origin = (i32::from(objects[index].x), i32::from(objects[index].y));
    let a = (a.0 + origin.0, a.1 + origin.1);
    let b = (b.0 + origin.0, b.1 + origin.1);
    let pivot = objects[index]
        .x
        .wrapping_add((objects[index].width >> 1) as i16);
    for (other, part) in objects.iter_mut().enumerate() {
        if other == index
            || part.list != ObjectList::Moving
            || part
                .render_flags
                .contains(crate::machine::RenderFlag::Hidden)
        {
            continue;
        }
        let Some(outline) = part.outline.as_ref() else {
            continue;
        };
        let hit = outline
            .iter()
            .zip(outline.iter().cycle().skip(1))
            .any(|(&(x, y), &(ex, ey))| {
                segments_cross(
                    a,
                    b,
                    (
                        i32::from(part.x) + i32::from(x),
                        i32::from(part.y) + i32::from(y),
                    ),
                    (
                        i32::from(part.x) + i32::from(ex),
                        i32::from(part.y) + i32::from(ey),
                    ),
                )
            });
        if !hit {
            continue;
        }
        let speed = seesaw_throw_speed(part.definition);
        let left = part.x.wrapping_add((part.width >> 1) as i16) < pivot;
        if power == -1 || power == 1 {
            part.motion.velocity_x = if left {
                -(speed >> HORIZONTAL_THROW_SHIFT)
            } else {
                speed >> HORIZONTAL_THROW_SHIFT
            };
            part.motion.velocity_y = if (power == -1 && left) || (power == 1 && !left) {
                speed
            } else {
                -speed
            };
            thrown.push(other);
        }
    }
    // 272c:420f probes both directions across the newly tilted beam before
    // committing the launch position. The second probe hides the seesaw.
    for other in thrown {
        let rising = objects[other].motion.velocity_y < 0;
        objects[other].previous_position.1 = objects[other].y.wrapping_add(if rising {
            -CONTACT_PROBE_PIXELS
        } else {
            CONTACT_PROBE_PIXELS
        });
        crate::contact::update_object_contacts(objects, other);
        objects[other].previous_position.1 = objects[other].y.wrapping_add(if rising {
            CONTACT_PROBE_PIXELS
        } else {
            -CONTACT_PROBE_PIXELS
        });
        let flags = objects[index].render_flags;
        objects[index]
            .render_flags
            .insert(crate::machine::RenderFlag::Hidden);
        crate::contact::update_object_contacts(objects, other);
        objects[index].render_flags = flags;
        objects[other].previous_position.1 = objects[other].y;
        objects[other].motion.position_y = (i32::from(objects[other].y)
            << crate::motion::POSITION_FRACTION_BITS)
            + if rising { 0 } else { LAST_PIXEL_FRACTION };
    }
}

/// Ghidra 272c:420f. Apply the seesaw's signed impulse and dispatch its rope
/// endpoints before changing state, as the original update callback does.
#[allow(non_snake_case)]
pub(super) fn apply_seesaw_tilt_and_throw_parts(objects: &mut [Object], index: usize) {
    let power = objects[index].power_state;
    if power == 0 {
        // update_seesaw (272c:420f) runs its fixed-neighbor query on the
        // update after the beam reaches an endpoint. The native state history
        // at that point is current == previous, with older still at the
        // departed state; 272c:477d dispatches the selected fixed callbacks.
        if SeesawPosition::from_raw(objects[index].state) == Some(SeesawPosition::Right)
            && SeesawPosition::from_raw(objects[index].previous_state)
                == Some(SeesawPosition::Right)
            && SeesawPosition::from_raw(objects[index].older_state) != Some(SeesawPosition::Right)
        {
            dispatch_seesaw_fixed_neighbors(objects, index, RIGHT_LOWER_NEIGHBOR_BOUNDS, 1);
            // The native endpoint query's lower/right corner reports edge 3
            // to flipped neighbors (not edge 0); Bellows uses that edge to
            // compress in FUN_272c_0332.
            dispatch_seesaw_fixed_neighbors(objects, index, LEFT_UPPER_NEIGHBOR_BOUNDS, 3);
        } else if SeesawPosition::from_raw(objects[index].state) == Some(SeesawPosition::Left)
            && SeesawPosition::from_raw(objects[index].previous_state) == Some(SeesawPosition::Left)
            && SeesawPosition::from_raw(objects[index].older_state) != Some(SeesawPosition::Left)
        {
            dispatch_seesaw_fixed_neighbors(objects, index, RIGHT_UPPER_NEIGHBOR_BOUNDS, 0);
            dispatch_seesaw_fixed_neighbors(objects, index, LEFT_LOWER_NEIGHBOR_BOUNDS, 1);
        }
        return;
    }
    objects[index]
        .render_flags
        .insert(crate::machine::RenderFlag::ProcessedThisUpdate);
    if !objects[index]
        .render_flags
        .contains(crate::machine::RenderFlag::RopeProbeBypassed)
    {
        if DispatchConnectedRopeContact(objects, index, RopeContactFlag::ProbeOnly.mask()) {
            objects[index]
                .render_flags
                .insert(crate::machine::RenderFlag::CallbackBlocked);
            objects[index].power_state = 0;
            objects[index].collision_impulse = 0;
            return;
        }
        DispatchConnectedRopeContact(objects, index, 0);
    }
    let next_raw = (objects[index].state as i16 + power).clamp(
        SeesawPosition::Left.raw() as i16,
        SeesawPosition::Right.raw() as i16,
    ) as u16;
    let next_position = SeesawPosition::from_raw(next_raw)
        .expect("native seesaw movement is clamped to its three positions");
    let next_state = next_position.raw();
    objects[index].state = next_state;
    objects[index].outline = crate::outline::vertices(&objects[index]);
    objects[index].outline_angles = objects[index]
        .outline
        .as_deref()
        .map(crate::outline::edge_angles)
        .unwrap_or_default();
    if let Some(anchors) = crate::outline::seesaw_anchors(next_state) {
        objects[index].rope_anchors = anchors;
    }
    if std::env::var_os("TIM_TRACE_MOTION").is_some() {
        eprintln!(
            "apply_seesaw_tilt_and_throw_parts: state {} -> {}",
            objects[index].previous_state, next_state
        );
    }
    if next_state != objects[index].previous_state {
        if matches!(
            SeesawPosition::from_raw(objects[index].previous_state),
            Some(SeesawPosition::Left | SeesawPosition::Right)
        ) {
            crate::sound::play_effect(SoundEffect::MechanismMovement);
        }
        throw_seesaw_crossing_parts(objects, index, power);
    }
    objects[index].power_state = 0;
    objects[index].collision_impulse = 0;
}

/// Seesaw endpoint queries and fixed callbacks, 272c:420f / 1000:36de /
/// 272c:477d. The rectangle is strict on all four edges, like 1000:36de.
pub(super) fn dispatch_seesaw_fixed_neighbors(
    objects: &mut [Object],
    index: usize,
    bounds: (i16, i16, i16, i16),
    contact_edge: u16,
) {
    let source = &objects[index];
    let (left, right, top, bottom) = bounds;
    let candidates: Vec<_> = objects
        .iter()
        .enumerate()
        .filter_map(|(candidate, part)| {
            if candidate == index
                || part.list != ObjectList::Fixed
                || part
                    .render_flags
                    .contains(crate::machine::RenderFlag::Hidden)
            {
                return None;
            }
            let (width, height) = crate::spatial::native_object_extents(part);
            let overlaps = part.x < source.x.wrapping_add(right)
                && source.x.wrapping_add(left) < part.x.wrapping_add(width as i16)
                && part.y < source.y.wrapping_add(bottom)
                && source.y.wrapping_add(top) < part.y.wrapping_add(height as i16);
            overlaps.then_some(candidate)
        })
        .collect();
    for candidate in candidates {
        match objects[candidate].definition {
            Def::CatPoweredDrive => activate_contacted_object(&mut objects[candidate]),
            Def::Fish => {
                advance_fish_state(&mut objects[candidate]);
            }
            Def::Bellows => {
                bellows_contact(&mut objects[candidate], contact_edge);
            }
            Def::Scissors => {
                process_scissors_contact(objects, index, candidate, contact_edge);
            }
            _ => {}
        }
    }
}

#[cfg(test)]
#[path = "seesaw_tests.rs"]
mod mass_curve_tests;
