//! Puzzle win conditions. TIM keeps them in code, not in the LEV files:
//! run_game_loop (1000:12ab) calls run_level_goal_check (1000:1465) after every
//! update, which dispatches through the far-pointer table at DS:2632 indexed
//! by puzzle number. A goal ends the run by setting game_mode to 0x200.
//!
//! The check runs after advance_all_object_histories, so the native
//! "previous" position equals the current one and the native "saved"
//! position (+0x26/+0x28) is the position one update earlier, which is
//! Rust's `previous_position` once `step` has returned. Likewise the native
//! `older_state` is Rust's `previous_state`.
mod goal_state;
pub use goal_state::GoalState;

use crate::definition::PartDefinition as Def;
use crate::machine::{Object, ObjectList};
use crate::machine::{PropertyFlag, UpdateFlag};

/// Native list order for goal iteration. The moving list (head 1000) is kept
/// sorted by definition mass by insert_object_into_sorted_list; equal masses
/// keep insertion order. The fixed list (head 2000) is prepended to.
fn moving(objects: &[Object]) -> Vec<&Object> {
    let mut list: Vec<_> = objects
        .iter()
        .filter(|o| o.list == ObjectList::Moving)
        .collect();
    list.sort_by_key(|o| crate::forces::mass(o.definition));
    list
}
fn fixed(objects: &[Object]) -> Vec<&Object> {
    objects
        .iter()
        .filter(|o| o.list == ObjectList::Fixed)
        .rev()
        .collect()
}
/// get_object_list_head_for_mask(0x3000) walks the fixed list, then moving.
fn all(objects: &[Object]) -> Vec<&Object> {
    let mut list = fixed(objects);
    list.extend(moving(objects));
    list
}
fn bin(objects: &[Object]) -> impl Iterator<Item = &Object> {
    objects.iter().filter(|o| o.list == ObjectList::Inventory)
}
fn hidden(o: &Object) -> bool {
    o.render_flags.contains(crate::machine::RenderFlag::Hidden)
}
fn locked(o: &Object) -> bool {
    o.update_flags & UpdateFlag::Locked.mask() != 0
}
fn in_bucket(o: &Object) -> bool {
    o.flags & PropertyFlag::CarriedByBucket.mask() != 0
}
/// Native `reset_pixel_x/y` (+0x8c/+0x8e): the placed position.
fn reset(o: &Object) -> (i16, i16) {
    o.saved_position
}

