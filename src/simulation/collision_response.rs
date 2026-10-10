use super::*;
use crate::sound::SoundEffect;

const CONTACT_TABLE_INDEX_SHIFT: u32 = 4;
const CONTACT_TABLE_HALF_TURN_BIT: usize = 2048;
const CONTACT_TABLE_FULL_TURN: usize = 4096;
const QUARTER_TURN: u16 = 16384;
const HALF_TURN_BIT: u16 = 32768;
const TRIG_FRACTION_BITS: u32 = 14;
const FRICTION_FRACTION_BITS: u32 = 8;
const BOWLING_ROLL_SOUND_THRESHOLD: i32 = 4096;
const BOUNCE_DAMPING: i16 = 64;
const BOOSTED_SURFACE_DRAG: i16 = 32;
const ORDINARY_SURFACE_DRAG: i16 = 2;
const AIR_DISTANCE_SCALE: i32 = 256;
const AIR_FRACTION_BITS: u32 = 8;

pub(super) fn contact_cosine(angle: u16) -> i32 {
    let mut index = usize::from(angle >> CONTACT_TABLE_INDEX_SHIFT);
    if index & CONTACT_TABLE_HALF_TURN_BIT != 0 {
        index = CONTACT_TABLE_FULL_TURN - index;
    }
    i32::from(contact_trig::COSINE[index])
}

/// rotate_vector_by_angle, 1000:3b17: signed Q14 products, arithmetic shift.
pub(super) fn rotate_vector_by_angle(x: i16, y: i16, angle: u16) -> (i16, i16) {
    let cosine = contact_cosine(angle);
    let sine = contact_cosine(angle.wrapping_sub(QUARTER_TURN));
    (
        ((i32::from(x) * cosine - i32::from(y) * sine) >> TRIG_FRACTION_BITS) as i16,
        ((i32::from(x) * sine + i32::from(y) * cosine) >> TRIG_FRACTION_BITS) as i16,
    )
}

/// play_bowling_ball_rolling_sound, 1000:3009: a bowling ball striking
/// something faster than 4096 (|vx| + |vy|).
pub(super) fn play_bowling_ball_rolling_sound(object: &Object) {
    let speed =
        i32::from(object.motion.velocity_x).abs() + i32::from(object.motion.velocity_y).abs();
    if object.definition == Def::BowlingBall && speed > BOWLING_ROLL_SOUND_THRESHOLD {
        crate::sound::play_effect(SoundEffect::BowlingBallRolling);
    }
}

/// reflect_velocity_off_contact_surface, 1000:3046, velocity portion.
/// SAT supplies the target edge; native endpoint flags and pixel snapping remain pending.
pub(super) fn reflect_velocity_off_contact_surface(
    motion: &mut crate::motion::Motion,
    angle: u16,
    restitution: i16,
    speed_limit: i16,
) {
    let (tangent, normal) = rotate_vector_by_angle(motion.velocity_x, motion.velocity_y, angle);
    let reflected = ((i32::from(normal) * i32::from(restitution)) >> FRICTION_FRACTION_BITS) as i16;
    let reflected = reflected.wrapping_neg();
    let damped = if reflected < 0 {
        reflected.saturating_add(BOUNCE_DAMPING).min(0)
    } else {
        reflected.saturating_sub(BOUNCE_DAMPING).max(0)
    };
    let (x, y) = rotate_vector_by_angle(tangent, damped, angle.wrapping_neg());
    motion.velocity_x = x;
    motion.velocity_y = y;
    motion.limit_velocity(speed_limit);
}

/// resolve_contact_surface_response, 1000:2da0, supported velocity calculation.
/// Native Q14 trigonometry and Q8 friction; SAT supplies support eligibility.
pub(super) fn resolve_contact_surface_response(
    motion: &mut crate::motion::Motion,
    flags: u16,
    angle: u16,
    acceleration: i16,
    friction: i16,
    speed_limit: i16,
) {
    let cosine = contact_cosine(angle.wrapping_neg());
    let sine = contact_cosine(angle.wrapping_neg().wrapping_sub(QUARTER_TURN));
    let product = |a: i32, b: i32| ((a * b) >> TRIG_FRACTION_BITS) as i16;
    let gravity = i32::from(acceleration.wrapping_abs());
    let velocity = motion.velocity_x;
    let load = product(cosine, gravity);
    let inward = if (velocity > 0 && (angle as i16) > 0) || (velocity < 0 && (angle as i16) < 0) {
        product(sine, i32::from(velocity))
    } else {
        0
    };
    let load = load.wrapping_abs().wrapping_add(inward.wrapping_abs());
    let friction_load = ((i32::from(load) * i32::from(friction)) >> FRICTION_FRACTION_BITS) as i16;
    let drag = product(cosine, i32::from(friction_load))
        .wrapping_abs()
        .wrapping_add(if flags & UpdateFlag::SurfaceDragBoost.mask() != 0 {
            BOOSTED_SURFACE_DRAG
        } else {
            ORDINARY_SURFACE_DRAG
        });
    let along = product(sine, gravity);
    let along = product(cosine.abs(), i32::from(along));
    let velocity = velocity.wrapping_add(along);
    let velocity = if velocity < 0 {
        velocity.wrapping_add(drag).min(0)
    } else {
        velocity.wrapping_sub(drag).max(0)
    };
    motion.velocity_x = velocity;
    let projection = if angle.wrapping_add(QUARTER_TURN) & HALF_TURN_BIT != 0 {
        velocity.wrapping_neg()
    } else {
        velocity
    };
    motion.velocity_y = product(sine, i32::from(projection));
    motion.limit_velocity(speed_limit);
}

/// 272c:3fae cycles an activated spring and clears power at state 5.
/// Native response return: one requests ordinary collision handling, zero
/// suppresses it. Keep this separate from update callback coverage.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum CollisionResponse {
    Ordinary,
    SuppressBounce,
}

/// Native simple target responses: 016e (balloon), 0552 (glove),
/// and 0c6c (cat). Each retains ordinary collision handling.
pub(super) fn simple_target_contact(
    objects: &mut [Object],
    moving: usize,
    target: usize,
    edge: u16,
) -> CollisionResponse {
    let source = objects[moving].definition;
    let part = &mut objects[target];
    match part.definition {
        Def::Balloon if source == Def::Bullet => part.power_state = 1,
        Def::BoxingGlove if edge == 2 => part.power_state = 1,
        Def::Cat if part.state == CAT_IDLE_FRAME => {
            part.state = CAT_STARTLED_FRAME;
            part.activation_timer = 0;
            crate::sound::play_effect(SoundEffect::CatStartled);
        }
        _ => {}
    }
    CollisionResponse::Ordinary
}

/// Native 272c:0405: signed compression/return pulse and directed pressure.
/// The push a bellows (272c:0405, strength 2048) or fan (272c:1a82,
/// 4096) gives a part: the 32-bit product strength * (256 - |gap|)
/// (3000:a269), shifted right 8 bits (1000:be62, CL = 8), divided by the
/// part's mass (1000:bd93), and added to the 16-bit velocity. Without the
/// shift a light part's push overflowed and threw it the wrong way.
pub(super) fn air_impulse(strength: i32, distance: i32, definition: Def) -> i32 {
    ((strength * (AIR_DISTANCE_SCALE - distance)) >> AIR_FRACTION_BITS)
        / i32::from(crate::forces::mass(definition))
}
