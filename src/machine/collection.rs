use super::*;

pub(super) fn read_header(bytes: &[u8], level: bool) -> Result<CollectionMetadata, String> {
    let mut reader = Reader { bytes, offset: 0 };
    if reader.word()? != 0xaced || reader.word()? != 0x102 {
        return Err("Unsupported machine version".into());
    }
    let mut string = || -> Result<String, String> {
        let start = reader.offset;
        while reader.byte()? != 0 {}
        std::str::from_utf8(&bytes[start..reader.offset - 1])
            .map(str::to_owned)
            .map_err(|e| e.to_string())
    };
    let (title, description) = if level {
        (string()?, string()?)
    } else {
        (String::new(), String::new())
    };
    let bonuses = if level {
        Some((reader.word()?, reader.word()?))
    } else {
        None
    };
    let air = reader.word()? as i16;
    let gravity = reader.word()? as i16;
    let scroll_limits = if level {
        Some((reader.word()? as i16, reader.word()? as i16))
    } else {
        None
    };
    let music = reader.word()?;
    let counts = [reader.word()?, reader.word()?, reader.word()?];
    Ok(CollectionMetadata {
        title,
        description,
        bonuses,
        air,
        gravity,
        scroll_limits,
        music,
        counts,
        records_offset: reader.offset,
    })
}
pub fn collection_metadata(
    resources: &Resources,
    name: &str,
) -> Result<CollectionMetadata, String> {
    read_header(resources.get(name)?, name.ends_with(".LEV"))
}

/// Saved collection environment used by the original force initializer.
pub fn collection_forces(
    resources: &Resources,
    name: &str,
) -> Result<[crate::forces::PartForce; 58], String> {
    let header = collection_metadata(resources, name)?;
    Ok(crate::forces::initialize(header.air, header.gravity))
}

pub fn collection_objects(resources: &Resources, name: &str) -> Result<Vec<Object>, String> {
    Ok(parse_collection(resources.get(name)?, name.ends_with(".LEV"))?.1)
}

/// A saved machine from its bytes: LEV files carry the puzzle header, GKC and
/// TIM files (freeform saves) the short one.
pub fn parse_collection(
    bytes: &[u8],
    level: bool,
) -> Result<(CollectionMetadata, Vec<Object>), String> {
    let header = read_header(bytes, level)?;
    let mut reader = Reader {
        bytes,
        offset: header.records_offset,
    };
    let count: usize = header.counts.iter().map(|&n| n as usize).sum();
    let mut objects = Vec::with_capacity(count);
    for index in 0..count {
        let list = if index < header.counts[0] as usize {
            ObjectList::Fixed
        } else if index < (header.counts[0] as usize + header.counts[1] as usize) {
            ObjectList::Moving
        } else {
            ObjectList::Inventory
        };
        let definition = PartDefinition::try_from(reader.word()?)?;
        // Native saved reader 1dff:3e4f: +06, +94(render), +0a.
        let update_flags = reader.word()?;
        let render_flags = RenderFlags::from(reader.word()?);
        let flags = reader.word()?;
        let state = reader.word()?;
        let power_state = reader.word()? as i16;
        let width = reader.word()?;
        let height = reader.word()?;
        let alt_size = (reader.word()?, reader.word()?); // Native +0x50/+0x52.
        let x = reader.word()? as i16;
        let y = reader.word()? as i16;
        let activation_timer = reader.word()? as i16;
        let has_belt = reader.word()? != 0;
        let belt_anchor = (reader.byte()?, reader.byte()?);
        let belt_size = reader.word()? as i16;
        let belt = if has_belt {
            Some([reader.reference(count)?, reader.reference(count)?])
        } else {
            None
        };
        let mut ropes = [None, None];
        let mut rope_anchors = [(0, 0); 2];
        for (slot, rope) in ropes.iter_mut().enumerate() {
            let linked = reader.word()? != 0;
            rope_anchors[slot] = (reader.byte()?, reader.byte()?);
            if linked {
                *rope = Some(RopeConnection {
                    endpoints: [reader.reference(count)?, reader.reference(count)?],
                    attachment_slots: [reader.byte()?, reader.byte()?],
                    rest_length: 0.0,
                    slack_count: 0,
                });
            }
        }
        let saved_links = [
            reader.reference(count)?,
            reader.reference(count)?,
            reader.reference(count)?,
            reader.reference(count)?,
        ];
        // Serialized slots 0/1 and 4/5; runtime slots 2/3 initially copy 0/1.
        let linked_objects = [
            saved_links[0],
            saved_links[1],
            saved_links[0],
            saved_links[1],
            saved_links[2],
            saved_links[3],
        ];
        let pulley_connection = if definition == PartDefinition::Pulley {
            reader.reference(count)?
        } else {
            None
        };
        objects.push(Object {
            list,
            update_flags,
            contact: None,
            definition,
            motion: crate::motion::Motion::at_pixels(x, y),
            effective_mass: crate::forces::mass(definition),
            collision_impulse: 0,
            saved_position: (x, y),
            previous_position: (x, y),
            older_position: (x, y),
            saved_state: state,
            outline: None,
            outline_angles: Vec::new(),
            state,
            previous_state: state,
            older_state: state,
            power_state,
            callback_timer: 0,
            pending_light_power: 0,
            activation_timer,
            belt_anchor,
            belt_size,
            rope_anchors,
            belt,
            ropes,
            linked_objects,
            next_part: None,
            pulley_connection,
            flags,
            render_flags,
            x,
            y,
            width,
            height,
            extent_override: None,
            alt_size,
        });
    }
    if reader.offset != reader.bytes.len() {
        return Err("Unconsumed title machine data".into());
    }
    finalize_objects(&mut objects)?;
    Ok((header, objects))
}

