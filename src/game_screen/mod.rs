//! The puzzle screen: playfield, borders, parts bin, start control, hover
//! handles, status bar, and the level-complete panels.
//!
//! Layout from draw_game_screen (24de:0c4f), draw_screen_borders (24de:0d18),
//! draw_parts_list_panel (24de:101d), draw_selected_part_icon (24de:11ca),
//! draw_hover_highlight (24de:1429), and the status page drawn by
//! initialize_score_screen_assets (1dff:0fbb) and draw_status_bar_numbers
//! (1000:25d8). The status page is shown below the 400-line game page.
mod assets;
pub use assets::Assets;
mod hover;
pub use hover::Hover;
mod replay_choice;
pub use replay_choice::ReplayChoice;

use crate::editor::{Attach, AttachPoint, Carry, Handle, VIEWPORT};
use crate::font::Font;
use crate::machine::ObjectList;
use crate::puzzle::{Phase, PuzzleSession};
use crate::resources::{self, Resources, Screen};

pub(crate) type Clip = (i32, i32, i32, i32);
pub(crate) const ALL: Clip = (0, 0, 640, 480);

pub(crate) fn blit(dst: &mut Screen, src: &Screen, x: i32, y: i32, clip: Clip) {
    for sy in 0..src.height as i32 {
        let dy = y + sy;
        if dy < clip.1 || dy >= clip.3 {
            continue;
        }
        for sx in 0..src.width as i32 {
            let dx = x + sx;
            if dx < clip.0 || dx >= clip.2 {
                continue;
            }
            let s = ((sy as usize * src.width as usize) + sx as usize) * 4;
            if src.rgba[s + 3] == 0 {
                continue;
            }
            let d = (dy as usize * dst.width as usize + dx as usize) * 4;
            dst.rgba[d..d + 4].copy_from_slice(&src.rgba[s..s + 4]);
        }
    }
}

pub(crate) fn fill(dst: &mut Screen, (x0, y0, x1, y1): Clip, color: [u8; 4]) {
    for y in y0.max(0)..y1.min(dst.height as i32) {
        for x in x0.max(0)..x1.min(dst.width as i32) {
            let d = (y as usize * dst.width as usize + x as usize) * 4;
            dst.rgba[d..d + 4].copy_from_slice(&color);
        }
    }
}

pub(crate) fn line(dst: &mut Screen, a: (i32, i32), b: (i32, i32), color: [u8; 4], clip: Clip) {
    let steps = (b.0 - a.0).abs().max((b.1 - a.1).abs()).max(1);
    for i in 0..=steps {
        let x = a.0 + (b.0 - a.0) * i / steps;
        let y = a.1 + (b.1 - a.1) * i / steps;
        if clip.0 <= x && x < clip.2 && clip.1 <= y && y < clip.3 {
            let d = (y as usize * dst.width as usize + x as usize) * 4;
            dst.rgba[d..d + 4].copy_from_slice(&color);
        }
    }
}

/// Text with the one-pixel shadow draw_parts_list_panel uses.
pub(crate) fn shadowed(
    dst: &mut Screen,
    font: &Font,
    assets: &Assets,
    text: &str,
    x: i32,
    y: i32,
    color: [u8; 4],
) {
    font.draw(dst, text, x - 1, y + 1, assets.shadow);
    font.draw(dst, text, x, y, color);
}

/// Playfield clip: draw_rectangle(8, 8, 0x230, 0x160).
pub const PLAYFIELD: Clip = (8, 8, 0x238, 0x168);

/// Hit-test the game screen's UI elements (create_ui_element_list, 1000:85c9).
pub fn hover(session: &PuzzleSession, cursor: (i16, i16)) -> Hover {
    let (x, y) = cursor;
    if (0x240..0x278).contains(&x) && (0..0x3f).contains(&y) {
        return Hover::Start;
    }
    if (0x240..0x25b).contains(&x) && (0x43..0x5a).contains(&y) {
        return Hover::ScrollUp;
    }
    if (0x260..0x27f).contains(&x) && (0x43..0x5a).contains(&y) {
        return Hover::ScrollDown;
    }
    if (crate::editor::BIN_X.0..crate::editor::BIN_X.1).contains(&x) {
        if let Some(slot) = crate::editor::BIN_SLOTS
            .iter()
            .position(|&(a, b)| (a..=b).contains(&y))
        {
            return Hover::BinSlot(slot);
        }
    }
    let inside = (PLAYFIELD.0..PLAYFIELD.2).contains(&i32::from(x))
        && (PLAYFIELD.1..PLAYFIELD.3).contains(&i32::from(y));
    if inside
        && session.phase == Phase::Editing
        && session.editor.held.is_none()
        && session.editor.resizing.is_none()
    {
        let under = session.editor.handle_under(cursor);
        if matches!(under, None | Some((_, Some(Handle::Body) | None))) {
            if let Some(point) = session.editor.attach_point(cursor) {
                return Hover::Attach(point);
            }
        }
        if let Some((index, handle)) = under {
            return Hover::Part(index, handle);
        }
    }
    Hover::Nothing
}

