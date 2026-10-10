use super::*;

pub(crate) fn belt_segments(a: &Object, b: &Object) -> [((i32, i32), (i32, i32)); 2] {
    let p = a.belt_origin();
    let q = b.belt_origin();
    let (ra, rb) = (a.belt_size as i32, b.belt_size as i32);
    if (p.0 - q.0).abs() < (p.1 - q.1).abs() {
        [
            ((p.0, p.1 + (ra >> 1)), (q.0, q.1 + (rb >> 1))),
            ((p.0 + ra, p.1 + (ra >> 1)), (q.0 + rb, q.1 + (rb >> 1))),
        ]
    } else {
        [
            ((p.0 + (ra >> 1), p.1), (q.0 + (rb >> 1), q.1)),
            ((p.0 + (ra >> 1), p.1 + ra), (q.0 + (rb >> 1), q.1 + rb)),
        ]
    }
}
pub(super) fn draw_connection(
    dst: &mut [u8],
    mut a: (i32, i32),
    mut b: (i32, i32),
    preview: Option<i32>,
    color: [u8; 4],
) {
    let clip = if let Some(scale) = preview {
        a = (
            272 + ((a.0 + 8) * scale >> 10),
            72 + ((a.1 + 8) * scale >> 10),
        );
        b = (
            272 + ((b.0 + 8) * scale >> 10),
            72 + ((b.1 + 8) * scale >> 10),
        );
        (272, 72, 528, 232)
    } else {
        (0, 0, 640, 400)
    };
    // TIM's 42df:0998 VGA callback groups pixels along the major axis. It
    // draws a run, steps once along the minor axis, then uses a 16-bit
    // fractional accumulator to distribute the remaining run lengths. This
    // differs from a symmetric Bresenham loop at tie positions.
    let mut draw_pixel = |x: i32, y: i32| {
        if x >= clip.0 && x < clip.2 && y >= clip.1 && y < clip.3 {
            let offset = (y as usize * 640 + x as usize) * 4;
            dst[offset..offset + 4].copy_from_slice(&color);
        }
    };
    let dx = (b.0 - a.0).abs();
    let dy = (b.1 - a.1).abs();
    let sy = if a.1 <= b.1 { 1 } else { -1 };

    if dx == 0 {
        for y in 0..=dy {
            draw_pixel(a.0, a.1 + y * sy);
        }
    } else if dy == 0 {
        let sx = if a.0 <= b.0 { 1 } else { -1 };
        for x in 0..=dx {
            draw_pixel(a.0 + x * sx, a.1);
        }
    } else {
        // The DOS routine receives belt points in increasing X order. Keep
        // that ordering for the same endpoint tie behavior when a caller
        // supplies the reverse direction.
        if a.0 > b.0 {
            std::mem::swap(&mut a, &mut b);
        }
        let dx = b.0 - a.0;
        let dy = (b.1 - a.1).abs();
        let sy = if a.1 < b.1 { 1 } else { -1 };
        let (mut x, mut y) = a;
        draw_pixel(x, y);

        // The callback has a dedicated 45-degree path that alternates X and Y
        // steps, including both pixels at each corner of the staircase.
        if dx == dy {
            for _ in 0..dx {
                x += 1;
                draw_pixel(x, y);
                y += sy;
                draw_pixel(x, y);
            }
            return;
        }

        let x_major = dx > dy;
        let (major_delta, minor_delta) = if x_major { (dx, dy) } else { (dy, dx) };
        let groups = minor_delta + 1;
        let quotient = major_delta / groups;
        let remainder = major_delta % groups;
        let fraction = (((remainder as u64) << 16) / groups as u64) as u16;
        let mut fraction_accumulator = 0u16;
        let mut remaining = major_delta;
        let mut run = quotient + 1;
        loop {
            let run_length = run.min(remaining);
            for _ in 0..run_length {
                if x_major {
                    x += 1;
                } else {
                    y += sy;
                }
                draw_pixel(x, y);
            }
            remaining -= run_length;
            if remaining == 0 {
                break;
            }
            if x_major {
                y += sy;
            } else {
                x += 1;
            }
            draw_pixel(x, y);
            fraction_accumulator = fraction_accumulator.wrapping_add(fraction);
            run = quotient + i32::from(fraction_accumulator < fraction);
        }
    }
}

