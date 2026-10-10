use super::*;
use crate::sound::SoundEffect;

const FRAME_BLAST: u16 = 2;
const FRAME_LAST: u16 = 5;
const BLAST_HORIZONTAL_RANGE: (i16, i16) = (-20, 20);
const BLAST_VERTICAL_RANGE: (i16, i16) = (-24, 24);
const WALL_TILE_OFFSET: i16 = 8;
const WALL_TILE_ALIGNMENT_MASK: i16 = 15;
const WALL_CUT_BEFORE_PIXELS: i16 = 32;
const WALL_CUT_AFTER_PIXELS: i16 = 24;

/// PART41.BMP frame sizes; load_object_anchor_offsets (1000:5c77), called by
/// update_object_anchor_offsets (1000:5be4), copies the current frame size.
pub(super) const EXPLOSION_EXTENTS: [(u16, u16); 6] =
    [(72, 61), (80, 83), (80, 76), (48, 43), (32, 30), (40, 25)];

pub(super) fn spawn_explosion_at(objects: &mut Vec<Object>, x: i16, y: i16) {
    crate::sound::play_effect(SoundEffect::Explosion);
    let explosion = Object {
        contact: None,
        list: ObjectList::Fixed,
        update_flags: UpdateFlag::CreatedDuringSimulation.mask(),
        definition: Def::Explosion,
        // The native allocator zeroes +0x3a for a fixed explosion record.
        effective_mass: 0,
        collision_impulse: 0,
        motion: crate::motion::Motion::at_pixels(x, y),
        saved_position: (x, y),
        previous_position: (x, y),
        older_position: (x, y),
        saved_state: 0,
        outline: None,
        outline_angles: Vec::new(),
        state: 0,
        previous_state: 0,
        older_state: 0,
        power_state: 0,
        callback_timer: 0,
        pending_light_power: 0,
        activation_timer: 0,
        belt_anchor: (0, 0),
        belt_size: 0,
        rope_anchors: [(0, 0); 2],
        belt: None,
        ropes: [None, None],
        linked_objects: [None; 6],
        next_part: None,
        pulley_connection: None,
        flags: PropertyFlag::FreePlacement.mask(),
        render_flags: RenderFlags::EMPTY,
        x,
        y,
        width: 0,
        height: 0,
        extent_override: None,
        alt_size: (0, 0),
    };
    let mut explosion = explosion;
    set_explosion_extents(&mut explosion);
    objects.push(explosion);
}

pub(super) fn set_explosion_extents(explosion: &mut Object) {
    if let Some(&(width, height)) = EXPLOSION_EXTENTS.get(explosion.state as usize) {
        explosion.width = width;
        explosion.height = height;
    }
}

/// update_explosion_expand, 272c:1649. Blast response happens only at frame
/// 2. Fixed parts dispatch through CS:1738 (ids 1, 6, 15, 48); every handler
/// continues the scan. Moving parts are popped, chained, or thrown.
pub(super) fn update_explosion_expand(objects: &mut Vec<Object>, index: usize) {
    if objects[index].state == FRAME_LAST {
        objects[index]
            .render_flags
            .insert(crate::machine::RenderFlag::Hidden);
    } else {
        objects[index].state = objects[index].state.wrapping_add(1);
        // The native update advances the sprite state without changing the
        // object's persistent width/height fields. Collision queries derive
        // frame bounds separately through native_object_extents().
    }
    if objects[index].state != FRAME_BLAST {
        return;
    }
    for contact in
        crate::spatial::nearby_parts(objects, index, BLAST_HORIZONTAL_RANGE, BLAST_VERTICAL_RANGE)
    {
        let target = contact.index;
        if objects[target].update_flags & UpdateFlag::ActiveMoving.mask() == 0 {
            match objects[target].definition {
                Def::BrickWall | Def::WoodWall => {
                    trim_wall_around_explosion(objects, target, index)
                }
                Def::CatPoweredDrive => activate_contacted_object(&mut objects[target]),
                Def::Fish => make_fish_jump(&mut objects[target]),
                _ => {}
            }
            continue;
        }
        match objects[target].definition {
            Def::Balloon => objects[target].power_state = 1,
            Def::Dynamite => detonate_dynamite(objects, target),
            _ => {
                let (vx, vy) = crate::blast::impulse(&objects[index], &objects[target]);
                objects[target].motion.velocity_x = vx;
                objects[target].motion.velocity_y = vy;
            }
        }
    }
}

/// trim_rectangle_outline_around_object, 272c:17bc. Cuts a 16-pixel-aligned
/// hole (tile grid offset by 8) around the explosion centre along the wall's
/// long axis: splits the wall in two, shortens one end, or hides it entirely.
pub(super) fn trim_wall_around_explosion(objects: &mut Vec<Object>, wall: usize, explosion: usize) {
    let (width, height) = crate::spatial::native_object_extents(&objects[explosion]);
    let centre_x = objects[explosion].x.wrapping_add((width >> 1) as i16);
    let centre_y = objects[explosion].y.wrapping_add((height >> 1) as i16);
    let original_extents = crate::spatial::native_object_extents(&objects[wall]);
    let horizontal = original_extents.1 < original_extents.0;
    let centre = if horizontal { centre_x } else { centre_y };
    let near = (centre.wrapping_sub(WALL_CUT_BEFORE_PIXELS) & !WALL_TILE_ALIGNMENT_MASK)
        .wrapping_add(WALL_TILE_OFFSET);
    let far = (centre.wrapping_add(WALL_CUT_AFTER_PIXELS) & !WALL_TILE_ALIGNMENT_MASK)
        .wrapping_add(WALL_TILE_OFFSET);
    let part = &objects[wall];
    let (start, length) = if horizontal {
        (part.x, original_extents.0)
    } else {
        (part.y, original_extents.1)
    };
    let end = start.wrapping_add(length as i16);
    let set = |part: &mut Object, start: i16, length: i16| {
        if horizontal {
            part.x = start;
        } else {
            part.y = start;
        }
        part.extent_override = Some(if horizontal {
            (length as u16, original_extents.1)
        } else {
            (original_extents.0, length as u16)
        });
        update_object_outline(part);
    };
    if start < near {
        if far < end {
            let mut piece = objects[wall].clone();
            piece.update_flags |= UpdateFlag::CreatedDuringSimulation.mask();
            // create_object_from_template allocates the second wall record
            // with zeroed motion state; the following native writes set only
            // its pixel and anchor coordinates.
            piece.motion.position_x = 0;
            piece.motion.position_y = 0;
            piece.motion.velocity_x = 0;
            piece.motion.velocity_y = 0;
            piece.effective_mass = 0;
            set(&mut piece, far, end.wrapping_sub(far));
            objects.push(piece);
            set(&mut objects[wall], start, near.wrapping_sub(start));
        } else if near < end {
            set(&mut objects[wall], start, near.wrapping_sub(start));
        } else {
            update_object_outline(&mut objects[wall]);
        }
    } else if far < end {
        if start < far {
            set(&mut objects[wall], far, end.wrapping_sub(far));
        }
    } else if start < far && near < end {
        objects[wall]
            .render_flags
            .insert(crate::machine::RenderFlag::Hidden);
    }
}
