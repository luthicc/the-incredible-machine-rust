use super::*;

// The match is the pre-refactor native curve, independent of table lookup.
#[test]
fn launch_table_preserves_every_signed_native_mass() {
    for mass in i16::MIN..=i16::MAX {
        let native_speed = match mass {
            ..=5 => 6656,
            6..=9 => 6144,
            10..=20 => 5632,
            21..=120 => 5120,
            121..=150 => 4608,
            151..=200 => 4096,
            201..=1800 => 3584,
            _ => 3072,
        };
        assert_eq!(
            launch_speed_for_mass(mass, PUNCH_SPEED_BY_MASS),
            native_speed,
            "mass {mass}"
        );
    }
}
