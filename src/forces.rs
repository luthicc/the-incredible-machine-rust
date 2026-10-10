//! Definition forces from Ghidra 1000:2ac0.
#[path = "tables/part_density.rs"]
mod part_density;

#[derive(Clone, Copy, Debug)]
pub struct PartForce {
    pub acceleration: i16,
    pub speed_limit: i16,
}

/// Builds the original 58 definition entries for a collection's environment.
/// Products are signed 32-bit; division truncates toward zero as in the DOS helper.
pub fn initialize(air: i16, gravity: i16) -> [PartForce; 58] {
    let gravity = if gravity < 140 {
        (gravity >> 2).wrapping_add(1)
    } else if gravity > 278 {
        gravity.wrapping_shl(1)
    } else {
        gravity
    };
    let air = if air < 70 {
        air >> 1
    } else {
        air.wrapping_shl(4)
    };
    std::array::from_fn(|definition| {
        let density = part_density::PART_DENSITY[definition];
        let acceleration = if density == air {
            0
        } else if density > air {
            gravity.wrapping_sub(((air as i32 * gravity as i32) / density as i32) as i16)
        } else {
            (((density as i32 * gravity as i32) / air as i32) as i16).wrapping_sub(gravity)
        };
        PartForce {
            acceleration: if definition == 20 { 0 } else { acceleration },
            speed_limit: if definition == 20 || definition == 43 {
                0x3000
            } else {
                0x2600_i16.wrapping_sub(air)
            },
        }
    })
}

/// Original part-definition density used by wind-force callbacks.
pub fn density(definition: usize) -> i16 {
    part_density::PART_DENSITY[definition]
}

#[path = "tables/part_mass.rs"]
mod part_mass;

pub fn mass(definition: crate::definition::PartDefinition) -> i16 {
    part_mass::PART_MASS[definition.id() as usize]
}
