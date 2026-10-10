use super::*;
#[test]
fn native_axis_projection_and_mass_boundaries() {
    assert_eq!(
        (component(0), component(0x4000), component(0x8000)),
        (16384, 0, -16384)
    );
    for (m, v) in [
        (1, 0x1800),
        (2, 0x1600),
        (5, 0x1600),
        (6, 0x1400),
        (20, 0x1200),
        (21, 0x1000),
        (120, 0x1000),
        (121, 0xe00),
        (150, 0xe00),
        (151, 0xc00),
        (200, 0xc00),
        (201, 0xa00),
        (1800, 0xa00),
        (1801, 0x800),
    ] {
        assert_eq!(magnitude(m), v);
    }
    assert_eq!(crate::outline::vector_angle(1, 0), 0xc000);
    assert_eq!(crate::outline::vector_angle(0, 1), 0);
    assert_eq!(crate::outline::vector_angle(-1, 0), 0x4000);
    assert_eq!(crate::outline::vector_angle(0, -1), 0x8000);
}