/// A directed rope span, retaining graph slots for splitting at scissors.
/// compute_object_anchor_coordinates (1000:5be4): where the current frame
/// is drawn relative to the part, from the per-state offset table and, when
/// mirrored, the frame's extent (load_object_anchor_offsets, 1000:5c77).
pub fn draw_offset(object: &Object) -> (i32, i32) {
    use crate::definition::PartDefinition as Def;
    if matches!(object.definition, Def::Belt | Def::Rope | Def::RopeEnd)
        || !render_tables::has_anchor_table(object.definition.id())
    {
        // 1000:5be4 first copies the body position and only enters its
        // offset/flip branch when the native definition's table is non-null.
        return (0, 0);
    }
    let (ox, oy) = render_tables::anchor(object.definition.id(), object.state);
    // Native 1000:5c77 copies the live +0x44/+0x46 frame extents here.
    // These can cover multiple sprite layers (Gun state 5 is 128x43 even
    // though its first rendered layer is 64x31), so the sprite-frame list
    // alone is not sufficient for mirrored anchor placement.
    let (ex, ey) = crate::spatial::native_object_extents(object);
    let (ex, ey) = (i32::from(ex), i32::from(ey));
    let x = if !object.render_flags.contains(RenderFlag::FlipHorizontal) {
        ox
    } else {
        i32::from(object.width) - ox - ex
    };
    let y = if !object.render_flags.contains(RenderFlag::FlipVertical) {
        oy
    } else {
        i32::from(object.height) - oy - ey
    };
    (x, y)
}

/// A rope end's point (rebuild_rope_endpoint_coordinates, 1000:4f7f): the
/// drawn frame's origin plus the local anchor, so a tilted seesaw or a
/// swinging balloon carries its knot. Pulleys use their plain position.
pub fn rope_point(object: &Object, slot: usize) -> (i32, i32) {
    let (ax, ay) = object.rope_anchors[slot];
    let (dx, dy) = if object.definition == crate::definition::PartDefinition::Pulley {
        (0, 0)
    } else {
        draw_offset(object)
    };
    (
        i32::from(object.x) + dx + i32::from(ax),
        i32::from(object.y) + dy + i32::from(ay),
    )
}

pub(crate) fn rope_spans(
    objects: &[Object],
    rope: &RopeConnection,
) -> Result<Vec<RopeSpan>, String> {
    let [Some(mut current), Some(end)] = rope.endpoints else {
        return Ok(Vec::new());
    };
    let slot = rope.attachment_slots[0] as usize;
    let end_slot = rope.attachment_slots[1] as usize;
    let mut next = objects
        .get(current)
        .ok_or("Invalid rope endpoint")?
        .linked_objects[slot]
        .unwrap_or(end);
    let mut spans = Vec::new();
    let mut visited = std::collections::HashSet::new();
    loop {
        if !visited.insert(current) {
            return Err("Cyclic rope pulley route".into());
        }
        let a = objects.get(current).ok_or("Invalid rope route source")?;
        let b = objects.get(next).ok_or("Invalid rope route target")?;
        let a_slot = if a.definition == PartDefinition::Pulley {
            1
        } else {
            slot
        };
        let b_slot = if b.definition == PartDefinition::Pulley {
            0
        } else {
            end_slot
        };
        let anchor = rope_point;
        spans.push(RopeSpan {
            from: current,
            from_slot: a_slot,
            to: next,
            to_slot: b_slot,
            start: anchor(a, a_slot),
            end: anchor(b, b_slot),
        });
        if b.definition != PartDefinition::Pulley {
            break;
        }
        current = next;
        let Some(link) = b.linked_objects[0] else {
            break;
        };
        next = link;
    }
    Ok(spans)
}
pub(crate) fn rope_segments(
    objects: &[Object],
    rope: &RopeConnection,
) -> Result<Vec<((i32, i32), (i32, i32))>, String> {
    Ok(rope_spans(objects, rope)?
        .into_iter()
        .map(|span| (span.start, span.end))
        .collect())
}

/// compute_linked_point_distance (1000:6f8e): native integer octagonal metric.
pub(crate) fn linked_point_distance(a: (i32, i32), b: (i32, i32)) -> i32 {
    let dx = (b.0 - a.0).abs();
    let dy = (b.1 - a.1).abs();
    let minor = dx.min(dy);
    dx.max(dy) + (minor >> 2) + (minor >> 3)
}

