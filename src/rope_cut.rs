//! split_ropes_at_object (272c:3970): split intersected routed ropes into
//! two linked pieces with moving RopeEnd parts. Drawing history and sound
//! are handled separately from this simulation graph.
use crate::{
    definition::PartDefinition as Def,
    machine::{Object, ObjectList, RenderFlag, RopeConnection, rope_spans},
    motion::Motion,
};

fn intersection(a: (i32, i32), b: (i32, i32), c: (i32, i32), d: (i32, i32)) -> Option<(i16, i16)> {
    let cross = |x: (i64, i64), y: (i64, i64)| x.0 * y.1 - x.1 * y.0;
    let delta = |x: (i32, i32), y: (i32, i32)| (i64::from(y.0 - x.0), i64::from(y.1 - x.1));
    let ab = delta(a, b);
    let cd = delta(c, d);
    let ac = delta(a, c);
    let denominator = cross(ab, cd);
    if denominator == 0 {
        return None;
    }
    let numerator = cross(ac, cd);
    let along_rope = cross(ac, ab);
    let within = |v: i64| {
        if denominator > 0 {
            (0..=denominator).contains(&v)
        } else {
            (denominator..=0).contains(&v)
        }
    };
    if !within(numerator) || !within(along_rope) {
        return None;
    }
    Some((
        (i64::from(a.0) + ab.0 * numerator / denominator) as i16,
        (i64::from(a.1) + ab.1 * numerator / denominator) as i16,
    ))
}

fn loose_end(
    template: &Object,
    point: (i16, i16),
    neighbor: usize,
    rope: RopeConnection,
) -> Object {
    let mut end = template.clone();
    end.definition = Def::RopeEnd;
    end.list = ObjectList::Moving;
    end.update_flags = 0x810;
    end.render_flags = RenderFlag::RopeAttachable.mask();
    end.flags = 8;
    end.x = point.0;
    end.y = point.1;
    end.saved_position = point;
    end.previous_position = point;
    end.older_position = point;
    end.saved_state = 0;
    end.motion = Motion::at_pixels(point.0, point.1);
    // The DOS RopeEnd initializer seeds the horizontal component at the
    // environment's negative terminal speed; the first taut-rope update then
    // corrects it from the endpoint displacement.
    end.effective_mass = crate::forces::mass(Def::RopeEnd);
    end.collision_impulse = 0;
    end.width = 0;
    end.height = 0;
    end.state = 0;
    end.previous_state = 0;
    end.older_state = 0;
    end.power_state = 0;
    end.callback_timer = 0;
    end.pending_light_power = 0;
    end.activation_timer = 0;
    end.contact = None;
    end.outline = None;
    end.outline_angles.clear();
    end.rope_anchors = [(0, 0); 2];
    end.belt = None;
    end.belt_anchor = (0, 0);
    end.belt_size = 0;
    end.pulley_connection = None;
    end.ropes = [Some(rope), None];
    end.linked_objects = [Some(neighbor), None, None, None, None, None];
    end
}

