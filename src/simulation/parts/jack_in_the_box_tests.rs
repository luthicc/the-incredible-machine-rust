use super::*;

// The match is the pre-refactor native curve, independent of table lookup.
#[test]
fn launch_table_preserves_every_signed_native_mass() {
    for mass in i16::MIN..=i16::MAX {
        let native_speed = match mass {
            ..=1 => 6144,
            2..=5 => 5632,
            6..=9 => 5120,
            10..=20 => 4608,
            21..=120 => 4096,
            121..=150 => 3584,
            151..=200 => 3072,
            201..=1800 => 2560,
            _ => 2048,
        };
        assert_eq!(
            launch_speed_for_mass(mass, THROW_SPEED_BY_MASS),
            native_speed,
            "mass {mass}"
        );
    }
}