/// Goal routine for `level` (1-based), from the DS:2632 table.
pub fn check(level: u16, objects: &[Object], state: &mut GoalState) -> bool {
    let m = || moving(objects);
    let f = || fixed(objects);
    let all_moving = |pred: &dyn Fn(&Object) -> bool| m().into_iter().all(pred);
    let any_moving = |pred: &dyn Fn(&Object) -> bool| m().into_iter().any(pred);
    let all_fixed = |pred: &dyn Fn(&Object) -> bool| f().into_iter().all(pred);
    match level {
        // check_level_goal_01, 1000:151b: the first basketball in the hoop.
        1 => m()
            .into_iter()
            .find(|o| o.definition == Def::Basketball)
            .is_some_and(|o| (0x1a9..0x1da).contains(&o.x) && (0x89..0x98).contains(&o.y)),
        // 1000:1476: every bowling ball resting at y 0x108.
        2 => all_moving(&|o| o.definition != Def::BowlingBall || o.y == 0x108),
        // 1000:15fa: every puzzle balloon popped.
        3 | 8 | 27 | 30 | 33 | 45 | 50 | 62 => {
            all_moving(&|o| o.definition != Def::Balloon || !locked(o) || hidden(o))
        }
        // 1000:1cc4: a bowling ball below y 0x170.
        4 => any_moving(&|o| o.definition == Def::BowlingBall && o.y > 0x170),
        // 1000:1cea: a baseball on the pipe at y 0xf9.
        5 => any_moving(&|o| {
            o.definition == Def::Baseball && (0x18..0xf4).contains(&o.x) && o.y == 0xf9
        }),
        // 1000:1d1d: every mouse cage powered for more than four updates.
        6 | 58 => {
            if all_fixed(&|o| o.definition != Def::CatPoweredDrive || o.power_state != 0) {
                state.flags[0] = state.flags[0].wrapping_add(1);
            } else {
                state.flags[0] = 0;
            }
            state.flags[0] as i16 > 4
        }
        // 1000:1d5e: every fishbowl broken.
        7 | 51 | 65 => all_fixed(&|o| o.definition != Def::Fish || o.state >= 11),
        // 1000:1d8c: every bucket lowered to y 0xf8.
        9 => all_moving(&|o| o.definition != Def::Bucket || o.y == 0xf8),
        // 1000:1846: every gun fired, none left in the bin.
        10 | 32 => {
            all_fixed(&|o| o.definition != Def::Gun || o.state >= 6)
                && bin(objects).all(|o| o.definition != Def::Gun)
        }
        // 1000:1dbb: all three fans running.
        11 => {
            let fans: Vec<_> = f()
                .into_iter()
                .filter(|o| o.definition == Def::Fan)
                .collect();
            fans.len() == 3 && fans.iter().all(|o| o.power_state != 0)
        }
        // 1000:1df1: a tennis ball in the pipe hole at y 0x139.
        12 => any_moving(&|o| o.definition == Def::TennisBall && o.x > 0x153 && o.y == 0x139),
        // 1000:1e1e: every gear turning for more than twelve updates.
        13 => {
            if all_fixed(&|o| o.definition != Def::Gear || o.state != o.previous_state) {
                state.flags[0] = state.flags[0].wrapping_add(1);
            }
            state.flags[0] as i16 > 12
        }
        // 1000:1907: every cannon fired (state 11).
        14 | 15 | 64 | 73 => all_fixed(&|o| o.definition != Def::Cannon || o.state == 11),
        // 1000:1a49: every candle lit.
        16 | 56 | 83 => all_moving(&|o| o.definition != Def::Candle || o.state != 0),
        // 1000:1e59: detonators pressed and all dynamite spent.
        17 => all(objects).into_iter().all(|o| match o.definition {
            Def::Detonator => o.state == 2,
            Def::Explosion | Def::Dynamite => hidden(o),
            _ => true,
        }),
        // 1000:1eb9: the mouse reaches y 0x10d past x 0x198.
        18 => any_moving(&|o| o.definition == Def::Mouse && o.x > 0x198 && o.y == 0x10d),
        // 1000:1b89: every light switch on and held for twelve updates; a
        // switch turning off after any success ends the run as well.
        19 | 48 => {
            let on = all_fixed(&|o| o.definition != Def::Monkey || (o.state != 0 && o.state <= 4));
            if on {
                state.flags[0] = state.flags[0].wrapping_add(1);
            }
            state.flags[0] as i16 > 12 || (!on && state.flags[0] != 0)
        }
        // 1000:14ad: the moving-list head past x 0x1e0 at y 200.
        20 => m().first().is_some_and(|o| o.x > 0x1e0 && o.y == 200),
        // 1000:14cc: the first cat below y 300.
        21 => m()
            .into_iter()
            .find(|o| o.definition == Def::Cat)
            .is_some_and(|o| o.y > 300),
        // 1000:14ee: at most one balloon left unpopped (state 0).
        22 => {
            m().into_iter()
                .filter(|o| o.definition == Def::Balloon && o.state == 0)
                .count()
                < 2
        }
        // 1000:16a6: baseballs in the right bin, tennis balls left of it.
        23 => all_moving(&|o| match o.definition {
            Def::Baseball => (0x1e8..=0x210).contains(&o.x) && o.y >= 0x110,
            Def::TennisBall => o.x < 0x1e8 && o.y >= 0x110,
            _ => true,
        }),
        // 1000:1935: every balloon intact and still for 20 updates.
        24 => {
            all_moving(&|o| {
                o.definition != Def::Balloon || (o.state == 0 && (o.x, o.y) == o.previous_position)
            }) && state.level_timer >= 0x14
        }
        // 1000:17db: every puzzle balloon placed right of x 300 popped.
        25 => all_moving(&|o| {
            o.definition != Def::Balloon || reset(o).0 <= 300 || !locked(o) || hidden(o)
        }),
        // 1000:16fb: a bowling ball dropped exactly 0x40 from its start.
        26 => any_moving(&|o| {
            o.definition == Def::BowlingBall
                && reset(o).1 > 100
                && o.y.wrapping_sub(reset(o).1) == 0x40
        }),
        // check_baseballs_on_six_markers, 1000:2065.
        28 => [0x39, 0x99, 0xf9, 0x69, 0xc9, 0x129]
            .iter()
            .all(|&y| any_moving(&|o| o.definition == Def::Baseball && o.y == y)),
        // check_all_baseballs_past_right_marker, 1000:2010.
        29 => all_moving(&|o| o.definition != Def::Baseball || o.y >= 0x170),
        // 1000:1fa6: every basketball in the target box.
        31 => all_moving(&|o| {
            o.definition != Def::Basketball || ((0x156..=0x1ba).contains(&o.x) && o.y >= 0xda)
        }),
        // 1000:18d9: every jack-in-the-box popped (state 0x12).
        34 => all_fixed(&|o| o.definition != Def::JackInTheBox || o.state == 0x12),
        // check_bowlingball_at_target, 1000:242c.
        35 => any_moving(&|o| {
            o.definition == Def::BowlingBall
                && (0x1d6..0x1fd).contains(&o.x)
                && (0xc6..0xd1).contains(&o.y)
        }),
        // check_fish_survives_cat_goal, 1000:20fa.
        36 => {
            let mut done = true;
            for o in all(objects) {
                match o.definition {
                    Def::Gun if o.state < 6 => done = false,
                    Def::Fish if o.state > 10 => done = false,
                    Def::Cat if o.state != 0 => state.flags[0] = 1,
                    Def::Bullet if !hidden(o) => done = false,
                    _ => {}
                }
            }
            done && state.flags[0] == 0
        }
        // check_mouse_in_lower_left_zone, 1000:2260.
        37 => any_moving(&|o| {
            o.definition == Def::Mouse && (0x20..0x79).contains(&o.x) && o.y > 0x120
        }),
        // 1000:19ac: a bowling ball on the ledge at y 0xe8.
        38 => any_moving(&|o| {
            o.definition == Def::BowlingBall && (0x148..0x169).contains(&o.x) && o.y == 0xe8
        }),
        // 1000:1753: cage down, cat at x 0x1a9, every cannonball in a bucket.
        39 => {
            let mut done = true;
            let mut projectile = false;
            for o in m() {
                match o.definition {
                    Def::BirdCage if o.y != 0xf8 => done = false,
                    Def::Cat if o.x != 0x1a9 => done = false,
                    Def::CannonProjectile => {
                        projectile = true;
                        done &= in_bucket(o);
                    }
                    _ => {}
                }
            }
            done && projectile
        }
        // check_basketball_at_target, 1000:23ef.
        40 => all_moving(&|o| {
            o.definition != Def::Basketball || ((0xf6..=0x14c).contains(&o.x) && o.y == 0xe8)
        }),
        // 1000:1819: a cat at y 0x11f past x 0x1b9.
        41 => any_moving(&|o| o.definition == Def::Cat && o.x > 0x1b9 && o.y == 0x11f),
        // 1000:1f25: all dynamite and explosions spent.
        42 | 75 => all(objects)
            .into_iter()
            .all(|o| !matches!(o.definition, Def::Dynamite | Def::Explosion) || hidden(o)),
        // 1000:172d: a bucket lowered to y 0x118.
        43 => any_moving(&|o| o.definition == Def::Bucket && o.y == 0x118),
        // 1000:19e0: a tennis ball past x 0x117 below y 0x5a.
        44 => any_moving(&|o| o.definition == Def::TennisBall && o.x > 0x117 && o.y > 0x5a),
        // 1000:1888: fishbowls intact and every cannonball off screen.
        46 => all(objects).into_iter().all(|o| match o.definition {
            Def::Fish => o.state <= 10,
            Def::CannonProjectile => o.y >= 0x170,
            _ => true,
        }),
        // 1000:1ab0: buckets at y 0x118 on both sides.
        47 => {
            let at_floor = |o: &&Object| o.definition == Def::Bucket && o.y == 0x118;
            m().into_iter().filter(at_floor).any(|o| o.x < 0x75)
                && m().into_iter().filter(at_floor).any(|o| o.x > 0x197)
        }
        // check_baseball_and_tennisball_zones, 1000:23a4.
        49 => all_moving(&|o| match o.definition {
            Def::Baseball => o.x <= 0x4a && o.y >= 0x124,
            Def::TennisBall => o.x >= 0x1c6 && o.y >= 0x124,
            _ => true,
        }),
        // 1000:1b63: a birdcage lowered to y 0x108.
        52 => any_moving(&|o| o.definition == Def::BirdCage && o.y == 0x108),
        // 1000:17ad: every rocket above the screen.
        53 | 54 | 63 | 67 | 87 => all_moving(&|o| o.definition != Def::Rocket || o.y <= -0x30),
        // 1000:197e: every boxing glove punched (state 9).
        55 => all_fixed(&|o| o.definition != Def::BoxingGlove || o.state == 9),
        // 1000:1f77: every mouse off the bottom.
        57 | 74 => all_moving(&|o| o.definition != Def::Mouse || o.y >= 0x170),
        // check_bucket_and_projectile_zone, 1000:2351.
        59 => {
            let caught = any_moving(&|o| o.definition == Def::CannonProjectile && in_bucket(o));
            caught
                && all_moving(&|o| {
                    o.definition != Def::Bucket || ((0xd0..=0xee).contains(&o.x) && o.y == 0x128)
                })
        }
        // check_all_cats_past_marker, 1000:2231.
        60 => all_moving(&|o| o.definition != Def::Cat || o.y >= 200),
        // check_birdcage_at_left_marker, 1000:203f.
        61 => any_moving(&|o| o.definition == Def::BirdCage && o.y == 0xf8),
        // 1000:1fe3: a baseball placed at x 0x219 has fallen below y 0x3f.
        66 => any_moving(&|o| o.definition == Def::Baseball && reset(o).0 == 0x219 && o.y > 0x3f),
        // check_mouse_beside_birdcage, 1000:22d8: uses the last mouse and cage.
        68 => {
            let mut mouse_x = 0;
            let mut cage_x = 0;
            let mut done = true;
            for o in m() {
                if o.definition == Def::Mouse {
                    mouse_x = o.x;
                }
                if o.definition == Def::BirdCage {
                    cage_x = o.x;
                    done &= o.y == 0x108;
                }
            }
            done && cage_x < mouse_x && mouse_x < cage_x + 0x32
        }
        // 1000:1b2f: every light on (states 1 or 3).
        69 => all_fixed(&|o| o.definition != Def::Light || (o.state != 0 && o.state != 2)),
        // 1000:1af7: fishbowls intact for 0x134 updates.
        70 => {
            all_fixed(&|o| o.definition != Def::Fish || o.state <= 10) && state.frame_timer >= 0x134
        }
        // 1000:1a0c: every baseball in the target slot.
        71 => all_moving(&|o| {
            o.definition != Def::Baseball || ((0x168..=0x17a).contains(&o.x) && o.y >= 0xc1)
        }),
        // check_all_baseballs_activated, 1000:2322: every baseball in a bucket.
        72 => all_moving(&|o| o.definition != Def::Baseball || in_bucket(o)),
        // 1000:1ee6: every puzzle balloon popped by update 0x82.
        76 => {
            all_moving(&|o| o.definition != Def::Balloon || !locked(o) || o.state == 0)
                && state.level_timer >= 0x82
        }
        // check_balls_in_left_target_zone, 1000:21a6.
        77 => {
            let projectile = any_moving(&|o| o.definition == Def::CannonProjectile);
            projectile
                && all_moving(&|o| {
                    !matches!(
                        o.definition,
                        Def::Basketball | Def::BowlingBall | Def::CannonProjectile
                    ) || ((0x18..=0x94).contains(&o.x) && o.y >= 0x100)
                })
        }
        // 1000:1552: cats past their start, fishbowls whole, cage still up,
        // and every puzzle mouse cage powered at least once.
        78 => {
            let mut done = true;
            let mut cage = 0;
            for o in all(objects) {
                match o.definition {
                    Def::Cat if o.x <= reset(o).0 || o.state != 0 => done = false,
                    Def::Fish if o.state > 10 => done = false,
                    Def::BirdCage if reset(o).1 <= o.y.wrapping_add(8) => done = false,
                    Def::CatPoweredDrive if locked(o) => {
                        if o.power_state != 0 {
                            state.flags[cage.min(9)] = 1;
                        } else if state.flags[cage.min(9)] == 0 {
                            done = false;
                        }
                        cage += 1;
                    }
                    _ => {}
                }
            }
            done
        }
        // 1000:1630: the last basketball and bowling ball in their bins.
        79 => {
            let last = |d| {
                all(objects)
                    .into_iter()
                    .filter(|o| o.definition == d)
                    .next_back()
            };
            let (Some(ball), Some(bowl)) = (last(Def::Basketball), last(Def::BowlingBall)) else {
                return false;
            };
            (0x149..0x198).contains(&ball.x)
                && ball.y > 0x11c
                && (0x1c9..0x21a).contains(&bowl.x)
                && bowl.y > 0x11c
        }
        // 1000:1a77: a basketball in the upper hoop.
        80 => any_moving(&|o| {
            o.definition == Def::Basketball
                && (0x1a8..0x1ba).contains(&o.x)
                && (0x68..0x7a).contains(&o.y)
        }),
        // 1000:1c0a: every cannonball in a bucket, or the last one falling
        // into the last bucket's mouth.
        81 => {
            let mut done = true;
            let mut projectile = None;
            let mut bucket = None;
            for o in m() {
                if o.definition == Def::CannonProjectile {
                    projectile = Some(o);
                    done &= in_bucket(o);
                }
                if o.definition == Def::Bucket {
                    bucket = Some(o);
                }
            }
            let Some(p) = projectile else { return false };
            if done {
                return true;
            }
            let Some(b) = bucket else { return false };
            // Native previous_pixel equals the current position here.
            let centre = p.x + (p.width >> 1) as i16;
            let foot = p.y + p.height as i16;
            b.x < centre + 5
                && centre - 5 < b.x + b.width as i16
                && b.y + 0x16 < foot
                && foot < b.y + b.height as i16
        }
        // 1000:1bd9 (no Ghidra function; decoded from bytes): a basketball at
        // 8 <= x <= 0x28 resting at y 0x28.
        82 => any_moving(&|o| {
            o.definition == Def::Basketball && (8..=0x28).contains(&o.x) && o.y == 0x28
        }),
        // check_projectiles_and_bucket_goal, 1000:2292.
        84 => {
            let projectiles: Vec<_> = m()
                .into_iter()
                .filter(|o| o.definition == Def::CannonProjectile)
                .collect();
            projectiles.len() == 2
                && projectiles.iter().all(|o| in_bucket(o))
                && all_moving(&|o| o.definition != Def::Bucket || o.y >= 5000)
        }
        // check_birdcage_at_upper_target, 1000:21fd.
        85 => any_moving(&|o| {
            o.definition == Def::BirdCage && (0x1b6..0x1c1).contains(&o.x) && o.y == 0x108
        }),
        // check_mouse_at_mid_target, 1000:2172.
        86 => any_moving(&|o| {
            o.definition == Def::Mouse && (0x19b..0x1cd).contains(&o.x) && o.y == 0x12d
        }),
        _ => false,
    }
}
