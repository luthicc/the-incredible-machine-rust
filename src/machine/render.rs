use super::*;

/// Title composition with native sprite sizes, flips, and original platform tiles.
/// Composite sprite tables follow 24de:1fd1. Animation remains pending.
use super::render_tables;
pub fn title_screen(resources: &Resources) -> Result<crate::resources::Screen, String> {
    title_frame(resources, 0)
}

/// Fish callback 272c:1c5f advances states 0..10 and wraps at 11.
pub fn title_frame(resources: &Resources, tick: u16) -> Result<crate::resources::Screen, String> {
    collection_frame(resources, "TITLE.GKC", tick)
}
pub fn collection_frame(
    resources: &Resources,
    name: &str,
    tick: u16,
) -> Result<crate::resources::Screen, String> {
    render_collection(resources, name, tick, None, None)
}

/// Original preview scales positions by its environment-dependent factor and
/// sprite dimensions by 0x200, with a 10-bit fractional shift (24de:1fd1).
pub fn level_preview(
    resources: &Resources,
    name: &str,
) -> Result<crate::resources::Screen, String> {
    let header = collection_metadata(resources, name)?;
    let (x, y) = header.scroll_limits.ok_or("Preview requires a level")?;
    let denominator = i32::from(x.max(y)) + 560;
    if denominator <= 0 {
        return Err("Invalid preview extent".into());
    }
    render_collection(resources, name, 0, Some(0x40000 / denominator), None)
}
/// Puzzles and freeform saves share world coordinates and have no title
/// corners; the attract machines are drawn offset.
pub(super) fn is_player_machine(name: &str) -> bool {
    name.ends_with(".LEV") || name.ends_with(".TIM")
}

