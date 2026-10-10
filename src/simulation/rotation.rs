use super::*;
use crate::sound::SoundEffect;

const MOTOR_CONTACT_SPEED: i16 = 4096;
const GEAR_FULL_CONTACT_SPEED: i16 = 4096;
const GEAR_DIAGONAL_CONTACT_SPEED: i16 = 2048;
const OPPOSITE_EDGE_OFFSET: u16 = 4;
const OCTAGON_EDGE_MASK: u16 = 7;
const GEAR_FIRST_FRAME: u16 = 0;
const GEAR_LAST_FRAME: u16 = 3;
const GEAR_REVERSE_WRAP: i16 = -1;
const GEAR_CYCLE_END: i16 = 4;
const GEAR_SPACING_PIXELS: i16 = 32;
const GEAR_NEGATIVE_SPACING_PIXELS: i16 = -GEAR_SPACING_PIXELS;
const MOTOR_FRAMES_PER_GROUP: i16 = 7;
const MOTOR_WRAP_DELTA: i16 = MOTOR_FRAMES_PER_GROUP - 1;
const OUTLET_ROTATION_FRAME_MASK: u16 = 3;
const OUTLET_WRAP_DELTA: u16 = 3;
const ELECTRICAL_MOTOR_LAST_FRAME: u16 = 2;
const ELECTRICAL_MOTOR_REVERSE_WRAP: u16 = u16::MAX;

/// apply_motor_contact_thrust, 272c:2514: conveyor top/bottom edges drive
/// opposite directions, capped at the original 4096 horizontal velocity.
pub(super) fn apply_motor_contact_thrust(
    motion: &mut crate::motion::Motion,
    power: i16,
    edge: u16,
) -> CollisionResponse {
    let power = if edge == 0 {
        power
    } else if edge == 2 {
        power.wrapping_neg()
    } else {
        0
    };
    if power > 0 {
        motion.velocity_x = motion
            .velocity_x
            .wrapping_add(MOTOR_CONTACT_SPEED)
            .min(MOTOR_CONTACT_SPEED);
    } else if power < 0 {
        motion.velocity_x = motion.velocity_x.wrapping_sub(MOTOR_CONTACT_SPEED);
        if motion.velocity_x < MOTOR_CONTACT_SPEED {
            motion.velocity_x = -MOTOR_CONTACT_SPEED;
        }
    }
    CollisionResponse::Ordinary
}

/// apply_contact_velocity_by_edge, Ghidra 272c:1f78 (gear contact): only a
/// gear that turned this update acts. A balloon pops; anything else is pushed
/// along the touched edge of the octagon (reversed by four edges when the gear
/// turns backward) through the table at CS:2058.
pub(super) fn gear_contact(
    objects: &mut [Object],
    moving: usize,
    gear: usize,
    edge: u16,
) -> CollisionResponse {
    let turned = objects[gear].state as i16 - objects[gear].previous_state as i16;
    let direction = match turned {
        2.. => -1,
        ..-1 => 1,
        d => d,
    };
    if direction == 0 {
        return CollisionResponse::Ordinary;
    }
    let part = &mut objects[moving];
    if part.definition == Def::Balloon {
        part.power_state = 1;
        return CollisionResponse::Ordinary;
    }
    let edge = if direction > 0 {
        edge
    } else {
        (edge + OPPOSITE_EDGE_OFFSET) & OCTAGON_EDGE_MASK
    };
    // 272c:1f08 / 1f40: add, capped at delta.
    let add = |v: &mut i16, delta: i16| {
        *v = v.wrapping_add(delta);
        if delta < *v {
            *v = delta;
        }
    };
    // 272c:1f22 / 1f5a: subtract; the native test compares with +delta.
    let sub = |v: &mut i16, delta: i16| {
        *v = v.wrapping_sub(delta);
        if *v < delta {
            *v = -delta;
        }
    };
    let m = &mut part.motion;
    match edge {
        0 => add(&mut m.velocity_x, GEAR_FULL_CONTACT_SPEED),
        1 => {
            add(&mut m.velocity_x, GEAR_DIAGONAL_CONTACT_SPEED);
            add(&mut m.velocity_y, GEAR_DIAGONAL_CONTACT_SPEED);
        }
        2 => add(&mut m.velocity_y, GEAR_FULL_CONTACT_SPEED),
        3 => {
            sub(&mut m.velocity_x, GEAR_DIAGONAL_CONTACT_SPEED);
            add(&mut m.velocity_y, GEAR_DIAGONAL_CONTACT_SPEED);
        }
        4 => sub(&mut m.velocity_x, GEAR_FULL_CONTACT_SPEED),
        5 => {
            sub(&mut m.velocity_x, GEAR_DIAGONAL_CONTACT_SPEED);
            sub(&mut m.velocity_y, GEAR_DIAGONAL_CONTACT_SPEED);
        }
        6 => sub(&mut m.velocity_y, GEAR_FULL_CONTACT_SPEED),
        7 => {
            add(&mut m.velocity_x, GEAR_DIAGONAL_CONTACT_SPEED);
            sub(&mut m.velocity_y, GEAR_DIAGONAL_CONTACT_SPEED);
        }
        _ => {}
    }
    CollisionResponse::Ordinary
}