/// MOUSE.BMP cursor frame for the current state (UI element cursor words
/// and set_element_cursor_from_tool_mode).
pub fn cursor_frame(session: &PuzzleSession, hovered: Hover) -> usize {
    if session.running() {
        return 0;
    }
    if let Some(held) = &session.editor.held {
        return match held.carry {
            Carry::Belt { .. } => 8,
            Carry::Rope { .. } => 9,
            Carry::Part { .. } => 2,
        };
    }
    match hovered {
        Hover::Start => 26,
        Hover::BinSlot(_) => 2,
        Hover::Part(_, Some(Handle::Body)) => 2,
        Hover::Part(_, Some(Handle::Remove)) => 3,
        Hover::Part(_, Some(Handle::FlipHorizontal)) => 4,
        Hover::Part(_, Some(Handle::FlipVertical)) => 5,
        Hover::Part(_, Some(Handle::ResizeLeft | Handle::ResizeRight)) => 6,
        Hover::Part(_, Some(Handle::ResizeTop | Handle::ResizeBottom)) => 7,
        Hover::Attach(AttachPoint {
            kind: Attach::Belt, ..
        }) => 8,
        Hover::Attach(AttachPoint {
            kind: Attach::Rope(_),
            ..
        }) => 9,
        _ => 0,
    }
}

pub fn render(
    resources: &Resources,
    assets: &Assets,
    font: &Font,
    session: &PuzzleSession,
    cursor: Option<(i16, i16)>,
    frame: u32,
) -> Result<Screen, String> {
    render_blended(resources, assets, font, session, cursor, frame, None)
}

/// `render`, with the running machine drawn part-way from the positions
/// before its last update (`machine::blend_positions`).
pub fn render_blended(
    resources: &Resources,
    assets: &Assets,
    font: &Font,
    session: &PuzzleSession,
    cursor: Option<(i16, i16)>,
    frame: u32,
    blend: Option<(&[(i16, i16)], f32)>,
) -> Result<Screen, String> {
    let mut screen = playfield(resources, assets, session, blend, cursor)?;
    let hovered = cursor.map_or(Hover::Nothing, |c| hover(session, c));
    if session.phase == Phase::Editing {
        if let Some(c) = cursor {
            carry_overlay(&mut screen, assets, session, c, frame);
        }
        if let Some(resize) = session.editor.resizing {
            highlight(&mut screen, assets, session, resize.index, 10, frame);
        } else if let Hover::Part(index, _) = hovered {
            highlight(&mut screen, assets, session, index, 10, frame);
        }
        if session.editor.held.is_none() {
            if let Some(part) = cursor
                .and_then(|c| session.editor.part_at(c))
                .or(session.editor.hovered)
            {
                for point in session.editor.attach_points_on(part) {
                    anchor_square(
                        &mut screen,
                        (i32::from(point.at.0), i32::from(point.at.1)),
                        assets.yellow,
                    );
                }
            }
        }
        if let (Hover::Attach(point), Some(c)) = (hovered, cursor) {
            attach_marker(&mut screen, assets, font, session, point, c);
        }
    }
    borders(&mut screen, assets, session, hovered);
    parts_bin(&mut screen, assets, font, session);
    start_control(&mut screen, assets, session, frame);
    status_bar(&mut screen, assets, session);
    match session.phase {
        Phase::Complete => complete_panel(&mut screen, assets, font, session),
        Phase::AskReplay => {
            let pressed = cursor.and_then(|c| replay_choice(font, c));
            replay_dialog(&mut screen, assets, font, pressed);
        }
        _ => {}
    }
    if let Some(error) = &session.error {
        shadowed(
            &mut screen,
            font,
            assets,
            &format!("Stopped: {error}"),
            12,
            370,
            assets.white,
        );
    }
    // The cursor itself is drawn by the presenter, at full window resolution.
    Ok(screen)
}