/// The launch panel's preview of a machine being edited
/// (draw_object_preview_list), scaled as level_preview scales a level.
pub fn machine_preview(
    resources: &Resources,
    name: &str,
    objects: &[Object],
) -> Result<crate::resources::Screen, String> {
    render_collection(resources, name, 0, Some(0x40000 / (560 - 8)), Some(objects))
}
/// Render persistent simulation objects without reloading their saved positions.
pub fn live_collection_frame(
    resources: &Resources,
    name: &str,
    tick: u16,
    objects: &[Object],
) -> Result<crate::resources::Screen, String> {
    render_collection(resources, name, tick, None, Some(objects))
}
/// Render live objects with the optional ignition ray disabled. The machine
/// is cloned because the ray is a presentation overlay driven by object link 4.
pub fn live_collection_frame_with_magnifier_ray(
    resources: &Resources,
    name: &str,
    tick: u16,
    objects: &[Object],
    show_ray: bool,
) -> Result<crate::resources::Screen, String> {
    if show_ray {
        return live_collection_frame(resources, name, tick, objects);
    }
    let mut copy = objects.to_vec();
    for object in &mut copy {
        if object.definition == PartDefinition::MagnifyingGlass {
            object.linked_objects[4] = None;
        }
    }
    live_collection_frame(resources, name, tick, &copy)
}
pub(super) fn render_collection(
    resources: &Resources,
    name: &str,
    tick: u16,
    preview: Option<i32>,
    live_objects: Option<&[Object]>,
) -> Result<crate::resources::Screen, String> {
    // draw_game_screen (24de:0c4f) and the title (1dff:079a) clear with the
    // fill colour DS:52cb, which initialize_game_system sets to index 3.
    let background = crate::resources::palette_color(resources, "TIM.PAL", 3)?;
    let mut rgba = vec![0u8; 640 * 480 * 4];
    for (index, pixel) in rgba.chunks_exact_mut(4).enumerate() {
        pixel.copy_from_slice(if index / 640 < 400 {
            &background
        } else {
            &[0, 0, 0, 255]
        });
    }
    let mut objects = match live_objects {
        Some(objects) => objects.to_vec(),
        None => collection_objects(resources, name)?,
    };
    for object in &mut objects {
        // run_intro_and_menu_sequence (1dff:079a) sets a scene-specific DOS
        // display origin. The active title page begins 8 pixels in; credits
        // begins 12 pixels in. The origin does not add a vertical offset.
        object.x = object.x.wrapping_add(if name == "CREDITS.GKC" {
            12
        } else if name == "TITLE.GKC" {
            8
        } else {
            0
        });
        if live_objects.is_none() && object.definition == PartDefinition::Fish {
            object.state = tick % 11;
        }
    }
    let knots = if preview.is_none()
        && objects
            .iter()
            .any(|object| object.definition == PartDefinition::Rope)
    {
        let mut frames = crate::resources::sprites(resources, "GP_BORD.BMP", "TIM.PAL")?;
        if frames.len() <= 36 {
            return Err("Missing rope knot sprite".into());
        }
        Some(frames.swap_remove(36))
    } else {
        None
    };
    for layer in (0..6).rev() {
        for object in &objects {
            // Keep graph indices intact: spawned parts can follow inventory.
            if object.list == ObjectList::Inventory
                || object.render_flags.contains(RenderFlag::Hidden)
            {
                continue;
            }
            if object.definition == PartDefinition::Belt && render_tables::layer(8) == layer {
                if let Some([Some(a), Some(b)]) = object.belt {
                    if let (Some(a), Some(b)) = (objects.get(a), objects.get(b)) {
                        for (start, end) in belt_segments(a, b) {
                            draw_connection(&mut rgba, start, end, preview, [0, 0, 0, 255]);
                        }
                    }
                }
                continue;
            }
            if object.definition == PartDefinition::Rope && render_tables::layer(10) == layer {
                let rope_color = crate::resources::palette_color(resources, "TIM.PAL", 6)?;
                for rope in object.ropes.iter().flatten() {
                    for (start, end) in rope_render_segments(&objects, object, rope)? {
                        draw_connection(&mut rgba, start, end, preview, rope_color);
                    }
                    // 24de:1dcf adds frame 36 only at ordinary endpoints,
                    // excluding pulley (7) and invisible attachment (49).
                    if let Some(knot) = &knots {
                        for side in 0..2 {
                            if let Some(endpoint) =
                                rope.endpoints[side].and_then(|i| objects.get(i))
                            {
                                if endpoint.definition == PartDefinition::Pulley
                                    || endpoint.definition == PartDefinition::RopeEnd
                                {
                                    continue;
                                }
                                let slot = rope.attachment_slots[side] as usize;
                                let (x, y) = rope_point(endpoint, slot);
                                blit(&mut rgba, knot, x - 5, y - 2, false, false, None);
                            }
                        }
                    }
                }
                continue;
            }
            if (object.width == 0 || object.height == 0)
                && object.definition != PartDefinition::Explosion
            {
                continue;
            }
            let frames = crate::resources::sprites(
                resources,
                &format!("PART{}.BMP", object.definition.id()),
                "TIM.PAL",
            )?;
            if !object.render_flags.contains(RenderFlag::CompositeSprite)
                && render_tables::layer(object.definition.id()) != layer
            {
                continue;
            }
            if object.update_flags & UpdateFlag::UseAlternateSize.mask() != 0 {
                // render_objects_by_layer (24de:197e) visits each part only
                // from its spatial bucket for the current layer. Tile parts
                // have the same layer ownership; rendering them on all six
                // passes lets low-layer tiles cover later sprites.
                if render_tables::layer(object.definition.id()) != layer {
                    continue;
                }
                // 24de:1fd1 repeats 16-pixel tiles. Its interior variant is
                // selected from bit 4 of the initial screen origin, then
                // toggled once for each tile (the first interior tile does
                // not use the tile center).
                // render_object_graphic (24de:1fd1) iterates signed
                // extent_x/extent_y values, not unsigned saved dimensions.
                // Include live extent overrides from native blast wall cuts.
                let (extent_x, extent_y) = crate::spatial::native_object_extents(object);
                let columns = ((extent_x as i16) >> 4).max(0) as usize;
                let rows = ((extent_y as i16) >> 4).max(0) as usize;
                for row in 0..rows {
                    for column in 0..columns {
                        let variant = if rows == 1 {
                            if column == 0 {
                                0
                            } else if column + 1 == columns {
                                3
                            } else {
                                1 + ((((object.x as i32 & 0x10) >> 4) + column as i32) & 1) as usize
                            }
                        } else if columns == 1 {
                            if row == 0 {
                                4
                            } else if row + 1 == rows {
                                7
                            } else {
                                5 + ((((object.y as i32 & 0x10) >> 4) + row as i32) & 1) as usize
                            }
                        } else {
                            0
                        };
                        let frame = frames
                            .get(object.state as usize + variant)
                            .ok_or("Missing platform tile")?;
                        blit(
                            &mut rgba,
                            frame,
                            object.x as i32 + column as i32 * 16,
                            object.y as i32 + row as i32 * 16,
                            false,
                            false,
                            preview,
                        );
                    }
                }
            } else if object.render_flags.contains(RenderFlag::CompositeSprite) {
                let pieces = render_tables::composite(object.definition.id(), object.state)
                    .ok_or_else(|| {
                        format!(
                            "Missing composite for part {} state {}",
                            object.definition.id(),
                            object.state
                        )
                    })?;
                for &(piece_layer, index, offset_x, offset_y) in pieces {
                    if piece_layer != layer {
                        continue;
                    }
                    let frame = frames
                        .get(index as usize)
                        .ok_or("Missing composite sprite")?;
                    let flip_x = object.render_flags.contains(RenderFlag::FlipHorizontal);
                    let flip_y = object.render_flags.contains(RenderFlag::FlipVertical);
                    let x = if flip_x {
                        object.width as i32 - offset_x - frame.width as i32
                    } else {
                        offset_x
                    };
                    let y = if flip_y {
                        object.height as i32 - offset_y - frame.height as i32
                    } else {
                        offset_y
                    };
                    blit(
                        &mut rgba,
                        frame,
                        object.x as i32 + x,
                        object.y as i32 + y,
                        flip_x,
                        flip_y,
                        preview,
                    );
                }
            } else {
                let frame = frames.get(object.state as usize).unwrap_or(&frames[0]);
                let (offset_x, offset_y) =
                    render_tables::anchor(object.definition.id(), object.state);
                // render_object_graphic, 24de:1fd1: flipped sprites remain
                // relative to the saved canvas, even when a lit frame grows.
                let offset_x = if object.render_flags.contains(RenderFlag::FlipHorizontal) {
                    i32::from(object.width) - offset_x - frame.width as i32
                } else {
                    offset_x
                };
                let offset_y = if object.render_flags.contains(RenderFlag::FlipVertical) {
                    i32::from(object.height) - offset_y - frame.height as i32
                } else {
                    offset_y
                };
                blit(
                    &mut rgba,
                    frame,
                    object.x as i32 + offset_x,
                    object.y as i32 + offset_y,
                    object.render_flags.contains(RenderFlag::FlipHorizontal),
                    object.render_flags.contains(RenderFlag::FlipVertical),
                    preview,
                );
            }
            if object.definition == PartDefinition::MagnifyingGlass && preview.is_none() {
                if let Some(target) = object.linked_objects[4].and_then(|i| objects.get(i)) {
                    if let Some((ax, ay)) = target.ignition_anchor() {
                        // Native 24de:23d5 follows PartRecord +0x62 and draws
                        // on every render while that live pointer is present.
                        let lens_x = i32::from(object.x)
                            + if !object.render_flags.contains(RenderFlag::FlipHorizontal) {
                                15
                            } else {
                                -1
                            };
                        let vertices = [
                            (lens_x, i32::from(object.y) + 6),
                            (
                                i32::from(target.x) + i32::from(ax),
                                i32::from(target.y) + i32::from(ay),
                            ),
                            (lens_x, i32::from(object.y) + 16),
                        ];
                        fill_triangle(
                            &mut rgba,
                            vertices,
                            crate::resources::palette_color(resources, "TIM.PAL", 14)?,
                        );
                    }
                }
            }
        }
    }
    // DOS redraws an expanding/popping balloon into its dirty region after
    // stationary parts have already been painted. Keep its new pixels above
    // those parts, matching the native incremental update at 24de:197e.
    for object in objects.iter().filter(|object| {
        object.definition == PartDefinition::Balloon && matches!(object.state, 1 | 2)
    }) {
        let frames = crate::resources::sprites(resources, "PART4.BMP", "TIM.PAL")?;
        let frame = frames
            .get(object.state as usize)
            .ok_or("Missing balloon sprite")?;
        let (mut offset_x, mut offset_y) =
            render_tables::anchor(object.definition.id(), object.state);
        let flip_x = object.render_flags.contains(RenderFlag::FlipHorizontal);
        let flip_y = object.render_flags.contains(RenderFlag::FlipVertical);
        if flip_x {
            offset_x = i32::from(object.width) - offset_x - frame.width as i32;
        }
        if flip_y {
            offset_y = i32::from(object.height) - offset_y - frame.height as i32;
        }
        let balloon_bounds = (
            i32::from(object.x) + offset_x,
            i32::from(object.y) + offset_y,
            frame.width as i32,
            frame.height as i32,
        );
        let overlaps_scissors = objects.iter().any(|candidate| {
            if candidate.definition != PartDefinition::Scissors {
                return false;
            }
            let x0 = i32::from(candidate.x);
            let y0 = i32::from(candidate.y);
            balloon_bounds.0 < x0 + i32::from(candidate.width)
                && x0 < balloon_bounds.0 + balloon_bounds.2
                && balloon_bounds.1 < y0 + i32::from(candidate.height)
                && y0 < balloon_bounds.1 + balloon_bounds.3
        });
        if !overlaps_scissors {
            continue;
        }
        blit(
            &mut rgba,
            frame,
            i32::from(object.x) + offset_x,
            i32::from(object.y) + offset_y,
            flip_x,
            flip_y,
            preview,
        );
    }
    if !is_player_machine(name) {
        let corners = crate::resources::sprites(resources, "CORNERS.BMP", "TIM.PAL")?;
        for (frame, (x, y)) in corners.iter().zip([(0, 0), (610, 0), (0, 373), (610, 373)]) {
            blit(&mut rgba, frame, x, y, false, false, preview);
        }
    }
    Ok(crate::resources::Screen {
        width: 640,
        height: 480,
        rgba,
    })
}