/// 272c:20fc, 215d and 2225: propagate a powered gear network, then consume
/// each gear's signed rotation impulse. Power sources are supplied by callbacks.
pub fn update_gear_rotation(objects: &mut [Object], gear: usize) -> Result<(), String> {
    if objects.get(gear).is_none_or(|o| o.definition != Def::Gear) {
        return Err("Gear callback requires a gear object".into());
    }
    if objects[gear].power_state == 0 {
        return Ok(());
    }
    objects[gear]
        .render_flags
        .insert(crate::machine::RenderFlag::ProcessedThisUpdate);
    let links: [Option<usize>; 4] = objects[gear].linked_objects[..4].try_into().unwrap();
    let mut conflict = false;
    for target in links.into_iter().flatten() {
        conflict |= propagate_rotation(objects, gear, target, false)?;
    }
    if conflict {
        objects[gear].power_state = 0;
    }
    finish_rotation(objects, gear, conflict)
}

pub(super) fn belt_partner(objects: &[Object], index: usize) -> Option<usize> {
    objects
        .iter()
        .filter_map(|o| o.belt)
        .find_map(|ends| match ends {
            [Some(a), Some(b)] if a == index => Some(b),
            [Some(a), Some(b)] if b == index => Some(a),
            _ => None,
        })
}

pub(super) fn propagate_rotation(
    objects: &mut [Object],
    source: usize,
    target: usize,
    same: bool,
) -> Result<bool, String> {
    if target >= objects.len() {
        return Err("Invalid gear network reference".into());
    }
    let power = objects[source].power_state;
    let target_power = objects[target].power_state;
    let mut conflict = false;
    if target_power == 0 {
        objects[target].power_state = if same { power } else { power.wrapping_neg() };
    } else if (same && target_power != power) || (!same && target_power == power) {
        conflict = true;
    }
    if objects[target].definition == Def::Gear
        && !objects[target]
            .render_flags
            .contains(crate::machine::RenderFlag::ProcessedThisUpdate)
    {
        objects[target]
            .render_flags
            .insert(crate::machine::RenderFlag::ProcessedThisUpdate);
        let links: [Option<usize>; 4] = objects[target].linked_objects[..4].try_into().unwrap();
        for neighbor in links.into_iter().flatten() {
            if objects.get(neighbor).is_none_or(|o| {
                o.render_flags
                    .contains(crate::machine::RenderFlag::PowerUpdateCallback)
            }) {
                continue;
            }
            conflict |= propagate_rotation(objects, target, neighbor, false)?;
        }
        if let Some(neighbor) = belt_partner(objects, target) {
            if !objects[neighbor]
                .render_flags
                .contains(crate::machine::RenderFlag::PowerUpdateCallback)
            {
                conflict |= propagate_rotation(objects, target, neighbor, true)?;
            }
        }
    }
    Ok(conflict)
}

