//! Integer outline sweeps from TIM 1000:0556, 07af and 0b6c/0d37.
//! Contact detection uses directed native edges, not a convex hull.
use crate::machine::UpdateFlag;
use crate::{
    definition::PartDefinition as Def,
    machine::{ContactRecord, Object, ObjectList, RenderFlag},
};
type Point = (i32, i32);
fn sub(a: Point, b: Point) -> Point {
    (a.0 - b.0, a.1 - b.1)
}
fn add(a: Point, b: Point) -> Point {
    (a.0 + b.0, a.1 + b.1)
}
fn vertex(p: (u8, u8)) -> Point {
    (i32::from(p.0), i32::from(p.1))
}
fn signed(a: u16) -> i16 {
    a as i16
}
fn between(n: i32, a: i32, b: i32) -> bool {
    n >= a.min(b) && n <= a.max(b)
}

/// in_range, 1000:3d67: native signed branch followed by unsigned 16-bit
/// distance comparisons (not an ordinary signed min/max test).
fn native_in_range(point: i32, start: i32, end: i32) -> bool {
    let (point, start, end) = (point as i16, start as i16, end as i16);
    if end < start {
        (start.wrapping_sub(end) as u16) >= (point.wrapping_sub(end) as u16)
    } else {
        (point.wrapping_sub(start) as u16) <= (end.wrapping_sub(start) as u16)
    }
}