/// Objects drawn at world - viewport; the carried part is drawn as placed.
fn playfield(
    resources: &Resources,
    assets: &Assets,
    session: &PuzzleSession,
    blend: Option<(&[(i16, i16)], f32)>,
    cursor: Option<(i16, i16)>,
) -> Result<Screen, String> {
    // The machine stays as it finished behind the completion panels.
    let mut objects = match &session.run {
        Some(run) if session.phase != Phase::Editing => match blend {
            Some((before, alpha)) if session.running() => {
                crate::machine::blend_positions(&run.simulation.objects, before, alpha)
            }
            _ => run.simulation.objects.clone(),
        },
        _ if session.phase == Phase::Editing => session.editor.placement_preview(),
        _ => session.editor.objects.clone(),
    };
    if !session.running() {
        if let Some(held) = &session.editor.held {
            if matches!(held.carry, Carry::Part { .. }) {
                objects[held.index].list = ObjectList::Moving;
            }
        }
    }
    if session.phase == Phase::Editing {
        if let Some(c) = cursor {
            session.editor.preview_rope(&mut objects, c);
        }
    }
    // update_linked_pulley_geometry (1000:4cc8) is an editor placement
    // operation. Running ropes use the saved pulley tangents, even when
    // interpolated endpoints swing across an orientation quadrant.
    if session.phase == Phase::Editing {
        crate::machine::refresh_pulleys(&mut objects, None);
        // Editor moves change rope length immediately, even before a valid
        // drop. Run-mode rendering retains its stored allowances for slack.
        crate::machine::refresh_rope_lengths(&mut objects)?;
    }
    for o in &mut objects {
        o.x -= VIEWPORT.0;
        o.y -= VIEWPORT.1;
    }
    let mut screen = crate::machine::live_collection_frame_with_magnifier_ray(
        resources,
        &session.name,
        0,
        &objects,
        session.running()
            && session
                .run
                .as_ref()
                .is_some_and(|run| run.simulation.has_stepped()),
    )?;
    // Everything outside the playfield belongs to the border and panels.
    let background = assets.panel;
    fill(&mut screen, (0, 0, 640, PLAYFIELD.1), background);
    fill(&mut screen, (0, PLAYFIELD.3, 640, 0x170), background);
    fill(&mut screen, (0, 0, PLAYFIELD.0, PLAYFIELD.3), background);
    fill(&mut screen, (PLAYFIELD.2, 0, 640, PLAYFIELD.3), background);
    Ok(screen)
}

/// Placement feedback: green/red marching outlines and a red cross on collision
/// (draw_hover_highlight colour 0xe), the belt/rope line from the first end.
fn carry_overlay(
    screen: &mut Screen,
    assets: &Assets,
    session: &PuzzleSession,
    cursor: (i16, i16),
    frame: u32,
) {
    let editor = &session.editor;
    let Some(held) = &editor.held else { return };
    let red = [252, 0, 0, 255];
    let target = editor.connection_preview_target(cursor);
    let connection_color = if target.is_some() {
        [0, 220, 0, 255]
    } else {
        red
    };
    let end = target
        .map(|(x, y)| (x - i32::from(VIEWPORT.0), y - i32::from(VIEWPORT.1)))
        .unwrap_or((i32::from(cursor.0), i32::from(cursor.1)));
    match &held.carry {
        Carry::Part { .. } => {
            highlight(
                screen,
                assets,
                session,
                held.index,
                if editor.can_place() { 12 } else { 14 },
                frame,
            );
            for (owner, rope_slot, fits) in editor.held_connection_states() {
                let color = if fits { assets.green } else { assets.red };
                if let Some(slot) = rope_slot {
                    if let Some(rope) = editor.objects[owner].ropes[slot].as_ref() {
                        if let Ok(segments) = crate::machine::rope_segments(&editor.objects, rope) {
                            for (a, b) in segments {
                                line(
                                    screen,
                                    (a.0 - i32::from(VIEWPORT.0), a.1 - i32::from(VIEWPORT.1)),
                                    (b.0 - i32::from(VIEWPORT.0), b.1 - i32::from(VIEWPORT.1)),
                                    color,
                                    PLAYFIELD,
                                );
                            }
                        }
                    }
                } else if let Some([Some(a), Some(b)]) = editor.objects[owner].belt {
                    for (p, q) in
                        crate::machine::belt_segments(&editor.objects[a], &editor.objects[b])
                    {
                        line(
                            screen,
                            (p.0 - i32::from(VIEWPORT.0), p.1 - i32::from(VIEWPORT.1)),
                            (q.0 - i32::from(VIEWPORT.0), q.1 - i32::from(VIEWPORT.1)),
                            color,
                            PLAYFIELD,
                        );
                    }
                }
            }
        }
        Carry::Belt { first: Some(a) } => {
            let o = &editor.objects[*a];
            let (x, y) = o.belt_origin();
            let half = i32::from(o.belt_size / 2);
            let start = (
                x + half - i32::from(VIEWPORT.0),
                y + half - i32::from(VIEWPORT.1),
            );
            line(screen, start, end, connection_color, PLAYFIELD);
        }
        Carry::Rope { route } if !route.is_empty() => {
            let mut objects = editor.objects.clone();
            editor.preview_rope(&mut objects, cursor);
            for (n, &(part, slot)) in route.iter().enumerate() {
                let outgoing =
                    if objects[part].definition == crate::definition::PartDefinition::Pulley {
                        1
                    } else {
                        slot as usize
                    };
                let p = crate::machine::rope_point(&objects[part], outgoing);
                let q = route
                    .get(n + 1)
                    .map(|&(next, next_slot)| {
                        let incoming = if objects[next].definition
                            == crate::definition::PartDefinition::Pulley
                        {
                            0
                        } else {
                            next_slot as usize
                        };
                        crate::machine::rope_point(&objects[next], incoming)
                    })
                    .map(|(x, y)| (x - i32::from(VIEWPORT.0), y - i32::from(VIEWPORT.1)))
                    .unwrap_or(end);
                line(
                    screen,
                    (p.0 - i32::from(VIEWPORT.0), p.1 - i32::from(VIEWPORT.1)),
                    q,
                    connection_color,
                    PLAYFIELD,
                );
            }
        }
        _ => {}
    }
}