/// draw_rope_segment / draw_curved_line (24de:1d59 / 24de:1b9d).
/// The native quadratic uses 16 integer steps and a control point below
/// the chord midpoint by the slack. Its visible midpoint drops slack / 2.
pub(super) fn curved_rope_segments(
    a: (i32, i32),
    b: (i32, i32),
    slack: i32,
) -> Vec<((i32, i32), (i32, i32))> {
    if slack < 5 {
        return vec![(a, b)];
    }
    let control = ((a.0 + b.0) >> 1, ((a.1 + b.1) >> 1) + slack);
    let point = |step: i32| {
        let t = 16 - step;
        (
            (t * t * a.0 + 2 * t * step * control.0 + step * step * b.0) >> 8,
            (t * t * a.1 + 2 * t * step * control.1 + step * step * b.1) >> 8,
        )
    };
    (0..16).map(|i| (point(i), point(i + 1))).collect()
}

/// render_rope_object (24de:1dcf), compute_rope_slack (1000:713d).
/// Slack belongs to the end spans; pulley-to-pulley spans stay straight.
/// Keep the physical graph spans separate from these presentation segments.
pub fn rope_render_segments(
    objects: &[Object],
    owner: &Object,
    rope: &RopeConnection,
) -> Result<Vec<((i32, i32), (i32, i32))>, String> {
    let spans = rope_spans(objects, rope)?;
    Ok(spans
        .iter()
        .flat_map(|span| {
            let length = if objects[span.from].definition == PartDefinition::Pulley
                && objects[span.to].definition == PartDefinition::Pulley
            {
                0
            } else if Some(span.from) == rope.endpoints[0] {
                i32::from(owner.activation_timer)
            } else if Some(span.to) == rope.endpoints[1] {
                i32::from(owner.callback_timer)
            } else {
                0
            };
            curved_rope_segments(
                span.start,
                span.end,
                length - linked_point_distance(span.start, span.end),
            )
        })
        .collect())
}

pub(super) fn fill_triangle(rgba: &mut [u8], vertices: [(i32, i32); 3], color: [u8; 4]) {
    let cross = |a: (i32, i32), b: (i32, i32), p: (i32, i32)| {
        i64::from(b.0 - a.0) * i64::from(p.1 - a.1) - i64::from(b.1 - a.1) * i64::from(p.0 - a.0)
    };
    if cross(vertices[0], vertices[1], vertices[2]) == 0 {
        return;
    }
    let min_x = vertices.iter().map(|p| p.0).min().unwrap().max(0);
    let max_x = vertices.iter().map(|p| p.0).max().unwrap().min(639);
    let min_y = vertices.iter().map(|p| p.1).min().unwrap().max(0);
    let max_y = vertices.iter().map(|p| p.1).max().unwrap().min(399);
    for y in min_y..=max_y {
        for x in min_x..=max_x {
            let signs = [
                cross(vertices[0], vertices[1], (x, y)),
                cross(vertices[1], vertices[2], (x, y)),
                cross(vertices[2], vertices[0], (x, y)),
            ];
            if signs.iter().all(|v| *v >= 0) || signs.iter().all(|v| *v <= 0) {
                let at = (y as usize * 640 + x as usize) * 4;
                rgba[at..at + 4].copy_from_slice(&color);
            }
        }
    }
}

/// Positions before a simulation update, for presentation interpolation.
pub fn positions(objects: &[Object]) -> Vec<(i16, i16)> {
    objects.iter().map(|object| (object.x, object.y)).collect()
}

/// Blend ordinary motion for drawing while leaving native-sized jumps intact.
pub fn blend_positions(objects: &[Object], before: &[(i16, i16)], alpha: f32) -> Vec<Object> {
    const MAX_STEP: i16 = 48;
    let mut out = objects.to_vec();
    if before.len() != objects.len() {
        return out;
    }
    let alpha = alpha.clamp(0.0, 1.0);
    for (object, &(x, y)) in out.iter_mut().zip(before) {
        let (dx, dy) = (object.x - x, object.y - y);
        if (dx, dy) == (0, 0) || dx.abs() > MAX_STEP || dy.abs() > MAX_STEP {
            continue;
        }
        if !(object.definition == PartDefinition::Cat && object.state == 2 && dx.abs() == 32) {
            object.x = x + (f32::from(dx) * alpha).round() as i16;
        }
        object.y = y + (f32::from(dy) * alpha).round() as i16;
    }
    out
}
