use super::*;

/// Ghidra 1000:6de9, applied to the route points rebuilt by 1000:4f7f.
/// The low contact bits match the native 1/2/4 outcomes; 8/0x10 carry the
/// horizontal-side comparison used by the other definition callbacks.
#[allow(non_snake_case)]
pub(super) fn ComputeRopeContactFlags(
    objects: &[Object],
    rope: &crate::machine::RopeConnection,
    endpoint_side: usize,
    direction_mode: bool,
) -> u16 {
    let Ok(segments) = crate::machine::rope_segments(objects, rope) else {
        return 0;
    };
    if segments.is_empty() || endpoint_side > 1 {
        return 0;
    }
    // Each pulley has distinct incoming/outgoing anchors. Dropping a segment's
    // start loses the outgoing anchor and makes a one-pulley route look invalid.
    let points_iter = segments.iter().flat_map(|segment| [segment.0, segment.1]);
    let mut points: Vec<_> = points_iter.collect();
    if endpoint_side == 1 {
        points.reverse();
    }
    if points.len() < 2 {
        return 0;
    }

    let current_y = points[0].1;
    let near_route_y = points[1].1;
    let far_y = points.last().unwrap().1;
    let far_route_y = points[points.len() - 2].1;
    let side = if points[points.len() - 2].0 < points.last().unwrap().0 {
        RopeContactFlag::RouteLeftOfTarget.mask()
    } else {
        RopeContactFlag::RouteAtOrRightOfTarget.mask()
    };
    let contact_side = if !direction_mode {
        if near_route_y < current_y {
            RopeContactFlag::LocalDirectionMatch.mask()
        } else if far_y <= far_route_y {
            RopeContactFlag::RemoteDirectionMatch.mask()
        } else {
            RopeContactFlag::RemoteDirectionMismatch.mask()
        }
    } else if current_y < near_route_y {
        RopeContactFlag::LocalDirectionMatch.mask()
    } else if far_y < far_route_y {
        RopeContactFlag::RemoteDirectionMatch.mask()
    } else {
        RopeContactFlag::RemoteDirectionMismatch.mask()
    };
    if contact_side == RopeContactFlag::LocalDirectionMatch.mask() {
        RopeContactFlag::LocalDirectionMatch.mask()
    } else {
        (side | contact_side) as u16
    }
}

/// Routed light ropes follow the pulley-neighbor coordinate reads in
/// 1000:6de9: outgoing anchor on the source-side pulley, incoming anchor on
/// the target-side pulley. Keep this narrow until other rope-contact users
/// are traced through their callback-specific paths.
#[allow(non_snake_case)]
pub(super) fn ComputeRoutedLightContactFlags(
    objects: &[Object],
    rope: &crate::machine::RopeConnection,
    endpoint_side: usize,
    direction_mode: bool,
) -> u16 {
    let Ok(segments) = crate::machine::rope_segments(objects, rope) else {
        return 0;
    };
    if segments.len() < 2 || endpoint_side > 1 {
        return ComputeRopeContactFlags(objects, rope, endpoint_side, direction_mode);
    }
    let mut points: Vec<_> = segments
        .iter()
        .flat_map(|segment| [segment.0, segment.1])
        .collect();
    if endpoint_side == 1 {
        points.reverse();
    }
    let source = points[0];
    let target = *points.last().unwrap();
    let source_neighbor = points[2];
    let target_neighbor = points[points.len() - 2];
    let side = if target_neighbor.0 < target.0 {
        RopeContactFlag::RouteLeftOfTarget.mask()
    } else {
        RopeContactFlag::RouteAtOrRightOfTarget.mask()
    };
    let contact = if !direction_mode {
        if source_neighbor.1 < source.1 {
            RopeContactFlag::LocalDirectionMatch.mask()
        } else if target.1 <= target_neighbor.1 {
            RopeContactFlag::RemoteDirectionMatch.mask()
        } else {
            RopeContactFlag::RemoteDirectionMismatch.mask()
        }
    } else if source.1 < source_neighbor.1 {
        RopeContactFlag::LocalDirectionMatch.mask()
    } else if target.1 < target_neighbor.1 {
        RopeContactFlag::RemoteDirectionMatch.mask()
    } else {
        RopeContactFlag::RemoteDirectionMismatch.mask()
    };
    if contact == RopeContactFlag::LocalDirectionMatch.mask() {
        RopeContactFlag::LocalDirectionMatch.mask()
    } else {
        (side | contact) as u16
    }
}

/// Ghidra 272c:461a. Probe connected rope callbacks with 0x8000 first, then
/// dispatch the real event only when none of the contacted parts blocks it.
#[allow(non_snake_case)]
pub(super) fn DispatchConnectedRopeContact(
    objects: &mut [Object],
    source_index: usize,
    callback_flags: u16,
) -> bool {
    dispatch_rope_contact(objects, source_index, None, callback_flags, &mut Vec::new())
}

pub(super) fn dispatch_rope_contact(
    objects: &mut [Object],
    source_index: usize,
    caller: Option<usize>,
    callback_flags: u16,
    path: &mut Vec<usize>,
) -> bool {
    if path.contains(&source_index) {
        return false;
    }
    path.push(source_index);
    let blocked = dispatch_rope_contact_inner(objects, source_index, caller, callback_flags, path);
    path.pop();
    blocked
}