fn anchor_square(screen: &mut Screen, at: (i32, i32), color: [u8; 4]) {
    let (x, y) = at;
    let ring = [
        (x - 4, y - 4),
        (x + 4, y - 4),
        (x + 4, y + 4),
        (x - 4, y + 4),
    ];
    for i in 0..4 {
        line(screen, ring[i], ring[(i + 1) % 4], color, PLAYFIELD);
    }
}

/// The anchor ringed and, when a click would attach a new one, the number
/// left in the bin beside the belt or rope cursor, e.g. "2x".
fn attach_marker(
    screen: &mut Screen,
    assets: &Assets,
    font: &Font,
    session: &PuzzleSession,
    point: AttachPoint,
    cursor: (i16, i16),
) {
    let (x, y) = (i32::from(point.at.0), i32::from(point.at.1));
    anchor_square(screen, (x, y), assets.yellow);
    let definition = match point.kind {
        Attach::Belt => crate::definition::PartDefinition::Belt,
        Attach::Rope(_) => crate::definition::PartDefinition::Rope,
    };
    let count = session.editor.bin_count(definition);
    let label = if session.editor.templates.is_some() || point.connection.is_some() {
        String::new()
    } else {
        format!("{count}x")
    };
    let frame = if definition == crate::definition::PartDefinition::Belt {
        8
    } else {
        9
    };
    let width = assets.mouse.get(frame).map_or(48, |s| s.width as i32);
    shadowed(
        screen,
        font,
        assets,
        &label,
        i32::from(cursor.0) + width + 2,
        i32::from(cursor.1) + 4,
        assets.yellow,
    );
}

/// draw_hover_highlight: a marching dashed frame and the handle icons that
/// compute_object_connection_flags allows.
fn highlight(
    screen: &mut Screen,
    assets: &Assets,
    session: &PuzzleSession,
    index: usize,
    color: usize,
    frame: u32,
) {
    let editor = &session.editor;
    let o = &editor.objects[index];
    let (x0, y0) = (i32::from(o.x - VIEWPORT.0), i32::from(o.y - VIEWPORT.1));
    let (w, h) = (i32::from(o.width), i32::from(o.height));
    let x1 = (x0 + w - 1).min(0x237);
    let y1 = (y0 + h - 1).min(0x167);
    let (x0, y0) = (x0.max(8), y0.max(8));
    if x0 > x1 || y0 > y1 {
        return;
    }
    if color == 14 {
        line(screen, (x0, y0 + 1), (x1, y1 + 1), assets.shadow, PLAYFIELD);
        line(screen, (x0, y1 + 1), (x1, y0 + 1), assets.shadow, PLAYFIELD);
        line(screen, (x0, y0), (x1, y1), assets.red, PLAYFIELD);
        line(screen, (x0, y1), (x1, y0), assets.red, PLAYFIELD);
    }
    let march = (frame % 4) as i32;
    let horizontal = &assets.mouse[color];
    let vertical = &assets.mouse[color + 1];
    let edge = |x0: i32, y0: i32, x1: i32, y1: i32| (x0, y0, x1 + 1, y1 + 1);
    blit(screen, horizontal, x0 - march, y0, edge(x0, y0, x1, y0));
    blit(
        screen,
        horizontal,
        x0 - (4 - march),
        y1,
        edge(x0, y1, x1, y1),
    );
    blit(screen, vertical, x0, y0 - (4 - march), edge(x0, y0, x0, y1));
    blit(screen, vertical, x1, y0 - march, edge(x1, y0, x1, y1));
    for icon in editor.handle_icons(index) {
        blit(
            screen,
            &assets.mouse[icon.frame],
            i32::from(icon.at.0),
            i32::from(icon.at.1),
            ALL,
        );
    }
}

