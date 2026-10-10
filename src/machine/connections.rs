use super::*;

/// Derived state the original rebuilds after loading or editing a machine:
/// rope lengths along their pulley routes, gear neighbours, and outlines.
/// The outlets' geometry callbacks show a plug in each used socket through
/// state bits: build_power_distributor_outline (272c:1556) bits 0-1 beside
/// the switch bit 2, build_unknown26_outline (272c:1dfb) bits 2-3 above the
/// generator's rotation, rebuild_solar_panel_state (272c:3de5) bits 0-1.
/// Like the callbacks this leaves saved_state alone; plugging in during
/// editing copies it (rebind_held_object_attachment).
pub fn refresh_outlet_state(outlet: &mut Object) {
    let first = u16::from(outlet.linked_objects[4].is_some());
    let second = u16::from(outlet.linked_objects[5].is_some());
    outlet.state = match outlet.definition {
        crate::definition::PartDefinition::PowerDistributor => {
            (outlet.state & 4) | first | second << 1
        }
        crate::definition::PartDefinition::ElectricalOutlet => {
            (outlet.state & 3) | first << 2 | second << 3
        }
        crate::definition::PartDefinition::SolarPanel => first | second << 1,
        _ => return,
    };
}

/// Rebuild pulley orientation and tangent anchors (1000:4c0d/4d4c).
/// Link 0 faces the next hop; link 1 faces the previous hop. Tangent anchor
/// 0 is incoming and anchor 1 is outgoing (24de:1dcf). A missing next
/// hop uses the loose end's world position while editing.
pub fn refresh_pulleys(objects: &mut [Object], loose_end: Option<(i16, i16)>) {
    let updates: Vec<_> = objects
        .iter()
        .enumerate()
        .filter_map(|(index, o)| {
            if o.definition != PartDefinition::Pulley {
                return None;
            }
            let previous = o.linked_objects[1]?;
            let angle = |neighbor: Option<usize>| -> Option<u16> {
                let (x, y) = if let Some(i) = neighbor {
                    let other = objects.get(i)?;
                    if other.definition == PartDefinition::Pulley {
                        (other.x, other.y)
                    } else {
                        let slot = other.linked_objects[..2]
                            .iter()
                            .position(|p| *p == Some(index))
                            .unwrap_or(0);
                        let (ax, ay) = other.rope_anchors[slot];
                        (
                            other.x.wrapping_add(i16::from(ax)),
                            other.y.wrapping_add(i16::from(ay)),
                        )
                    }
                } else {
                    loose_end?
                };
                Some(
                    crate::outline::vector_angle(o.x.wrapping_sub(x), o.y.wrapping_sub(y))
                        .wrapping_add(0x2000),
                )
            };
            let back = angle(Some(previous))?;
            let forward = angle(o.linked_objects[0])?;
            let delta = forward.wrapping_sub(back);
            let (start, span) = if delta > 0x7fff {
                (forward, delta.wrapping_neg())
            } else {
                (back, delta)
            };
            let state = start.wrapping_add(span >> 1) >> 14;
            let normal = delta < 0x8000;
            let anchors = match state {
                0 | 2 => {
                    let y = if state == 0 { 10 } else { 6 };
                    if (state == 0 && normal) || (state == 2 && !normal) {
                        [(0, y), (15, y)]
                    } else {
                        [(15, y), (0, y)]
                    }
                }
                _ => {
                    let x = if state == 1 { 6 } else { 10 };
                    if (state == 1 && normal) || (state == 3 && !normal) {
                        [(x, 0), (x, 15)]
                    } else {
                        [(x, 15), (x, 0)]
                    }
                }
            };
            Some((index, state, anchors))
        })
        .collect();
    for (index, state, anchors) in updates {
        objects[index].state = state;
        objects[index].saved_state = state;
        objects[index].rope_anchors = anchors;
    }
}

/// rebuild_rope_endpoint_coordinates (1000:4f7f), outside run mode.
/// Rebase rope allowances on the current editor geometry, including pulleys.
pub fn refresh_rope_lengths(objects: &mut [Object]) -> Result<(), String> {
    // Preserve the saved rope path through any linked pulleys. A straight
    // endpoint chord loses the pulley wrap and makes the live constraint slack.
    let mut rope_lengths = Vec::new();
    for (owner, object) in objects.iter().enumerate() {
        for (slot, rope) in object.ropes.iter().enumerate() {
            if let Some(rope) = rope {
                if let [Some(a), Some(b)] = rope.endpoints {
                    let _ = (a, b);
                    let length = rope_segments(objects, rope)?
                        .iter()
                        .map(|(p, q)| {
                            let dx = (q.0 - p.0) as f32;
                            let dy = (q.1 - p.1) as f32;
                            dx.hypot(dy)
                        })
                        .sum();
                    let spans = rope_spans(objects, rope)?;
                    let ends = [spans.first(), spans.last()]
                        .map(|s| s.map_or(0, |s| linked_point_distance(s.start, s.end)));
                    rope_lengths.push((owner, slot, length, ends));
                }
            }
        }
    }
    for (owner, slot, length, ends) in rope_lengths {
        if objects[owner].definition == PartDefinition::Rope {
            // rebuild_rope_endpoint_coordinates (1000:4f7f), outside run mode.
            objects[owner].activation_timer = ends[0] as i16;
            objects[owner].callback_timer = ends[1] as i16;
        }
        if let Some(rope) = objects[owner].ropes[slot].as_mut() {
            rope.rest_length = length;
        }
    }
    Ok(())
}

pub fn finalize_objects(objects: &mut [Object]) -> Result<(), String> {
    refresh_pulleys(objects, None);
    refresh_rope_lengths(objects)?;
    crate::simulation::initialize_gear_neighbors(objects);
    for object in objects.iter_mut() {
        object.refresh_belt_anchor();
        apply_native_loaded_dimensions(object);
        // The initializers run on load, so saved plugs show in their sockets.
        if object.flags & PropertyFlag::ElectricalOutlet.mask() != 0 {
            refresh_outlet_state(object);
        }
        object.outline = crate::outline::vertices(object);
        object.outline_angles = object
            .outline
            .as_deref()
            .map(crate::outline::edge_angles)
            .unwrap_or_default();
    }
    Ok(())
}
/// definition/state frame, as `load_object_anchor_offsets` (1000:5c77) does.
/// The serialized size remains the alternate size only when update flag 0x40
/// selects the user-resized branch. These entries are confirmed against DOS
/// snapshots for the saved Motor and Light states covered by this port.
pub(super) fn apply_native_loaded_dimensions(object: &mut Object) {
    if object.update_flags & UpdateFlag::UseAlternateSize.mask() != 0 {
        return;
    }
    let dimensions = match object.definition {
        PartDefinition::Motor => match object.state {
            0..=6 => Some((32, 16)),
            7..=13 => Some((48, 16)),
            14..=20 => Some((64, 16)),
            21..=27 => Some((80, 16)),
            28..=34 => Some((96, 16)),
            _ => None,
        },
        PartDefinition::Light if matches!(object.state, 2 | 3) => Some((32, 38)),
        _ => None,
    };
    if let Some((width, height)) = dimensions {
        object.width = width;
        object.height = height;
    }
}
