use super::*;

#[derive(Clone, Copy, Debug)]
pub enum CallbackPass {
    FixedPowered,
    FixedGears,
    FixedOrdinary,
}

pub fn begin_update(objects: &mut [Object]) {
    // The DOS loop clears these transient render-state bits on list 0x2000.
    for object in objects.iter_mut().filter(|o| o.list == ObjectList::Fixed) {
        object
            .render_flags
            .retain(RenderFlags::UPDATE_PRESERVE_MASK);
    }
}

pub(super) fn selected_for_callback(object: &Object, pass: CallbackPass) -> bool {
    if object.list != ObjectList::Fixed {
        return false;
    }
    let flags = object.render_flags;
    let update_already_handled = crate::machine::RenderFlag::ProcessedThisUpdate.mask()
        | crate::machine::RenderFlag::Hidden.mask();
    match pass {
        CallbackPass::FixedPowered => {
            flags.contains(crate::machine::RenderFlag::PowerUpdateCallback)
                && !flags.intersects(update_already_handled)
        }
        CallbackPass::FixedGears => {
            object.definition == Def::Gear && !flags.intersects(update_already_handled)
        }
        CallbackPass::FixedOrdinary => !flags.intersects(
            update_already_handled | crate::machine::RenderFlag::PowerUpdateCallback.mask(),
        ),
    }
}

pub fn callback_candidates(objects: &[Object], pass: CallbackPass) -> Vec<usize> {
    objects
        .iter()
        .enumerate()
        .filter_map(|(index, object)| selected_for_callback(object, pass).then_some(index))
        .collect()
}

pub fn acceleration_candidates(objects: &[Object]) -> Vec<usize> {
    // process_machine_simulation_update (1000:0f86) walks the live moving
    // linked list, which is maintained in native mass order. Saved-record
    // order can differ and changes which swept contact is retained when two
    // moving parts meet in the same update.
    crate::contact::native_list_order(objects)
        .into_iter()
        .filter(|&index| {
            objects[index].list == ObjectList::Moving
                && !objects[index]
                    .render_flags
                    .contains(crate::machine::RenderFlag::Hidden)
        })
        .collect()
}

/// The ordinary moving pass invokes its definition callback before integration
/// and collision handling. Definition 17 is handled in three later special passes.
pub fn ordinary_motion_candidates(objects: &[Object]) -> Vec<usize> {
    acceleration_candidates(objects)
        .into_iter()
        .filter(|&index| objects[index].definition != Def::Bucket)
        .collect()
}

/// Run callbacks selected by TIM's three fixed-list passes. This dispatcher
/// deliberately covers the definitions whose callback bodies are ported below.
#[cfg(test)]
pub(super) fn dispatch_fixed_callbacks(
    objects: &mut Vec<Object>,
    forces: &[crate::forces::PartForce; 58],
    tick: u32,
) -> Result<Vec<Def>, SimulationError> {
    dispatch_fixed_callbacks_with_priorities(objects, forces, tick, &[])
}