/// save_level_without_shared_objects (1dff:493d) through
/// save_object_collection_to_file: the short GKC/TIM header (version 0x102,
/// environment, music, and fixed/moving lists.
pub fn write_collection(objects: &[Object], air: i16, gravity: i16, music: u16) -> Vec<u8> {
    write_lists(objects, air, gravity, music, false)
}

/// `write_collection` plus the parts bin as the third list, which the TIM
/// format counts but the original never fills; puzzle saves need it.
pub fn write_collection_with_bin(
    objects: &[Object],
    air: i16,
    gravity: i16,
    music: u16,
) -> Vec<u8> {
    write_lists(objects, air, gravity, music, true)
}

pub(super) fn write_lists(
    objects: &[Object],
    air: i16,
    gravity: i16,
    music: u16,
    bin: bool,
) -> Vec<u8> {
    let lists: &[ObjectList] = if bin {
        &[ObjectList::Fixed, ObjectList::Moving, ObjectList::Inventory]
    } else {
        &[ObjectList::Fixed, ObjectList::Moving]
    };
    let order: Vec<usize> = lists
        .iter()
        .flat_map(|&list| (0..objects.len()).filter(move |&i| objects[i].list == list))
        .collect();
    let mut position = vec![None; objects.len()];
    for (saved, &index) in order.iter().enumerate() {
        position[index] = Some(saved as u16);
    }
    let mut out = Vec::new();
    let word = |out: &mut Vec<u8>, v: u16| out.extend_from_slice(&v.to_le_bytes());
    let reference = |out: &mut Vec<u8>, r: Option<usize>| {
        let saved = r
            .and_then(|i| position.get(i).copied().flatten())
            .unwrap_or(0xffff);
        out.extend_from_slice(&saved.to_le_bytes());
    };
    for v in [0xaced, 0x102, air as u16, gravity as u16, music] {
        word(&mut out, v);
    }
    let fixed = order
        .iter()
        .filter(|&&i| objects[i].list == ObjectList::Fixed)
        .count();
    let inventory = order
        .iter()
        .filter(|&&i| objects[i].list == ObjectList::Inventory)
        .count();
    for v in [fixed, order.len() - fixed - inventory, inventory] {
        word(&mut out, v as u16);
    }
    for &i in &order {
        let o = &objects[i];
        // write_object_record_state (1dff:4440) writes the saved state.
        for v in [
            o.definition.id(),
            o.update_flags,
            o.render_flags.bits(),
            o.flags,
            o.saved_state,
            o.power_state as u16,
            o.width,
            o.height,
            o.alt_size.0,
            o.alt_size.1,
        ] {
            word(&mut out, v);
        }
        let (x, y) = o.saved_position;
        for v in [
            x as u16,
            y as u16,
            o.activation_timer as u16,
            u16::from(o.belt.is_some()),
        ] {
            word(&mut out, v);
        }
        out.extend_from_slice(&[o.belt_anchor.0, o.belt_anchor.1]);
        word(&mut out, o.belt_size as u16);
        if let Some(ends) = o.belt {
            reference(&mut out, ends[0]);
            reference(&mut out, ends[1]);
        }
        for slot in 0..2 {
            let rope = &o.ropes[slot];
            word(&mut out, u16::from(rope.is_some()));
            out.extend_from_slice(&[o.rope_anchors[slot].0, o.rope_anchors[slot].1]);
            if let Some(rope) = rope {
                reference(&mut out, rope.endpoints[0]);
                reference(&mut out, rope.endpoints[1]);
                out.extend_from_slice(&rope.attachment_slots);
            }
        }
        for link in [
            o.linked_objects[0],
            o.linked_objects[1],
            o.linked_objects[4],
            o.linked_objects[5],
        ] {
            reference(&mut out, link);
        }
        if o.definition == PartDefinition::Pulley {
            reference(&mut out, o.pulley_connection);
        }
    }
    out
}
