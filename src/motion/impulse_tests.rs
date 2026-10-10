use super::*;
#[test]
fn native_impulse_signed_product_and_wrapping_speed_sum() {
    let mut m = Motion::at_pixels(0, 0);
    m.velocity_x = -512;
    m.velocity_y = 1024;
    assert_eq!(m.collision_impulse(10), 0x0600_3c00);
    assert_eq!(m.collision_impulse(-10), 0x0600_c400);
    m.velocity_x = i16::MIN;
    m.velocity_y = 0;
    assert_eq!(m.collision_impulse(2), i32::MIN);
    m.velocity_x = 20000;
    m.velocity_y = 20000;
    assert_eq!(m.collision_impulse(1), 0x9c40_9c40u32 as i32);
}