/// 1000:3ba9: integer intersection, inclusive ordered coordinate bounds.
pub fn intersect_line_segments(a: (Point, Point), b: (Point, Point)) -> Option<Point> {
    let ay = a.0.1 - a.1.1;
    let ax = a.0.0 - a.1.0;
    let ac = a.1.0 * ay - a.1.1 * ax;
    let by = b.1.1 - b.0.1;
    let bx = b.1.0 - b.0.0;
    let bc = b.0.0 * by - b.0.1 * bx;
    let divisor = by * ax - ay * bx;
    let p = if divisor == 0 {
        if a.0.0 * by + a.0.1 * bx == 0 {
            a.1
        } else {
            (0, 0)
        }
    } else {
        (
            ((i64::from(bc) * i64::from(ax) - i64::from(ac) * i64::from(bx)) / i64::from(divisor))
                as i32,
            ((i64::from(ay) * i64::from(bc) - i64::from(by) * i64::from(ac)) / i64::from(divisor))
                as i32,
        )
    };
    (between(p.0, a.0.0, a.1.0)
        && between(p.0, b.0.0, b.1.0)
        && between(p.1, a.0.1, a.1.1)
        && between(p.1, b.0.1, b.1.1))
    .then_some(p)
}
/// 1000:3d2e.
fn update_line_segment_endpoint(p: Point) -> Point {
    (p.0 + p.0.signum(), p.1 + p.1.signum())
}
/// 1000:04d1.
fn get_coordinate_wrap_quadrant(a: u16) -> usize {
    if a == 0x2000 {
        0
    } else if a == 0xa000 {
        2
    } else {
        ((a.wrapping_add(0x2000) >> 14) & 3) as usize
    }
}
/// 1000:03df; previous contact limits rolling support to matching half slopes.
fn are_coordinates_in_same_wrap_quadrant(a: u16, prior: Option<ContactRecord>) -> bool {
    let Some(prior) = prior else {
        return false;
    };
    let b = prior.angle;
    if get_coordinate_wrap_quadrant(a) != get_coordinate_wrap_quadrant(b) {
        return false;
    }
    let (mut x, mut y) = (
        signed(a.wrapping_add(0x2000)),
        signed(b.wrapping_add(0x2000)),
    );
    if !(0..=0x4000).contains(&x) || !(0..=0x4000).contains(&y) {
        x = signed(a.wrapping_sub(0x6000));
        y = signed(b.wrapping_sub(0x6000));
        if !(0..=0x4000).contains(&x) || !(0..=0x4000).contains(&y) {
            return false;
        }
    }
    a == b || x == 0x2000 || y == 0x2000 || (x < 0x2000 && y < 0x2000) || (x > 0x2000 && y > 0x2000)
}
/// 1000:04fd, records center relative to the directed contact segment.
fn determine_contact_side_flags(x: i32, start: i32, end: i32) -> (bool, bool) {
    if native_in_range(x, start, end) {
        (true, true)
    } else if (start < end && x < start) || (start >= end && x >= start) {
        (false, true)
    } else {
        (true, false)
    }
}
const DIRECTIONS: [Point; 4] = [(0, -1), (-1, 0), (0, 1), (1, 0)]; // DS 258c/2594
fn eligible_vertex(before: u16, after: u16, edge: u16) -> bool {
    let a = signed(before.wrapping_sub(edge).wrapping_sub(0x8000));
    let b = signed(after.wrapping_sub(edge).wrapping_sub(0x8000));
    (a >= 0 || a == i16::MIN) && b <= 0
}
fn position(p: &Object) -> Point {
    (i32::from(p.x), i32::from(p.y))
}
fn motion_angle(delta: Point) -> u16 {
    crate::outline::vector_angle(-delta.0 as i16, delta.1 as i16)
}
fn correct_position(
    p: &mut Object,
    path: (Point, Point),
    line: (Point, Point),
    same: bool,
    reverse: bool,
) {
    let intersection = if same {
        let delta = sub(line.1, line.0);
        if delta.0 == 0 {
            None
        } else {
            Some((
                path.1.0,
                (delta.1 * line.0.0 - delta.0 * line.0.1 - delta.1 * path.1.0) / -delta.0,
            ))
        }
    } else {
        intersect_line_segments(path, line)
    };
    if let Some(hit) = intersection {
        let correction = sub(hit, path.1);
        let sign = if reverse { -1 } else { 1 };
        if !same {
            p.x = p.x.wrapping_add((sign * correction.0) as i16);
        }
        p.y = p.y.wrapping_add((sign * correction.1) as i16);
    } else {
        (p.x, p.y) = p.previous_position;
    }
}
fn record(
    p: &mut Object,
    target: &Object,
    index: usize,
    edge: usize,
    angle: u16,
    flags: (bool, bool),
) {
    p.update_flags &= !UpdateFlag::CONTACT_KIND_MASK;
    p.update_flags |= if !(p.render_flags | target.render_flags)
        .contains(RenderFlag::ForceSurfaceCollision)
        && target.update_flags & UpdateFlag::FixedDefinition.mask() == 0
    {
        UpdateFlag::MovingContact.mask()
    } else {
        UpdateFlag::SurfaceContact.mask()
    };
    // FUN_1000_04fd writes a 1 to either side byte when a swept edge is
    // outside its span; it never clears the other side during this scan.
    // Several contacts may be found before update_object_contacts returns,
    // so preserve side hits while keeping the most recent target/edge data.
    let (flag_a, flag_b) = p.contact.map_or(flags, |prior| {
        (prior.flag_a || flags.0, prior.flag_b || flags.1)
    });
    p.contact = Some(ContactRecord {
        target: index,
        outline_edge: edge as u16,
        angle,
        flag_a,
        flag_b,
    });
}
/// 1000:07af: moving vertices swept against target edges, in native order.
fn test_outline_contact_forward(
    p: &mut Object,
    b: &Object,
    index: usize,
    prior: Option<ContactRecord>,
) -> bool {
    let Some(bp) = b.outline.as_ref() else {
        return false;
    };
    let Some(ap) = p.outline.clone() else {
        return false;
    };
    let count = if b.definition == Def::PowerDistributor {
        1
    } else {
        bp.len()
    };
    // FUN_1000_07af uses the sweep angle captured by update_object_contacts
    // for every target edge, even after an earlier edge corrects p.x/p.y.
    let sweep_delta = sub(
        position(p),
        (
            i32::from(p.previous_position.0),
            i32::from(p.previous_position.1),
        ),
    );
    let sweep_angle = motion_angle(sweep_delta);
    let moving_vertex_count = if p.definition == Def::Bullet && p.state >= 1 {
        1
    } else {
        ap.len()
    };
    let mut found = false;
    for edge in 0..count {
        let angle = b.outline_angles[edge];
        if signed(sweep_angle.wrapping_sub(angle).wrapping_add(0x4000)) <= 0 {
            continue;
        }
        let origin = add(position(b), vertex(bp[edge]));
        let end =
            update_line_segment_endpoint(sub(vertex(bp[(edge + 1) % bp.len()]), vertex(bp[edge])));
        for v in (1..=moving_vertex_count).map(|v| v % ap.len()) {
            if !eligible_vertex(
                p.outline_angles[(v + ap.len() - 1) % ap.len()],
                p.outline_angles[v],
                angle,
            ) {
                continue;
            }
            let delta = sub(
                position(p),
                (
                    i32::from(p.previous_position.0),
                    i32::from(p.previous_position.1),
                ),
            );
            if delta == (0, 0) {
                continue;
            }
            let start = sub(
                add(
                    (
                        i32::from(p.previous_position.0),
                        i32::from(p.previous_position.1),
                    ),
                    vertex(ap[v]),
                ),
                origin,
            );
            let path = (start, add(start, delta));
            let Some(hit) = intersect_line_segments(path, ((0, 0), end)) else {
                continue;
            };
            if hit == end {
                continue;
            }
            let shift = DIRECTIONS[get_coordinate_wrap_quadrant(angle)];
            let line = (shift, add(end, shift));
            correct_position(
                p,
                path,
                line,
                are_coordinates_in_same_wrap_quadrant(angle, prior),
                false,
            );
            let center = i32::from(p.x)
                + i32::from(crate::spatial::native_object_extents(p).0 >> 1)
                - origin.0;
            let side_flags = determine_contact_side_flags(center, line.0.0, line.1.0);
            record(p, b, index, edge, angle, side_flags);
            found = true;
        }
    }
    found
}
/// 1000:0b6c/0d37: target vertices swept backward against moving edges.
fn test_outline_contact_backward(
    p: &mut Object,
    b: &Object,
    index: usize,
    prior: Option<ContactRecord>,
) -> bool {
    let Some(bp) = b.outline.as_ref() else {
        return false;
    };
    let Some(ap) = p.outline.clone() else {
        return false;
    };
    if b.definition == Def::PowerDistributor {
        return false;
    }
    let saved = position(p);
    let mut found = false;
    let moving_edge_count = if p.definition == Def::Bullet && p.state >= 1 {
        1
    } else {
        ap.len()
    };
    // 1000:0b6c holds the movement angle for this sweep. Collision bounds
    // refresh the displacement used by later edge paths after a correction.
    let sweep_delta = sub(
        saved,
        (
            i32::from(p.previous_position.0),
            i32::from(p.previous_position.1),
        ),
    );
    let sweep_angle = motion_angle(sweep_delta);
    let mut edge_origin = add(saved, vertex(ap[0]));
    for edge in 0..moving_edge_count {
        let angle = p.outline_angles[edge];
        let edge_end = add(position(p), vertex(ap[(edge + 1) % ap.len()]));
        let origin = edge_origin;
        let edge_delta = sub(edge_end, origin);
        // The native loop carries the preceding edge endpoint into the next
        // edge and rebuilds that next endpoint from the refreshed bounds.
        edge_origin = edge_end;
        if signed(sweep_angle.wrapping_sub(angle).wrapping_sub(0x4000)) <= 0 {
            continue;
        }
        let end = update_line_segment_endpoint(edge_delta);
        for v in (1..=bp.len()).map(|v| v % bp.len()) {
            if !eligible_vertex(
                b.outline_angles[(v + bp.len() - 1) % bp.len()],
                b.outline_angles[v],
                angle,
            ) {
                continue;
            }
            let delta = sub(
                position(p),
                (
                    i32::from(p.previous_position.0),
                    i32::from(p.previous_position.1),
                ),
            );
            if delta == (0, 0) {
                continue;
            }
            let finish = sub(add(position(b), vertex(bp[v])), origin);
            let path = (add(finish, delta), finish);
            let Some(hit) = intersect_line_segments(path, ((0, 0), end)) else {
                continue;
            };
            if hit == end {
                continue;
            }
            // The backward sweep records side flags from the original
            // intersection and the pre-correction center (+0x24 at this
            // point in 1000:0b6c/0d37). Position correction happens after
            // those locals are captured in the native routine.
            let center = i32::from(saved.0)
                + i32::from(crate::spatial::native_object_extents(p).0 >> 1)
                - origin.0;
            // The native backward sweep stores the moving edge angle minus
            // half a turn (1000:0b6c/0d37); endpoint hits do not substitute
            // the adjacent vertex angle.
            let contact_angle = angle.wrapping_sub(0x8000);
            let d = DIRECTIONS[get_coordinate_wrap_quadrant(contact_angle)];
            let shift = (-d.0, -d.1);
            let line = (shift, add(end, shift));
            correct_position(
                p,
                path,
                line,
                are_coordinates_in_same_wrap_quadrant(contact_angle, prior),
                true,
            );
            let flag_a = if end.0 < 0 {
                hit.0 < center
            } else {
                hit.0 >= center
            };
            let side_flags = if p.definition == Def::Bullet && p.state != 0 {
                // The native spent bullet has outline_vertex_count=0 but
                // retains its outline pointer; its reverse-contact side
                // bytes are written in the opposite order.
                (!flag_a, flag_a)
            } else {
                (flag_a, !flag_a)
            };
            record(
                p,
                b,
                index,
                (v + bp.len() - 1) % bp.len(),
                contact_angle,
                side_flags,
            );
            found = true;
        }
    }
    found
}
/// get_object_list_head_for_mask(0x3000) order: the fixed list, which
/// insert_object_into_sorted_list prepends to (newest first), then the moving
/// list, kept sorted by definition mass with ties in insertion order.
pub fn native_list_order(objects: &[Object]) -> Vec<usize> {
    let mut order: Vec<usize> = (0..objects.len())
        .filter(|&i| objects[i].list == ObjectList::Fixed)
        .rev()
        .collect();
    let mut moving: Vec<usize> = (0..objects.len())
        .filter(|&i| objects[i].list == ObjectList::Moving)
        .collect();
    moving.sort_by_key(|&i| crate::forces::mass(objects[i].definition));
    order.extend(moving);
    order
}