pub(super) fn dispatch_rope_contact_inner(
    objects: &mut [Object],
    source_index: usize,
    caller: Option<usize>,
    callback_flags: u16,
    path: &mut Vec<usize>,
) -> bool {
    if objects[source_index]
        .render_flags
        .contains(crate::machine::RenderFlag::CallbackBlocked)
    {
        return true;
    }
    let source_definition = objects[source_index].definition;
    let source_power = objects[source_index].power_state;
    let mut ropes: Vec<_> = objects
        .iter()
        .flat_map(|object| object.ropes.iter().flatten())
        .filter(|rope| rope.endpoints.contains(&Some(source_index)))
        .cloned()
        .collect();

    ropes.sort_by_key(|r| {
        r.endpoints
            .iter()
            .position(|&e| e == Some(source_index))
            .map_or(0, |side| r.attachment_slots[side])
    });
    let mut seen = Vec::new();
    for rope in ropes {
        if seen.contains(&(rope.endpoints, rope.attachment_slots)) {
            continue;
        }
        seen.push((rope.endpoints, rope.attachment_slots));
        let Some(endpoint_side) = rope
            .endpoints
            .iter()
            .position(|endpoint| *endpoint == Some(source_index))
        else {
            continue;
        };
        let Some(target_index) = rope.endpoints[1 - endpoint_side] else {
            continue;
        };
        if Some(target_index) == caller {
            continue;
        }
        let source_slot = rope.attachment_slots[endpoint_side] as usize;
        let direction_mode = if source_power < 1 {
            source_slot == 0
        } else {
            source_slot != 0
        };
        let contact_flags =
            if source_definition == Def::Seesaw && objects[target_index].definition == Def::Light {
                ComputeRoutedLightContactFlags(objects, &rope, endpoint_side, direction_mode)
            } else {
                ComputeRopeContactFlags(objects, &rope, endpoint_side, direction_mode)
            } | callback_flags;
        if std::env::var_os("TIM_TRACE_MOTION").is_some() {
            eprintln!(
                "DispatchConnectedRopeContact: source={source_index}/def{} target={target_index}/def{} ends={:?} slots={:?} source_side={endpoint_side} mode={direction_mode} flags={contact_flags:#06x} probe={}",
                source_definition.id(),
                objects[target_index].definition.id(),
                rope.endpoints,
                rope.attachment_slots,
                callback_flags & RopeContactFlag::ProbeOnly.mask() != 0
            );
        }
        let blocked = if objects[target_index].definition == Def::Seesaw {
            seesaw_rope_contact(
                objects,
                target_index,
                source_index,
                rope.attachment_slots[1 - endpoint_side],
                contact_flags,
                path,
            )
        } else {
            rope_contact_callback(objects, target_index, contact_flags)
        };
        if blocked {
            return true;
        }
    }
    false
}

/// Definition rope-contact slot (definition table +0xedc). A true result
/// blocks propagation, as the native callbacks' nonzero return does.
pub(super) fn rope_contact_callback(
    objects: &mut [Object],
    target_index: usize,
    contact_flags: u16,
) -> bool {
    match objects[target_index].definition {
        Def::Light => process_light_rope_contact(&mut objects[target_index], contact_flags),
        Def::Detonator => detonator_rope_contact(objects, target_index, contact_flags),
        Def::Cannon => {
            activate_cannon_from_rope(&mut objects[target_index], contact_flags);
            false
        }
        Def::Gun => gun_rope_contact(objects, target_index, contact_flags),
        Def::Monkey => monkey_rope_contact(&mut objects[target_index], contact_flags),
        _ => false,
    }
}

/// 1000:72c7 samples the moving endpoint's direction before the rope
/// correction: Some(true) rising, Some(false) falling, None when level.
pub(super) fn vertical_direction(object: &Object) -> Option<bool> {
    match object.y.cmp(&object.previous_position.1) {
        std::cmp::Ordering::Less => Some(true),
        std::cmp::Ordering::Greater => Some(false),
        std::cmp::Ordering::Equal => None,
    }
}

/// Tail of 1000:72c7: once a taut rope has constrained a moving endpoint, the
/// peer is told it is being pulled. A seesaw takes a queued signed impulse
/// toward the pulled tip; any other part runs its rope-contact callback.
pub(super) fn rope_pull_response(
    objects: &mut [Object],
    rope: &crate::machine::RopeConnection,
    moving_side: usize,
    rising: Option<bool>,
    priority: &mut ImpulsePriority,
) {
    let (Some(moving), Some(peer)) = (rope.endpoints[moving_side], rope.endpoints[1 - moving_side])
    else {
        return;
    };
    let Some(rising) = rising else {
        return;
    };
    if objects[moving].definition == Def::RopeEnd || objects[peer].definition == Def::RopeEnd {
        return;
    }
    // The native callback reads endpoint coordinates cached in the type-10
    // geometry record. That record is rebuilt after the simulation response
    // pass, so a moving balloon's newly changed anchor/position is not visible
    // here until the following update.
    let flags = if objects[peer].definition == Def::Gun && objects[peer].power_state == 0 {
        let mut cached = objects.to_vec();
        for object in &mut cached {
            object.x = object.previous_position.0;
            object.y = object.previous_position.1;
        }
        ComputeRopeContactFlags(&cached, rope, moving_side, !rising)
    } else {
        ComputeRopeContactFlags(objects, rope, moving_side, !rising)
    };
    if objects[peer].definition != Def::Seesaw {
        rope_contact_callback(objects, peer, flags);
        return;
    }
    let peer_slot = rope.attachment_slots[1 - moving_side];
    let state = objects[peer].state as i16;
    let direction = match (flags & 4 != 0, peer_slot == 0) {
        (true, true) if state > 0 => -1,
        (true, false) if state < 2 => 1,
        (false, true) if state < 2 => 1,
        (false, false) if state > 0 => -1,
        _ => 0,
    };
    let impulse = objects[moving].collision_impulse;
    if priority.accept(peer, impulse) {
        objects[peer].power_state = direction;
        objects[peer].collision_impulse = impulse;
    }
}