/// draw_top/bottom/left/right_border with GP_BORD frames.
pub(crate) fn borders(
    screen: &mut Screen,
    assets: &Assets,
    session: &PuzzleSession,
    hovered: Hover,
) {
    let b = &assets.border;
    for x in (0x10..0x22f).step_by(8) {
        blit(screen, &b[6], x, 0, ALL);
        blit(screen, &b[7], x, 0x168, ALL);
    }
    for y in (8..0x162).step_by(8) {
        blit(screen, &b[4], 0, y, ALL);
        blit(screen, &b[5], 0x238, y, ALL);
    }
    for y in (0..0x16f).step_by(8) {
        blit(screen, &b[8], 0x278, y, ALL);
    }
    blit(screen, &b[0], 0, 0, ALL);
    blit(screen, &b[2], 0, 0x160, ALL);
    blit(screen, &b[1], 0x230, 0, ALL);
    blit(screen, &b[3], 0x230, 0x160, ALL);
    blit(screen, &b[10], 0x238, 0, ALL);
    blit(screen, &b[10], 0x238, 0x3b, ALL);
    blit(screen, &b[11], 0x23f, 0x42, ALL);
    // Pressed scroll arrows while the bin scrolls (game modes 0x800/0x400).
    let pressing = session.phase == Phase::Editing;
    if pressing && hovered == Hover::ScrollUp {
        blit(screen, &b[40], 0x248, 0x45, ALL);
    } else if pressing && hovered == Hover::ScrollDown {
        blit(screen, &b[41], 0x25d, 0x45, ALL);
    }
    blit(screen, &b[10], 0x238, 0x59, ALL);
    blit(screen, &b[9], 0x240, 0x168, ALL);
}

/// draw_parts_list_panel: ICONS.BMP frame per definition centred in a
/// 0x38 x 0x2a cell, the count centred below in yellow with a shadow.
pub(crate) fn parts_bin(
    screen: &mut Screen,
    assets: &Assets,
    font: &Font,
    session: &PuzzleSession,
) {
    // draw_parts_list_panel (24de:101d) uses DS:52c9, palette entry 11.
    fill(screen, (0x241, 99, 0x278, 101), assets.picker);
    fill(screen, (0x240, 0x65, 0x278, 0x168), assets.picker);
    for (y, group) in session.editor.visible_bin() {
        let Some(icon) = assets.icons.get(group.definition.id() as usize) else {
            continue;
        };
        let (w, h) = (icon.width as i32, icon.height as i32);
        let y = i32::from(y);
        blit(
            screen,
            icon,
            0x240 + (0x38 - w) / 2,
            y + (0x2a - h) / 2,
            ALL,
        );
        if session.freeform {
            continue;
        }
        let text = group.count.to_string();
        let x = ((0x38 - font.width(&text)) >> 1) + 0x240;
        let ty = (y + h + ((0x2a - h) >> 1) + 1).min(0x161);
        shadowed(screen, font, assets, &text, x, ty, assets.yellow);
    }
}