pub(super) fn finish_rotation(
    objects: &mut [Object],
    gear: usize,
    conflict: bool,
) -> Result<(), String> {
    // The original accepts -1/+1 impulses and wraps only at -1 and 4.
    let state = (objects[gear].state as i16).wrapping_add(objects[gear].power_state);
    objects[gear].state = match state {
        GEAR_REVERSE_WRAP => GEAR_LAST_FRAME,
        GEAR_CYCLE_END => GEAR_FIRST_FRAME,
        _ => state as u16,
    };
    objects[gear].power_state = 0;
    let mut links = objects[gear].linked_objects[..4].to_vec();
    links.push(belt_partner(objects, gear));
    for neighbor in links.into_iter().flatten() {
        if neighbor >= objects.len() {
            return Err("Invalid gear network reference".into());
        }
        if objects[neighbor].power_state == 0
            || objects[neighbor]
                .render_flags
                .contains(crate::machine::RenderFlag::PowerUpdateCallback)
        {
            continue;
        }
        if conflict {
            objects[neighbor].power_state = 0;
        }
        if objects[neighbor].definition == Def::Gear {
            finish_rotation(objects, neighbor, conflict)?;
        }
    }
    Ok(())
}

/// Motor 272c:2592; preserves the current seven-frame animation group.
/// Returns whether the original callback would request motor sound.
pub fn update_motor_drive(objects: &mut [Object], motor: usize) -> Result<bool, String> {
    if objects
        .get(motor)
        .is_none_or(|o| o.definition != Def::Motor)
    {
        return Err("Motor callback requires a motor object".into());
    }
    if objects[motor].power_state != 0 {
        if let Some(partner) = belt_partner(objects, motor) {
            let gear = &objects[partner];
            if gear.definition == Def::Gear && gear.previous_state == gear.older_state {
                objects[motor].power_state = 0;
            }
        }
    }
    let object = &mut objects[motor];
    if object.power_state == 0 {
        return Ok(false);
    }
    let sound = object.state == object.previous_state;
    crate::sound::keep_effect_looping(SoundEffect::MotorRunning);
    if sound {
        crate::sound::play_effect(SoundEffect::MotorRunning);
    }
    let state = object.state as i16;
    object.state = if object.power_state < 0 {
        if state % MOTOR_FRAMES_PER_GROUP == 0 {
            state.wrapping_add(MOTOR_WRAP_DELTA)
        } else {
            state.wrapping_sub(1)
        }
    } else if state.wrapping_add(1) % MOTOR_FRAMES_PER_GROUP == 0 {
        state.wrapping_sub(MOTOR_WRAP_DELTA)
    } else {
        state.wrapping_add(1)
    } as u16;
    Ok(sound)
}

/// Electrical distributor 272c:15ce copies power to runtime slots 4 and 5.
pub fn update_power_distributor_links(objects: &mut [Object], index: usize) -> Result<(), String> {
    if objects
        .get(index)
        .is_none_or(|o| o.definition != Def::PowerDistributor)
    {
        return Err("Power distributor callback requires definition 21".into());
    }
    objects[index]
        .render_flags
        .insert(crate::machine::RenderFlag::ProcessedThisUpdate);
    let power = objects[index].power_state;
    let links = [
        objects[index].linked_objects[ELECTRICAL_OUTPUT_LINKS[0]],
        objects[index].linked_objects[ELECTRICAL_OUTPUT_LINKS[1]],
    ];
    for target in links.into_iter().flatten() {
        objects
            .get_mut(target)
            .ok_or("Invalid electrical link")?
            .power_state = power;
    }
    Ok(())
}

