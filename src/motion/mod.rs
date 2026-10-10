//! TIM integer motion, reconstructed from 1000:2bcc and 1000:2c93.
//! Collision response and per-definition force initialization are separate.

pub const POSITION_FRACTION_BITS: u32 = 9;
const SUPPORTED_POSITION_ADJUSTMENT: i32 = 1024;
const MIN_POSITION: i16 = -1000;
const MAX_POSITION: i16 = 6000;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Motion {
    pub position_x: i32,
    pub position_y: i32,
    pub velocity_x: i16,
    pub velocity_y: i16,
}
impl Motion {
    pub fn at_pixels(x: i16, y: i16) -> Self {
        Self {
            position_x: (x as i32) << POSITION_FRACTION_BITS,
            position_y: (y as i32) << POSITION_FRACTION_BITS,
            velocity_x: 0,
            velocity_y: 0,
        }
    }
    pub fn pixels(&self) -> (i16, i16) {
        (
            (self.position_x >> POSITION_FRACTION_BITS) as i16,
            (self.position_y >> POSITION_FRACTION_BITS) as i16,
        )
    }
    /// 1000:2c39 adds the definition's acceleration before 1000:2bcc caps speed.
    pub fn apply_vertical_acceleration(&mut self, acceleration: i16, speed_limit: i16) {
        self.velocity_y = self.velocity_y.wrapping_add(acceleration);
        self.limit_velocity(speed_limit);
    }
    /// Native 1000:2c39 stores +0x3c/+0x3e after acceleration and limiting.
    /// The absolute-value sum wraps as a signed 16-bit multiply operand.
    pub fn collision_impulse(self, effective_mass: i16) -> i32 {
        let speed = self
            .velocity_x
            .wrapping_abs()
            .wrapping_add(self.velocity_y.wrapping_abs());
        // apply_object_acceleration_and_impulse (1000:2c39) stores two words:
        // +0x3e is |vx|+|vy|, and +0x3c is that sum times effective mass.
        // queue_contact_impulse_record (1000:7b6f) compares Y as signed first,
        // then X as unsigned. Keep the native pair packed in the i32 field.
        let product = i32::from(speed).wrapping_mul(i32::from(effective_mass)) as u16;
        ((u32::from(speed as u16) << 16) | u32::from(product)) as i32
    }
    /// Both axes use the same positive, signed definition-table speed limit.
    pub fn limit_velocity(&mut self, speed_limit: i16) {
        assert!(speed_limit >= 0, "TIM speed limit must be nonnegative");
        self.velocity_x = self.velocity_x.clamp(-speed_limit, speed_limit);
        self.velocity_y = self.velocity_y.clamp(-speed_limit, speed_limit);
    }
    /// Integrates sign-extended velocities with 32-bit DOS arithmetic.
    /// Contact flag 1 applies the observed +/-1024 vertical adjustment.
    pub fn integrate(&mut self, contact_flags: u16, acceleration: i16) {
        self.position_x = self.position_x.wrapping_add(self.velocity_x as i32);
        self.position_y = self.position_y.wrapping_add(self.velocity_y as i32);
        if contact_flags & crate::machine::UpdateFlag::Supported.mask() != 0 {
            let adjustment = if acceleration > 0 {
                SUPPORTED_POSITION_ADJUSTMENT
            } else {
                -SUPPORTED_POSITION_ADJUSTMENT
            };
            self.position_y = self.position_y.wrapping_add(adjustment);
        }
        clamp_position(&mut self.position_x);
        clamp_position(&mut self.position_y);
    }
}
fn clamp_position(position: &mut i32) {
    // The DOS routine compares the low signed word after its arithmetic shift.
    let pixel = (*position >> POSITION_FRACTION_BITS) as i16;
    if pixel < MIN_POSITION {
        *position = (MIN_POSITION as i32) << POSITION_FRACTION_BITS;
    } else if pixel > MAX_POSITION {
        *position = (MAX_POSITION as i32) << POSITION_FRACTION_BITS;
    }
}

#[cfg(test)]
mod impulse_tests;