/// draw_selected_part_icon: sky, hedge, fence, the pitcher, and the slide.
pub(crate) fn start_control(
    screen: &mut Screen,
    assets: &Assets,
    session: &PuzzleSession,
    frame: u32,
) {
    let m = &assets.menu;
    let clip = (0x240, 10, 0x278, 0x3c);
    blit(screen, &m[0], 0x240, 10, clip);
    blit(screen, &m[1], 0x208, 0x1a, clip);
    blit(screen, &m[2], 0x208, 0x20, clip);
    // DS:25a2 frame, DS:25ae x, DS:25ba y; the throw animates while running.
    const FRAMES: [(usize, i32, i32); 4] = [(3, 600, 19), (4, 596, 16), (5, 596, 15), (6, 596, 19)];
    let step = if session.running() {
        (frame / 4) as usize % 4
    } else {
        0
    };
    let (sprite, x, y) = FRAMES[step];
    blit(screen, &m[sprite], x, y, clip);
    blit(screen, &m[7], 0x24a, 0x2a, clip);
}

/// The status page: SCORE1 plaques and SCORE2 odometer digits
/// (render_decimal_digit, 24de; clip y 0x1b..0x45).
fn status_bar(screen: &mut Screen, assets: &Assets, session: &PuzzleSession) {
    let (edit_roll, run_roll) = session.rolls();
    status_page(
        screen,
        assets,
        session.score,
        (session.edit_bonus, edit_roll),
        (session.run_bonus, run_roll),
    );
}

/// The status page sits directly below the 368-line game page: the VGA line
/// compare is 0x16f and vertical blank starts at 0x1bf (1dff:04ce), so the
/// picture is 448 lines and the last 32 are blank.
pub(crate) fn status_page(
    screen: &mut Screen,
    assets: &Assets,
    score: i32,
    edit: (i16, i16),
    run: (i16, i16),
) {
    let top = 0x170;
    fill(screen, (0, top, 640, 480), [0, 0, 0, 255]);
    blit(screen, &assets.score[0], 3, top, ALL);
    blit(screen, &assets.score[1], 0x107, top, ALL);
    blit(screen, &assets.score[2], 0x1bb, top, ALL);
    let clip = (0, top + 0x1b, 640, top + 0x46);
    let digit = |screen: &mut Screen, d: u8, x: i32, roll: i32| {
        let (strip, row) = if d < 5 { (0, d) } else { (1, d - 5) };
        blit(
            screen,
            &assets.digits[strip],
            x,
            top + 6 - 21 * i32::from(row) + roll,
            clip,
        );
    };
    let number = |screen: &mut Screen, value: i32, digits: usize, x: i32, roll: i32| {
        let text = format!("{:0width$}", value.max(0), width = digits);
        let mut borrow = true;
        for (i, c) in text.chars().rev().enumerate() {
            // 1000:262b continues left while the lower digits are zero.
            digit(
                screen,
                c as u8 - b'0',
                x - 0x20 * i as i32,
                if borrow { roll } else { 0 },
            );
            borrow &= c == '0';
        }
    };
    number(screen, score, 6, 0xd0, 0);
    number(screen, i32::from(edit.0), 4, 0x184, i32::from(edit.1));
    number(screen, i32::from(run.0), 4, 0x238, i32::from(run.1));
}

/// draw_inset_panel: the launch control panel's recessed CP.BMP text area.
pub(crate) fn inset(screen: &mut Screen, assets: &Assets, (x, y, w, h): (i32, i32, i32, i32)) {
    let cp = &assets.cp;
    for ty in (0..h).step_by(64) {
        for tx in (0..w).step_by(64) {
            blit(screen, &cp[58], x + tx, y + ty, (x, y, x + w, y + h));
        }
    }
    for ty in ((y + 19)..(y + h)).step_by(8) {
        blit(screen, &cp[14], x - 2, ty, ALL);
    }
    for tx in ((x + 16)..(x + w)).step_by(8) {
        blit(screen, &cp[15], tx, y + h - 4, ALL);
    }
    for (frame, px, py) in [
        (10, x - 7, y - 4),
        (11, x + w - 16, y - 4),
        (12, x - 7, y + h - 16),
        (13, x + w - 19, y + h - 14),
    ] {
        blit(screen, &cp[frame], px, py, ALL);
    }
}

