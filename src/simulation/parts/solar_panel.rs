use super::*;

const ILLUMINATION_SAMPLE_PERIOD: u32 = 8;
const ILLUMINATION_SAMPLE_PHASE: u32 = 4;
const ILLUMINATION_HORIZONTAL: (i16, i16) = (-26, 26);
const ILLUMINATION_VERTICAL: (i16, i16) = (-26, 26);

/// Solar panel update 272c:3e08 samples illumination on tick phase 4/8,
/// but forwards its retained power to both electrical outputs every update.
pub(super) fn update_solar_panel_power(
    objects: &mut [Object],
    index: usize,
    tick: u32,
) -> Result<(), String> {
    use crate::spatial::{Facing, nearby_parts};
    objects[index]
        .render_flags
        .insert(crate::machine::RenderFlag::ProcessedThisUpdate);
    if tick & (ILLUMINATION_SAMPLE_PERIOD - 1) == ILLUMINATION_SAMPLE_PHASE {
        objects[index].power_state = 0;
        let contacts = nearby_parts(
            objects,
            index,
            ILLUMINATION_HORIZONTAL,
            ILLUMINATION_VERTICAL,
        );
        for contact in contacts {
            let source = &objects[contact.index];
            if source.power_state == 0 {
                continue;
            }
            let illuminates = match source.definition {
                Def::Light | Def::Candle | Def::Explosion => true,
                Def::Flashlight => {
                    let facing = Facing::from_render_flags(source.render_flags);
                    if contact.horizontal_gap < 0 {
                        facing == Facing::Right
                    } else {
                        facing == Facing::Left
                    }
                }
                _ => false,
            };
            if illuminates {
                objects[index].power_state = 1;
            }
        }
    }
    let power = objects[index].power_state;
    let outputs = [
        objects[index].linked_objects[ELECTRICAL_OUTPUT_LINKS[0]],
        objects[index].linked_objects[ELECTRICAL_OUTPUT_LINKS[1]],
    ];
    for output in outputs.into_iter().flatten() {
        objects
            .get_mut(output)
            .ok_or("Invalid solar-panel electrical link")?
            .power_state = power;
    }
    Ok(())
}