/// 272c:2068 searches the fixed list using saved coordinates, in list order.
/// Contact is exactly 32 pixels along one axis; diagonal gears do not mesh.
pub fn initialize_gear_neighbors(objects: &mut [Object]) {
    let fixed_gears: Vec<_> = objects
        .iter()
        .enumerate()
        .filter(|(_, o)| o.list == ObjectList::Fixed && o.definition == Def::Gear)
        .map(|(i, o)| (i, o.saved_position))
        .collect();
    for (index, object) in objects
        .iter_mut()
        .enumerate()
        .filter(|(_, o)| o.definition == Def::Gear)
    {
        object.linked_objects[..4].fill(None);
        let (x, y) = object.saved_position;
        for &(neighbor, (nx, ny)) in &fixed_gears {
            if neighbor == index {
                continue;
            }
            let (dx, dy) = (x.wrapping_sub(nx), y.wrapping_sub(ny));
            let slot = if y == ny {
                match dx {
                    GEAR_SPACING_PIXELS => Some(0),
                    GEAR_NEGATIVE_SPACING_PIXELS => Some(1),
                    _ => None,
                }
            } else if x == nx {
                match dy {
                    GEAR_SPACING_PIXELS => Some(2),
                    GEAR_NEGATIVE_SPACING_PIXELS => Some(3),
                    _ => None,
                }
            } else {
                None
            };
            if let Some(slot) = slot {
                object.linked_objects[slot] = Some(neighbor);
            }
        }
    }
}

/// Definition 26, 272c:1e5c; four-frame groups preserve signed rotation.
pub(super) fn update_electrical_outlet_rotation(
    objects: &mut [Object],
    index: usize,
) -> Result<(), String> {
    if objects[index].power_state != 0 {
        if let Some(partner) = belt_partner(objects, index) {
            let gear = objects
                .get(partner)
                .ok_or("Invalid generator belt partner")?;
            if gear.definition == Def::Gear && gear.previous_state == gear.older_state {
                objects[index].power_state = 0;
            }
        }
    }
    let part = &mut objects[index];
    if part.power_state != 0 {
        crate::sound::keep_effect_looping(SoundEffect::ElectricalDriveRunning);
        if part.state == part.previous_state {
            crate::sound::play_effect(SoundEffect::ElectricalDriveRunning);
        }
    }
    if part.power_state < 0 {
        part.state = if part.state & OUTLET_ROTATION_FRAME_MASK == 0 {
            part.state.wrapping_add(OUTLET_WRAP_DELTA)
        } else {
            part.state.wrapping_sub(1)
        };
    } else if part.power_state > 0 {
        part.state = if part.state & OUTLET_ROTATION_FRAME_MASK == OUTLET_ROTATION_FRAME_MASK {
            part.state.wrapping_sub(OUTLET_WRAP_DELTA)
        } else {
            part.state.wrapping_add(1)
        };
    }
    let power = part.power_state;
    let outputs = [
        part.linked_objects[ELECTRICAL_OUTPUT_LINKS[0]],
        part.linked_objects[ELECTRICAL_OUTPUT_LINKS[1]],
    ];
    for output in outputs.into_iter().flatten() {
        objects
            .get_mut(output)
            .ok_or("Invalid generator electrical output")?
            .power_state = power;
    }
    Ok(())
}

/// update_unknown50_links_and_rotation, 272c:13c9: electrical motor to belt.
pub(super) fn update_electrical_motor_links_and_rotation(objects: &mut [Object], index: usize) {
    if let Some(peer) = belt_partner(objects, index) {
        if !objects[peer]
            .render_flags
            .contains(crate::machine::RenderFlag::PowerUpdateCallback)
        {
            objects[peer].power_state = if objects[index].power_state == 0 {
                0
            } else if !objects[index]
                .render_flags
                .contains(crate::machine::RenderFlag::FlipHorizontal)
            {
                -1
            } else {
                1
            };
        }
    }
    if objects[index].power_state != 0 {
        crate::sound::keep_effect_looping(SoundEffect::ElectricalDriveRunning);
        if objects[index].state == objects[index].previous_state {
            crate::sound::play_effect(SoundEffect::ElectricalDriveRunning);
        }
        objects[index].state = objects[index].state.wrapping_sub(1);
        if objects[index].state == ELECTRICAL_MOTOR_REVERSE_WRAP {
            objects[index].state = ELECTRICAL_MOTOR_LAST_FRAME;
        }
    }
}