/// draw_bordered_panel (24de:000c) with corners (x0,y0)-(x1,y1): a shadow
/// offset (-0xc,+0xc) with GP_BORD 37..39, wood tiles (42), gold edges
/// (16..19) and corners (12..15).
pub(crate) fn bordered_panel(screen: &mut Screen, assets: &Assets, (x0, y0, x1, y1): Clip) {
    let b = &assets.border;
    fill(
        screen,
        (x0 - 0xc, y0 + 0xc, x1 - 0xc, y1 + 0xc),
        assets.shadow,
    );
    blit(screen, &b[37], x0 - 0xf, y0 + 7, ALL);
    blit(screen, &b[38], x0 - 0xf, y1 - 9, ALL);
    blit(screen, &b[39], x1 - 0x20, y1 - 9, ALL);
    for y in (y0..y1).step_by(0x40) {
        for x in (x0..x1).step_by(0x80) {
            blit(screen, &b[42], x, y, (x0, y0, x1, y1));
        }
    }
    for x in (x0..x1).step_by(8) {
        blit(screen, &b[18], x, y0 - 4, ALL);
        blit(screen, &b[19], x, y1, ALL);
    }
    for y in (y0..y1).step_by(8) {
        blit(screen, &b[16], x0 - 4, y, ALL);
        blit(screen, &b[17], x1, y, ALL);
    }
    blit(screen, &b[12], x0 - 7, y0 - 7, ALL);
    blit(screen, &b[13], x1 - 0x11, y0 - 7, ALL);
    blit(screen, &b[14], x0 - 7, y1 - 0x11, ALL);
    blit(screen, &b[15], x1 - 0x11, y1 - 0x11, ALL);
}

/// draw_label_bar (24de:0224): CP.BMP 0/1/2 with centred text in palette
/// colours 0xf (offset shadow) and 5.
pub(crate) fn label_bar(
    screen: &mut Screen,
    assets: &Assets,
    font: &Font,
    text: &str,
    x: i32,
    y: i32,
    w: i32,
) {
    let cp = &assets.cp;
    blit(screen, &cp[0], x, y, ALL);
    let mut tx = x + 0x18;
    while tx < x + w - 0x18 {
        blit(screen, &cp[1], tx, y + 2, ALL);
        tx += 8;
    }
    blit(screen, &cp[2], x + w - 0x18, y, ALL);
    let tx = x + ((w - font.width(text)) >> 1);
    font.draw(screen, text, tx - 1, y + 6, assets.white);
    font.draw(screen, text, tx, y + 5, assets.label);
}

/// draw_button (24de:02fb): CP.BMP caps 44/48 and tiles 46 (+1 pressed);
/// the right cap sits at x + ((text width + 7) & !7) + 8.
pub(crate) fn button(
    screen: &mut Screen,
    assets: &Assets,
    font: &Font,
    text: &str,
    x: i32,
    y: i32,
    pressed: bool,
) {
    let cp = &assets.cp;
    let p = usize::from(pressed);
    let width = font.width(text);
    let span = (width + 7) & !7;
    let right = x + span + 8;
    blit(screen, &cp[44 + p], x, y, ALL);
    let mut tx = x + 8;
    while tx < right {
        blit(screen, &cp[46 + p], tx, y, ALL);
        tx += 8;
    }
    blit(screen, &cp[48 + p], right, y, ALL);
    let shift = p as i32;
    font.draw(
        screen,
        text,
        x + ((span - width) >> 1) + 8 - shift,
        y + shift * 2 + 4,
        assets.label,
    );
}

pub(crate) fn button_rect(font: &Font, text: &str, x: i32) -> Clip {
    let span = (font.width(text) + 7) & !7;
    (x, 0xd4, x + span + 0x18, 0xd4 + 0x14)
}

const REPLAY: &str = "REPLAY";
const ADVANCE: &str = "ADVANCE";

/// show_two_button_dialog (24de:08b8): the first button at x 200, the
/// second ending at 0x168, both at y 0xd4.
fn replay_buttons(font: &Font) -> (Clip, Clip) {
    let advance_x = 0x168 - ((font.width(ADVANCE) + 7) & !7);
    (
        button_rect(font, REPLAY, 200),
        button_rect(font, ADVANCE, advance_x),
    )
}

pub fn replay_choice(font: &Font, cursor: (i16, i16)) -> Option<ReplayChoice> {
    let (replay, advance) = replay_buttons(font);
    if inside(replay, cursor) {
        Some(ReplayChoice::Replay)
    } else if inside(advance, cursor) {
        Some(ReplayChoice::Advance)
    } else {
        None
    }
}

/// draw_level_complete_screen (24de:0ae5).
fn complete_panel(screen: &mut Screen, assets: &Assets, font: &Font, session: &PuzzleSession) {
    bordered_panel(screen, assets, (0xb0, 0x70, 0x190, 0xf8));
    label_bar(
        screen,
        assets,
        font,
        &format!("PUZZLE {} COMPLETED!", session.level),
        0xb8,
        0x80,
        0xd0,
    );
    label_bar(
        screen,
        assets,
        font,
        &format!("Total bonus points: {}", session.total_bonus()),
        0xb8,
        0x9c,
        0xd0,
    );
    if let Some(password) = &session.new_password {
        label_bar(screen, assets, font, "New Password", 0xb8, 0xc4, 0xd0);
        label_bar(screen, assets, font, password, 0xb8, 0xd8, 0xd0);
    }
    let text = "(click button to continue)";
    font.draw(screen, text, 0xd3, 0xee, assets.shadow);
    font.draw(screen, text, 0xd4, 0xed, assets.white);
}