/// 1000:0556/0687: previous target first, then fixed and moving lists.
pub fn update_object_contacts(objects: &mut [Object], index: usize) {
    rescan_object_contacts(objects, index);
}

/// is_object_linked (1000:3a61): a bucket excludes its collected load
/// from contact scans. This is directional; the load still contacts its bucket.
fn is_object_linked(objects: &[Object], part: usize, target: usize) -> bool {
    if objects[part].definition != Def::Bucket {
        return false;
    }
    let mut next = objects[part].next_part;
    for _ in 0..objects.len() {
        let Some(index) = next else {
            return false;
        };
        if index == target {
            return true;
        }
        next = objects[index].next_part;
    }
    false
}

/// update_object_contacts (1000:0556) does not itself clear the response
/// nibble. simulate_object_update (1000:1216) clears it before the first
/// scan and before a strained-rope rescan, but retains it for a slack rope.
pub(crate) fn rescan_object_contacts(objects: &mut [Object], index: usize) {
    // Impacted bullets retain their outline pointer in DOS with a zero active
    // vertex count; the scan helpers emulate the one first-edge probe.
    let prior = objects[index].contact.take();
    let mut moving = objects[index].clone();
    let mut order = Vec::new();
    if let Some(c) = prior {
        order.push(c.target);
    }
    order.extend(
        native_list_order(objects)
            .into_iter()
            .filter(|&i| i != index && Some(i) != prior.map(|c| c.target)),
    );
    for target in order {
        if is_object_linked(objects, index, target) {
            continue;
        }
        let b = &objects[target];
        if b.render_flags.contains(crate::machine::RenderFlag::Hidden)
            || b.outline.is_none()
            || moving.outline.is_none()
        {
            continue;
        }
        if moving.definition == Def::Cat && b.definition == Def::Mouse {
            continue;
        }
        let (w, h) = crate::spatial::native_object_extents(b);
        if w == 0 || h == 0 {
            continue;
        }
        let delta = sub(
            position(&moving),
            (
                i32::from(moving.previous_position.0),
                i32::from(moving.previous_position.1),
            ),
        );
        let (moving_width, moving_height) = crate::spatial::native_object_extents(&moving);
        let minx = i32::from(moving.x.min(moving.previous_position.0));
        let miny = i32::from(moving.y.min(moving.previous_position.1));
        let maxx = i32::from(moving.x) + i32::from(moving_width) + delta.0.abs();
        let maxy = i32::from(moving.y) + i32::from(moving_height) + delta.1.abs();
        let tangent_contact = matches!(
            (moving.definition, b.definition),
            (Def::Basketball, Def::CatPoweredDrive) | (Def::CatPoweredDrive, Def::Basketball)
        );
        // The DOS bounds are signed 16-bit endpoints. In particular a
        // resized wall with height 0xffe0 ends at y + (-32); widening that
        // word before addition turns the upper wall segment into a 65k-pixel
        // rectangle and steals contacts from the adjacent wall.
        let target_right = i32::from(b.x.wrapping_add(w as i16));
        let target_bottom = i32::from(b.y.wrapping_add(h as i16));
        let prior_target = prior.is_some_and(|contact| contact.target == target);
        let forward_separated = if tangent_contact {
            i32::from(b.x) > maxx
                || target_right < minx
                || i32::from(b.y) > maxy
                || target_bottom < miny
        } else {
            i32::from(b.x) >= maxx
                || target_right <= minx
                || i32::from(b.y) >= maxy
                || target_bottom <= miny
        };
        // 1000:0687's list traversal admits equal bounds for the backward
        // sweep; 1000:0556's previous-contact fast path remains strict.
        let backward_separated = if !prior_target && !tangent_contact {
            i32::from(b.x) > maxx
                || target_right < minx
                || i32::from(b.y) > maxy
                || target_bottom < miny
        } else {
            forward_separated
        };
        if !forward_separated {
            test_outline_contact_forward(&mut moving, b, target, prior);
        }
        if !backward_separated {
            test_outline_contact_backward(&mut moving, b, target, prior);
        }
    }
    if moving
        .contact
        .is_some_and(|c| are_coordinates_in_same_wrap_quadrant(c.angle, prior))
    {
        moving.update_flags |= UpdateFlag::Supported.mask();
    }
    objects[index] = moving;
}

