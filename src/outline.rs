//! Collision vertices from the original part initializers; unknown shapes stay
//! explicit until their initializer is reconstructed.
use crate::definition::PartDefinition as Def;
use crate::machine::Object;
#[path = "tables/outline_tables.rs"]
mod tables;
pub fn vertices(object: &Object) -> Option<Vec<(u8, u8)>> {
    match object.definition {
        // build_power_distributor_outline, 272c:1556, active switch strip.
        Def::PowerDistributor => {
            return Some(if object.state < 4 {
                vec![(8, 8), (15, 8), (15, 9), (8, 9)]
            } else {
                vec![(15, 24), (8, 24), (8, 23), (15, 23)]
            });
        }
        // Original direct-coordinate geometry callbacks.
        Def::Bucket => {
            return Some(vec![
                (0, 19),
                (10, 40),
                (25, 40),
                (36, 20),
                (27, 47),
                (8, 47),
            ]);
        } // 272c:07b2
        Def::Rocket => return Some(vec![(4, 0), (10, 0), (14, 51), (0, 51)]), // 272c:3737
        Def::Dynamite => {
            // build_dynamite_outline, Ghidra 272c:1261 / DS 3d3c:3290.
            // Live DOS outline records at 1000:5be4 / 272c:1261 store the
            // first vertex at y=14 (the collision anchor is separate).
            let normal = [(0, 14), (5, 0), (37, 18), (27, 27), (20, 27)];
            let flipped = [(10, 18), (42, 0), (47, 14), (27, 27), (20, 27)];
            return Some(
                if !object
                    .render_flags
                    .contains(crate::machine::RenderFlag::FlipHorizontal)
                {
                    normal
                } else {
                    flipped
                }
                .to_vec(),
            );
        }
        // build_gun_outline, 272c:23b1 / DS 3d3c:3314 and 3322.
        Def::Gun => {
            return Some(
                if !object
                    .render_flags
                    .contains(crate::machine::RenderFlag::FlipHorizontal)
                {
                    vec![
                        (0, 30),
                        (7, 10),
                        (18, 1),
                        (63, 3),
                        (63, 9),
                        (27, 16),
                        (12, 30),
                    ]
                } else {
                    vec![
                        (0, 3),
                        (45, 1),
                        (56, 10),
                        (63, 30),
                        (51, 30),
                        (36, 16),
                        (0, 9),
                    ]
                },
            );
        }
        // build_bullet_outline, 272c:08a1 / DS 3d3c:3222 and 322a.
        Def::Bullet => {
            return Some(
                if !object
                    .render_flags
                    .contains(crate::machine::RenderFlag::FlipHorizontal)
                {
                    vec![(28, 0), (39, 1), (39, 5), (28, 6)]
                } else {
                    vec![(0, 0), (11, 1), (11, 5), (0, 6)]
                },
            );
        }
        Def::JackInTheBox => return Some(vec![(0, 0), (31, 0), (31, 31), (0, 31)]), // 272c:295d
        Def::ElectricalOutlet => return Some(vec![(21, 0), (71, 0), (71, 31), (21, 31)]), // 272c:1dfb
        Def::CatPoweredDrive => {
            return Some(vec![
                (0, 0),
                (object.width as u8, 0),
                (object.width as u8, object.height as u8),
                (0, object.height as u8),
            ]);
        } // 272c:2ee1
        Def::BirdCage => {
            return Some(vec![
                (0, 24),
                (19, 0),
                (25, 0),
                (45, 24),
                (45, 63),
                (43, 63),
                (43, 26),
                (34, 16),
                (11, 16),
                (2, 26),
                (2, 63),
                (0, 63),
            ]);
        } // 272c:0f70
        _ => {}
    }
    if object.definition == Def::Bellows {
        // 272c:0371, all three compression states and both facings.
        return tables::selected(
            16,
            object.state,
            object
                .render_flags
                .contains(crate::machine::RenderFlag::FlipHorizontal),
        )
        .map(|p| p.to_vec());
    }
    if object.definition == Def::Trampoline {
        // 272c:3f72.
        return Some(vec![(0, 11), (47, 11), (47, 27), (0, 27)]);
    }
    if object.definition == Def::BoxingGlove {
        let points = match (
            object
                .render_flags
                .contains(crate::machine::RenderFlag::FlipHorizontal),
            object.state == 0,
        ) {
            (false, true) => [(0, 12), (16, 0), (47, 5), (47, 20), (26, 21), (9, 27)],
            (false, false) => [(5, 21), (17, 10), (47, 5), (47, 20), (26, 21), (9, 27)],
            (true, true) => [(38, 27), (21, 21), (0, 20), (0, 5), (31, 0), (47, 12)],
            (true, false) => [(38, 27), (21, 21), (0, 20), (0, 5), (30, 10), (43, 21)],
        };
        return Some(points.to_vec());
    }
    if object.definition == Def::Candle {
        // 272c:0950; fixed three-point base even while the flame animates.
        return Some(vec![(8, 31), (14, 22), (21, 31)]);
    }
    if object.definition == Def::Flashlight {
        // 272c:1d28, six vertices from 3d3c:32fc / 3308.
        let normal = [(0, 4), (23, 4), (31, 0), (31, 16), (23, 12), (0, 12)];
        let flipped = [(8, 4), (31, 4), (31, 12), (8, 12), (0, 16), (0, 0)];
        return Some(
            if !object
                .render_flags
                .contains(crate::machine::RenderFlag::FlipHorizontal)
            {
                normal
            } else {
                flipped
            }
            .to_vec(),
        );
    }
    if object.definition == Def::Cannon {
        // Cannon contact outline from initializer 272c:0b88. The facing bit
        // selects one of two mirrored muzzle/body polygons.
        let normal = [
            (9, 37),
            (18, 6),
            (60, 0),
            (63, 20),
            (47, 30),
            (47, 39),
            (34, 51),
            (22, 51),
        ];
        let flipped = [
            (0, 20),
            (3, 0),
            (45, 6),
            (54, 37),
            (41, 51),
            (29, 51),
            (16, 39),
            (16, 30),
        ];
        return Some(
            if object
                .render_flags
                .contains(crate::machine::RenderFlag::FlipHorizontal)
            {
                flipped
            } else {
                normal
            }
            .to_vec(),
        );
    }
    if object.definition == Def::Light {
        // Definition 29 has zero vertices (DS:0ec4 + 29*0x3a).
        // create_light (1dff:68f0) allocates no outline; 272c:2b58 only
        // sets the rope anchor. 1000:0687 skips the null outline pointer.
        return None;
    }
    if object.definition == Def::CannonProjectile {
        // build_cannon_projectile_outline, 272c:0065, eight native vertices.
        return Some(vec![
            (7, 0),
            (15, 0),
            (22, 8),
            (22, 15),
            (14, 22),
            (8, 22),
            (0, 15),
            (0, 8),
        ]);
    }
    if object.definition == Def::Fish {
        if object.state >= 11 {
            return Some(vec![(8, 47), (24, 44), (39, 47)]);
        }
        // Fish contact outline 272c:1be9, table 3d3c:32dc.
        return Some(vec![
            (0, 18),
            (11, 0),
            (37, 0),
            (47, 18),
            (47, 35),
            (39, 47),
            (8, 47),
            (0, 34),
        ]);
    }
    if object.definition == Def::Fan {
        // Fan collision initializer 272c:1a32 / 1bbd.
        let normal = [(0, 11), (22, 0), (31, 14), (23, 31), (3, 31)];
        let flipped = [(0, 14), (9, 0), (31, 11), (28, 31), (8, 31)];
        return Some(
            if object
                .render_flags
                .contains(crate::machine::RenderFlag::FlipHorizontal)
            {
                flipped
            } else {
                normal
            }
            .to_vec(),
        );
    }
    if object.definition == Def::Scissors {
        // build_scissors_outline_from_state, 272c:389b: state and facing tables.
        return tables::selected(
            37,
            object.state,
            object
                .render_flags
                .contains(crate::machine::RenderFlag::FlipHorizontal),
        )
        .map(|p| p.to_vec());
    }

    if object.definition == Def::Detonator {
        // build_detonator_outline_from_state, Ghidra 272c:3294.
        const NORMAL: [[(u8, u8); 4]; 3] = [
            [(103, 0), (134, 0), (127, 47), (111, 47)],
            [(103, 5), (134, 5), (127, 47), (111, 47)],
            [(103, 10), (134, 10), (127, 47), (111, 47)],
        ];
        const FLIPPED: [[(u8, u8); 4]; 3] = [
            [(0, 0), (31, 0), (24, 47), (8, 47)],
            [(0, 5), (31, 5), (24, 47), (8, 47)],
            [(0, 10), (31, 10), (24, 47), (8, 47)],
        ];
        let table = if !object
            .render_flags
            .contains(crate::machine::RenderFlag::FlipHorizontal)
        {
            &NORMAL
        } else {
            &FLIPPED
        };
        return Some(table[usize::from(object.state.min(2))].to_vec());
    }

    if object.definition == Def::Windmill {
        // Windmill triangle collision outline 272c:496f.
        return Some(vec![(8, 47), (18, 17), (28, 47)]);
    }
    // Cat collision initializer 272c:0c1c uses five vertices selected by flip.
    if object.definition == Def::Cat {
        let normal = [(0, 7), (10, 0), (36, 26), (36, 37), (10, 40)];
        let flipped = [(3, 26), (29, 0), (39, 10), (29, 40), (3, 37)];
        return Some(
            if object
                .render_flags
                .contains(crate::machine::RenderFlag::FlipHorizontal)
            {
                flipped
            } else {
                normal
            }
            .to_vec(),
        );
    }
    if object.definition == Def::Baseball || object.definition == Def::TennisBall {
        // 272c:00c9, shared by definitions 28 and 44.
        return Some(vec![
            (3, 0),
            (11, 0),
            (14, 4),
            (14, 10),
            (11, 14),
            (3, 14),
            (0, 10),
            (0, 4),
        ]);
    }
    if object.definition == Def::Mouse {
        // 272c:346f.
        return Some(vec![(0, 6), (12, 0), (23, 6), (23, 10), (0, 10)]);
    }
    if object.definition == Def::TitleThe || object.definition == Def::CreditsProducer {
        // 272c:1105: title-text rectangles. Definition 55 is trimmed to x=84;
        // definition 57 uses special extents in states 0 and 8.
        let mut right = (object.width as u8).wrapping_sub(1);
        let mut top = 0;
        let mut bottom = (object.height as u8).wrapping_sub(1);
        if object.definition == Def::TitleThe {
            right = b'T';
        } else if object.state == 8 {
            right = b'i';
            top = 10;
        } else if object.state == 0 {
            bottom = 1;
        }
        return Some(vec![(0, top), (right, top), (right, bottom), (0, bottom)]);
    }
    if object.definition == Def::Motor {
        // 272c:24d0: full saved rectangle, inclusive of width/height.
        let x = object.width.min(255) as u8;
        let y = object.height.min(255) as u8;
        return Some(vec![(0, 0), (x, 0), (x, y), (0, y)]);
    }
    if object.definition == Def::TitleSign {
        // 272c:10b6: title-sign state 0 uses 3d3c:3274 (whole sign with the
        // "THE" slot); every other sign piece uses 3d3c:3282.
        return Some(if object.state == 0 {
            vec![
                (25, 0),
                (25, 60),
                (114, 60),
                (114, 0),
                (248, 0),
                (248, 182),
                (0, 182),
            ]
        } else {
            vec![
                (240, 0),
                (248, 0),
                (248, 16),
                (246, 20),
                (244, 20),
                (242, 16),
                (240, 16),
            ]
        });
    }
    if let Some(points) = round::round(object.definition.id(), object.state) {
        return Some(points.to_vec());
    }
    if object.definition == Def::BowlingBall
        || object.definition == Def::Basketball
        || object.definition == Def::Gear
    {
        return Some(vec![
            (8, 0),
            (23, 0),
            (31, 8),
            (31, 23),
            (23, 31),
            (8, 31),
            (0, 23),
            (0, 8),
        ]);
    }
    if tables::rectangle(object.definition.id()) {
        // Outline builders read the active collision extent (+0x44/+0x46),
        // which may be the saved alternate size for a resized fixed part.
        let (width, height) = object
            .extent_override
            .unwrap_or_else(|| crate::spatial::native_object_extents(object));
        let x = (width as u8).wrapping_sub(1);
        let y = (height as u8).wrapping_sub(1);
        return Some(vec![(0, 0), (x, 0), (x, y), (0, y)]);
    }
    tables::selected(
        object.definition.id(),
        object.state,
        object
            .render_flags
            .contains(crate::machine::RenderFlag::FlipHorizontal),
    )
    .map(|points| points.to_vec())
}