pub(super) fn blit(
    destination: &mut [u8],
    frame: &crate::resources::Screen,
    x: i32,
    y: i32,
    flip_x: bool,
    flip_y: bool,
    preview: Option<i32>,
) {
    let (x, y, width, height, clip) = if let Some(scale) = preview {
        (
            272 + ((x + 8) * scale >> 10),
            72 + ((y + 8) * scale >> 10),
            frame.width as usize / 2,
            frame.height as usize / 2,
            (272, 72, 528, 232),
        )
    } else {
        (
            x,
            y,
            frame.width as usize,
            frame.height as usize,
            (0, 0, 640, 400),
        )
    };
    if width == 0 || height == 0 {
        return;
    }
    for py in 0..height {
        for px in 0..width {
            let (dx, dy) = (x + px as i32, y + py as i32);
            if dx < clip.0 || dx >= clip.2 || dy < clip.1 || dy >= clip.3 {
                continue;
            }
            let (px, py) = (
                px * frame.width as usize / width,
                py * frame.height as usize / height,
            );
            let sx = if flip_x {
                frame.width as usize - 1 - px
            } else {
                px
            };
            let sy = if flip_y {
                frame.height as usize - 1 - py
            } else {
                py
            };
            let source = &frame.rgba[(sy * frame.width as usize + sx) * 4..][..4];
            if source[3] != 0 {
                destination[(dy as usize * 640 + dx as usize) * 4..][..4].copy_from_slice(source);
            }
        }
    }
}