pub(super) fn dispatch_fixed_callbacks_with_priorities(
    objects: &mut Vec<Object>,
    forces: &[crate::forces::PartForce; 58],
    tick: u32,
    priorities: &[(usize, i32)],
) -> Result<Vec<Def>, SimulationError> {
    let mut unsupported = Vec::new();
    begin_update(objects);
    let pending_lights: Vec<_> = objects
        .iter()
        .enumerate()
        .filter(|(_, object)| object.definition == Def::Light && object.pending_light_power == 2)
        .map(|(index, _)| index)
        .collect();
    // 1000:0f86 visits prior accepted contacts before the powered/gear/ordinary
    // passes. Callback-owned processed bit 0x40 suppresses duplicate entries.
    for &(index, _) in priorities {
        if !objects[index]
            .render_flags
            .contains(crate::machine::RenderFlag::ProcessedThisUpdate)
        {
            let coverage = dispatch_definition_update(objects, index, forces, tick)?;
            if matches!(
                coverage,
                crate::callback_coverage::UpdateCoverage::Unimplemented(_)
            ) && !unsupported.contains(&objects[index].definition)
            {
                unsupported.push(objects[index].definition);
            }
        }
    }
    // A seesaw's newly delivered rope power becomes visible after this update's
    // impulse pass, allowing the seesaw to consume the pending pull first.
    for index in pending_lights {
        if objects[index].pending_light_power != 0 {
            objects[index].power_state = 1;
            objects[index].pending_light_power = 0;
        }
    }
    for pass in [
        CallbackPass::FixedPowered,
        CallbackPass::FixedGears,
        CallbackPass::FixedOrdinary,
    ] {
        // Recheck each predicate immediately before dispatch: an earlier callback
        // can mark a later object processed or disabled (1000:0f86).
        // object_list_head2000 is built by prepending serialized fixed
        // records, so native callbacks walk the fixed list in reverse record
        // order (1000:0f86).
        let count = objects.len();
        for index in (0..count).rev() {
            if selected_for_callback(&objects[index], pass) {
                let coverage = dispatch_definition_update(objects, index, forces, tick)?;
                if matches!(
                    coverage,
                    crate::callback_coverage::UpdateCoverage::Unimplemented(_)
                ) && !unsupported.contains(&objects[index].definition)
                {
                    unsupported.push(objects[index].definition);
                }
            }
        }
    }
    for object in objects.iter_mut().filter(|o| o.definition == Def::Light) {
        if object.pending_light_power == 1 {
            object.pending_light_power = 0;
        }
    }
    Ok(unsupported)
}

pub(super) fn dispatch_definition_update(
    objects: &mut Vec<Object>,
    index: usize,
    forces: &[crate::forces::PartForce; 58],
    tick: u32,
) -> Result<crate::callback_coverage::UpdateCoverage, SimulationError> {
    let result: Result<(), String> = (|| {
        match objects[index].definition {
            Def::Bellows => process_bellows(objects, index, forces),
            Def::Trampoline => update_trampoline_bounce(&mut objects[index]),
            Def::ElectricalOutlet => update_electrical_outlet_rotation(objects, index)?,
            Def::CatPoweredDrive => update_cat_powered_drive(objects, index),
            Def::Monkey => update_monkey_behavior(objects, index),
            Def::ElectricalMotor => update_electrical_motor_links_and_rotation(objects, index),
            Def::JackInTheBox => update_jack_in_the_box(objects, index),
            Def::MagnifyingGlass => update_magnifying_glass_focus(objects, index),
            Def::BoxingGlove => update_boxing_glove_punch(objects, index),
            Def::Balloon => update_balloon_pop(
                objects,
                index,
                forces[Def::RopeEnd.id() as usize].speed_limit,
            ),
            Def::Explosion => update_explosion_expand(objects, index),
            Def::Dynamite => update_dynamite_fuse(objects, index),
            Def::Detonator => update_detonator_trigger(objects, index),
            Def::Rocket => update_rocket_flight(objects, index, forces),
            Def::Gun => update_gun_launch(objects, index, forces),
            Def::Bullet => update_bullet_flight(&mut objects[index]),
            Def::Candle => update_candle_flame(objects, index),
            Def::Flashlight => update_flashlight_switch(&mut objects[index]),
            Def::SolarPanel => update_solar_panel_power(objects, index, tick)?,
            Def::Fish => update_fish_swim(&mut objects[index]),
            Def::Mouse => update_mouse_behavior(objects, index),
            Def::Cat => update_cat_behavior(objects, index),
            Def::CreditsProducer => start_part_horizontal_motion(&mut objects[index]),
            Def::Seesaw => apply_seesaw_tilt_and_throw_parts(objects, index),
            Def::Windmill => update_windmill_rotation(objects, index),
            Def::Gear => update_gear_rotation(objects, index)?,
            Def::Motor => {
                update_motor_drive(objects, index)?;
            }
            Def::Cannon => process_cannon(objects, index),
            Def::PowerDistributor => update_power_distributor_links(objects, index)?,
            Def::Fan => update_fan_airflow(objects, index, forces),
            Def::Light => {
                illuminate_light_on_power(objects, index);
            }
            Def::Scissors => update_scissors_snap(objects, index)?,
            _ => {}
        }
        Ok(())
    })();
    result.map_err(|reason| SimulationError::CallbackFailure {
        tick,
        object: index,
        definition: objects[index].definition,
        reason,
    })?;
    Ok(crate::callback_coverage::update_coverage(
        objects[index].definition,
    ))
}