/// test_object_outline_intersection, 1000:3f4d: extended integer edges.
pub fn test_object_outline_intersection(a: &Object, b: &Object) -> bool {
    let (Some(ap), Some(bp)) = (&a.outline, &b.outline) else {
        return false;
    };
    for edge in 0..ap.len() {
        let origin = add(position(a), vertex(ap[edge]));
        let end =
            update_line_segment_endpoint(sub(vertex(ap[(edge + 1) % ap.len()]), vertex(ap[edge])));
        for other in 0..bp.len() {
            let start = sub(add(position(b), vertex(bp[other])), origin);
            let finish = sub(add(position(b), vertex(bp[(other + 1) % bp.len()])), origin);
            let delta = sub(finish, start);
            let finish = add(finish, (delta.0.signum(), delta.1.signum()));
            if intersect_line_segments(((0, 0), end), (start, finish)).is_some_and(|hit| hit != end)
            {
                return true;
            }
        }
    }
    false
}
/// detect_object_collision, 1000:3e23, for moving actors' step probes and
/// part placement. Two fixed-list parts (update flag 0x4000) only compare
/// their saved +0x50/+0x52 boxes; outlines are tested otherwise.
pub fn detect_object_collision(objects: &[Object], index: usize) -> bool {
    let a = &objects[index];
    let (aw, ah) = crate::spatial::native_object_extents(a);
    let a_fixed = a.update_flags & UpdateFlag::FixedDefinition.mask() != 0;
    objects.iter().enumerate().any(|(i, b)| {
        if i == index
            || b.list == ObjectList::Inventory
            || b.render_flags.contains(crate::machine::RenderFlag::Hidden)
            || matches!(
                (a.definition, b.definition),
                (Def::Cat, Def::Mouse) | (Def::Mouse, Def::Cat)
            )
        {
            return false;
        }
        if a_fixed && b.update_flags & UpdateFlag::FixedDefinition.mask() != 0 {
            let ((aw, ah), (bw, bh)) = (a.alt_size, b.alt_size);
            return i32::from(b.x) < i32::from(a.x) + i32::from(aw)
                && i32::from(a.x) < i32::from(b.x) + i32::from(bw)
                && i32::from(b.y) < i32::from(a.y) + i32::from(ah)
                && i32::from(a.y) < i32::from(b.y) + i32::from(bh);
        }
        let (bw, bh) = crate::spatial::native_object_extents(b);
        i32::from(b.x) < i32::from(a.x) + i32::from(aw)
            && i32::from(a.x) < i32::from(b.x) + i32::from(bw)
            && i32::from(b.y) < i32::from(a.y) + i32::from(ah)
            && i32::from(a.y) < i32::from(b.y) + i32::from(bh)
            && test_object_outline_intersection(a, b)
    })
}