pub(crate) fn split_ropes_at_object(
    objects: &mut Vec<Object>,
    scissors: usize,
) -> Result<(), String> {
    let object = &objects[scissors];
    // Original blade segments at 3d3c:34ba / 34c2.
    let (left, right) = if !object
        .render_flags
        .contains(crate::machine::RenderFlag::FlipHorizontal)
    {
        (22, 39)
    } else {
        (0, 16)
    };
    let blade_start = (i32::from(object.x) + left, i32::from(object.y) + 15);
    let blade_end = (i32::from(object.x) + right, i32::from(object.y) + 15);
    let owners: Vec<_> = objects
        .iter()
        .enumerate()
        .filter(|(_, o)| {
            o.list == ObjectList::Fixed
                && o.definition == Def::Rope
                && !o.render_flags.contains(crate::machine::RenderFlag::Hidden)
        })
        .map(|(i, _)| i)
        .collect();
    for owner in owners {
        let Some(old) = objects[owner].ropes.iter().flatten().next().cloned() else {
            continue;
        };
        let spans = rope_spans(objects, &old)?;
        let Some((cut, point)) = spans.iter().enumerate().find_map(|(i, s)| {
            intersection(blade_start, blade_end, s.start, s.end).map(|p| (i, p))
        }) else {
            continue;
        };
        let span = spans[cut];
        let first_end = objects.len();
        let second_end = first_end + 1;
        let mut first = old.clone();
        first.endpoints[1] = Some(first_end);
        first.attachment_slots[1] = 0;
        let mut second = old.clone();
        second.endpoints[0] = Some(second_end);
        second.attachment_slots[0] = 0;
        let length = |a: (i32, i32), b: (i32, i32)| ((b.0 - a.0) as f32).hypot((b.1 - a.1) as f32);
        let cut_point = (i32::from(point.0), i32::from(point.1));
        first.rest_length = spans[..cut]
            .iter()
            .map(|s| length(s.start, s.end))
            .sum::<f32>()
            + length(span.start, cut_point);
        second.rest_length = length(cut_point, span.end)
            + spans[cut + 1..]
                .iter()
                .map(|s| length(s.start, s.end))
                .sum::<f32>();
        for (index, object) in objects.iter_mut().enumerate() {
            let on_first = index == owner || spans[..=cut].iter().any(|s| s.from == index);
            for rope in object.ropes.iter_mut().flatten() {
                if rope.endpoints == old.endpoints && rope.attachment_slots == old.attachment_slots
                {
                    *rope = if on_first {
                        first.clone()
                    } else {
                        second.clone()
                    };
                }
            }
        }
        let from_link = if objects[span.from].definition == Def::Pulley {
            0
        } else {
            span.from_slot
        };
        let to_link = if objects[span.to].definition == Def::Pulley {
            1
        } else {
            span.to_slot
        };
        objects[span.from].linked_objects[from_link] = Some(first_end);
        objects[span.to].linked_objects[to_link] = Some(second_end);
        if let Some(endpoint) = first.endpoints[0] {
            objects[endpoint].ropes[first.attachment_slots[0] as usize] = Some(first.clone());
        }
        if let Some(endpoint) = second.endpoints[1] {
            objects[endpoint].ropes[second.attachment_slots[1] as usize] = Some(second.clone());
        }
        for path in &spans[cut + 1..] {
            if objects[path.from].definition == Def::Pulley {
                objects[path.from].pulley_connection = Some(first_end + 2);
                objects[path.from].ropes[0] = Some(second.clone());
            }
        }
        if objects[span.from].definition == Def::Pulley {
            objects[span.from].ropes[0] = Some(first.clone());
        }
        if objects[span.to].definition == Def::Pulley {
            objects[span.to].pulley_connection = Some(first_end + 2);
            objects[span.to].ropes[0] = Some(second.clone());
        }
        let mut new_owner = objects[owner].clone();
        new_owner.ropes = [Some(second.clone()), None];
        // create_object_instance_from_definition(Rope) zeroes effective_mass
        // and starts with update_flags 0x4800; split_ropes_at_object
        // (272c:3970) then adds 0x10 before insertion.
        new_owner.update_flags = 0x4810;
        new_owner.effective_mass = 0;
        // create_object_instance_from_definition initializes the new fixed
        // Rope record at the native zero origin; it does not copy the split
        // owner's sentinel (-1,-1) coordinates.
        new_owner.x = 0;
        new_owner.y = 0;
        new_owner.previous_position = (0, 0);
        new_owner.older_position = (0, 0);
        new_owner.motion.position_x = 0;
        new_owner.motion.position_y = 0;
        // The untouched end keeps its stored allowance; the newly cut end
        // starts at the cut point. Direct pieces use their sole span on both sides.
        let first_cut_length = crate::machine::linked_point_distance(span.start, cut_point) as i16;
        let second_cut_length = crate::machine::linked_point_distance(cut_point, span.end) as i16;
        objects[owner].callback_timer = first_cut_length;
        if cut == 0 {
            objects[owner].activation_timer = first_cut_length;
        }
        new_owner.activation_timer = second_cut_length;
        if cut + 1 == spans.len() {
            new_owner.callback_timer = second_cut_length;
        }
        new_owner.linked_objects = [None; 6];
        new_owner.contact = None;
        objects.push(loose_end(&objects[owner], point, span.from, first));
        objects.push(loose_end(&objects[owner], point, span.to, second));
        objects.push(new_owner);
    }
    Ok(())
}