/// Convex pieces of an outline for separating-axis tests. Native outlines can
/// be concave; testing their hull would create invisible contacts.
pub fn convex_parts(object: &Object) -> Vec<Vec<(u8, u8)>> {
    let Some(outline) = object.outline.as_ref() else {
        return Vec::new();
    };
    match object.definition {
        // The lens is wider than the body. Treating the whole concave torch
        // as convex invents a flat surface over its sloping shoulder.
        Def::Flashlight if outline.len() == 6 => vec![
            vec![outline[0], outline[1], outline[2], outline[3]],
            vec![outline[0], outline[3], outline[4], outline[5]],
        ],
        // Only the beam of the seesaw is convex; the pedestal is ignored.
        Def::Seesaw if outline.len() == 8 => {
            vec![vec![outline[0], outline[1], outline[2], outline[7]]]
        }
        // Title sign: left wedge, base below the slot, and right block.
        Def::TitleSign if outline.len() == 7 && object.state == 0 => vec![
            vec![outline[0], outline[1], outline[6]],
            vec![outline[1], outline[2], outline[5], outline[6]],
            vec![outline[2], outline[3], outline[4], outline[5]],
        ],
        _ => vec![outline.clone()],
    }
}

#[path = "tables/angle_table.rs"]
mod angles;
#[path = "tables/round_outlines.rs"]
mod round;
/// Original angle convention: downward=0, right=-0x4000, upward=-0x8000.
/// This bounded helper covers the byte-coordinate deltas used by outlines.
pub fn vector_angle(dx: i16, dy: i16) -> u16 {
    let x = i32::from(dx).unsigned_abs();
    let y = i32::from(dy).unsigned_abs();
    let mut angle = if x == y {
        if x == 0 { 0x400 } else { 0x200 }
    } else if x > y {
        angles::ATAN[((y << 9) / x) as usize] as i32
    } else {
        0x400 - angles::ATAN[((x << 9) / y) as usize] as i32
    };
    if dx < 0 {
        angle = 0x800 - angle;
    }
    if dy < 0 {
        angle = 0x1000 - angle;
    }
    ((angle - 0x400) << 4) as u16
}
/// Direction stored at each vertex, including the closing edge.
pub fn edge_angles(points: &[(u8, u8)]) -> Vec<u16> {
    if points.is_empty() {
        return Vec::new();
    }
    points
        .iter()
        .zip(points.iter().cycle().skip(1))
        .map(|(&(x, y), &(nx, ny))| {
            let dx = i16::from(nx) - i16::from(x);
            let dy = i16::from(ny) - i16::from(y);
            let direction = vector_angle(dx + dx.signum(), dy + dy.signum());
            0xc000_u16.wrapping_sub(direction)
        })
        .collect()
}

/// Native 272c:40f0 reads dedicated anchors, independent of beam vertices.
pub fn seesaw_anchors(state: u16) -> Option<[(u8, u8); 2]> {
    tables::seesaw_anchors(state)
}