/// show_two_button_dialog("REPLAY SOLUTION", ..., "REPLAY", "ADVANCE").
fn replay_dialog(screen: &mut Screen, assets: &Assets, font: &Font, hovered: Option<ReplayChoice>) {
    bordered_panel(screen, assets, (0xb0, 0x70, 0x190, 0xf8));
    label_bar(screen, assets, font, "REPLAY SOLUTION", 0xb8, 0x74, 0xd0);
    inset(screen, assets, (0xb8, 0x90, 0xd0, 0x5a));
    font.wrapped(
        screen,
        "Do you want to advance to the next puzzle or replay your solution to this puzzle?",
        0xbc,
        0x94,
        200,
        assets.shadow,
    );
    let advance_x = 0x168 - ((font.width(ADVANCE) + 7) & !7);
    button(
        screen,
        assets,
        font,
        REPLAY,
        200,
        0xd4,
        hovered == Some(ReplayChoice::Replay),
    );
    button(
        screen,
        assets,
        font,
        ADVANCE,
        advance_x,
        0xd4,
        hovered == Some(ReplayChoice::Advance),
    );
}

/// draw_viewport_frame (24de:0743): a filled well framed by GP_BORD 20..27.
pub(crate) fn viewport_frame(
    screen: &mut Screen,
    assets: &Assets,
    (x, y, w, h): (i32, i32, i32, i32),
    color: [u8; 4],
) {
    let b = &assets.border;
    fill(screen, (x, y, x + w, y + h), color);
    let (right, bottom) = (x + w, y + h);
    for tx in (x..right).step_by(8) {
        blit(screen, &b[26], tx, y - 8, ALL);
        blit(screen, &b[27], tx, bottom, ALL);
    }
    for ty in (y..bottom).step_by(8) {
        blit(screen, &b[24], x - 8, ty, ALL);
        blit(screen, &b[25], right, ty, ALL);
    }
    blit(screen, &b[20], x - 8, y - 8, ALL);
    blit(screen, &b[21], right - 8, y - 8, ALL);
    blit(screen, &b[22], x - 8, bottom - 5, ALL);
    blit(screen, &b[23], right - 8, bottom - 8, ALL);
}

/// draw_beveled_box (24de:05d8): CP.BMP 43 tiles, 54..57 edges, 50..53 corners.
pub(crate) fn beveled_box(
    screen: &mut Screen,
    assets: &Assets,
    (x, y, w, h): (i32, i32, i32, i32),
) {
    let cp = &assets.cp;
    let mut ty = 8;
    while ty < h - 8 {
        let mut tx = 8;
        while tx < w - 8 {
            blit(screen, &cp[43], x + tx, y + ty, ALL);
            tx += 8;
        }
        blit(screen, &cp[54], x, y + ty, ALL);
        blit(screen, &cp[55], x + w - 8, y + ty, ALL);
        ty += 8;
    }
    let mut tx = 8;
    while tx < w - 8 {
        blit(screen, &cp[56], x + tx, y, ALL);
        blit(screen, &cp[57], x + tx, y + h - 8, ALL);
        tx += 8;
    }
    blit(screen, &cp[50], x, y, ALL);
    blit(screen, &cp[51], x + w - 0x10, y, ALL);
    blit(screen, &cp[52], x, y + h - 0x10, ALL);
    blit(screen, &cp[53], x + w - 0x10, y + h - 0x10, ALL);
}

/// Draw a MOUSE.BMP cursor frame with its hot spot at the pointer.
/// The MOUSE.BMP frame the puzzle screen shows for a cursor position.
pub fn puzzle_cursor(session: &PuzzleSession, cursor: (i16, i16)) -> usize {
    cursor_frame(session, hover(session, cursor))
}

pub fn inside(clip: Clip, (x, y): (i16, i16)) -> bool {
    clip.0 <= i32::from(x)
        && i32::from(x) < clip.2
        && clip.1 <= i32::from(y)
        && i32::from(y) < clip.3
}

#[cfg(test)]
mod tests;
