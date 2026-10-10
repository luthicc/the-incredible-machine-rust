//! The launch control panel and its dialogs: the two-button message boxes,
//! SELECT PUZZLE, and the LOAD/SAVE machine file dialog.
//!
//! Controls and their actions follow run_launch_control_panel (1dff:2f13):
//! UI element list DS:4e77 and the mode jump table at CS:34bf. Drawing follows
//! draw_launch_control_panel (1dff:3642), show_two_button_dialog (24de:08b8),
//! draw_password_entry_dialog (1dff:14c5), and draw_file_dialog (1dff:56d9).
mod graphics_menu;
pub use graphics_menu::GraphicsMenu;
mod control;
pub use control::Control;
mod panel_view;
pub use panel_view::PanelView;
mod options;
pub use options::Options;
mod message;
pub use message::Message;
mod select_puzzle;
pub use select_puzzle::SelectPuzzle;
mod select_hit;
pub use select_hit::SelectHit;
mod file_mode;
pub use file_mode::FileMode;
mod file_dialog;
pub use file_dialog::FileDialog;
mod file_hit;
pub use file_hit::FileHit;
mod solution_action;
pub use solution_action::SolutionAction;

use crate::font::Font;
use crate::game_screen::{
    ALL, Assets, Clip, beveled_box, blit, bordered_panel, button, button_rect, fill, inset, inside,
    label_bar, line, viewport_frame,
};
use crate::puzzle::PuzzleSession;
use crate::resources::{Resources, Screen};

/// Option buttons; each clip holds either of its labels.
pub const QOL_BUTTON: Clip = (40, 0x1c2, 144, 0x1d6);
pub const MUSIC: Clip = (156, 0x1c2, 252, 0x1d6);
pub const EFFECTS: Clip = (264, 0x1c2, 344, 0x1d6);
/// The speed slider's notches run from SPEED_TRACK.0 to SPEED_TRACK.1.
pub const SPEED_TRACK: (i32, i32) = (412, 468);
const SPEED_CLIP: Clip = (404, 0x1bc, 512, 0x1dc);

pub fn option_label(control: Control, on: bool) -> &'static str {
    match (control, on) {
        (Control::Qol, _) => "MENU",
        (Control::Music, true) => "MUSIC: ON",
        (Control::Music, false) => "MUSIC: OFF",
        (Control::Effects, true) => "SFX: ON",
        _ => "SFX: OFF",
    }
}

/// The slider notch nearest a cursor x.
pub fn speed_at(x: i16) -> u8 {
    let (left, right) = SPEED_TRACK;
    let steps = crate::progress::SPEEDS.len() as i32 - 1;
    let span = right - left;
    ((i32::from(x) - left) * steps + span / 2)
        .div_euclid(span)
        .clamp(0, steps) as u8
}

fn speed_x(speed: u8) -> i32 {
    let (left, right) = SPEED_TRACK;
    left + (right - left) * i32::from(speed) / (crate::progress::SPEEDS.len() as i32 - 1)
}

const CONTROLS: [(Clip, Control); 17] = [
    ((0x110, 0x48, 0x210, 0xe8), Control::Play),
    ((0x3a, 0x5b, 0x4f, 0x7e), Control::Play),
    ((0xd8, 0x60, 0xf0, 0x77), Control::Quit),
    ((0x39, 0x86, 0x5f, 0xab), Control::Freeform),
    ((0x96, 0x8c, 0xbf, 0xa4), Control::Load),
    ((0x58, 0x5d, 0x6d, 0x6d), Control::VolumeUp),
    ((0x58, 0x6f, 0x6d, 0x7e), Control::VolumeDown),
    ((0xbc, 0x5c, 0xce, 0x7b), Control::Restart),
    ((0x6d, 0x85, 0x8c, 0xa3), Control::SelectPuzzle),
    ((200, 0x8c, 0xf1, 0xa4), Control::Save),
    ((0x41, 200, 0xe1, 0xf8), Control::TopMeter),
    ((0x41, 0x114, 0xe1, 0x144), Control::BottomMeter),
    (SOLUTIONS_BUTTON, Control::Solutions),
    (QOL_BUTTON, Control::Qol),
    (MUSIC, Control::Music),
    (EFFECTS, Control::Effects),
    (SPEED_CLIP, Control::Speed),
];

pub fn control_at(cursor: (i16, i16)) -> Option<Control> {
    CONTROLS
        .iter()
        .find(|(clip, _)| inside(*clip, cursor))
        .map(|&(_, c)| c)
}

/// MOUSE.BMP label cursors from the elements' cursor words; LOAD, SAVE, and
/// the meters show theirs only in freeform mode (1dff:3508..355f).
pub fn control_cursor(control: Option<Control>, freeform: bool) -> usize {
    match control {
        Some(Control::Play) => 16,
        Some(Control::VolumeUp | Control::VolumeDown) => 17,
        Some(Control::Restart) => 18,
        Some(Control::SelectPuzzle) => 19,
        Some(Control::Freeform) if !freeform => 20,
        Some(Control::Quit) => 21,
        Some(Control::Save) if freeform => 22,
        Some(Control::Load) if freeform => 23,
        Some(Control::TopMeter) if freeform => 24,
        Some(Control::BottomMeter) if freeform => 25,
        _ => 0,
    }
}

/// A meter value from the cursor: (x - 0x43) * range / 0xa0, clamped
/// (1dff:3371 for the top meter, range 0x200; 1dff:340a, range 0x80).
pub fn meter_value(cursor_x: i16, range: i32) -> i16 {
    ((i32::from(cursor_x) - 0x43) * range / 0xa0).clamp(0, range) as i16
}

/// draw_launch_control_panel with the game-screen frame and parts bin
/// around it and the status page below.
pub fn render_panel(
    resources: &Resources,
    assets: &Assets,
    font: &Font,
    view: &PanelView,
) -> Result<Screen, String> {
    let session = view.session;
    let mut screen = Screen {
        width: 640,
        height: 480,
        rgba: vec![0; 640 * 480 * 4],
    };
    fill(&mut screen, (0, 0, 640, 0x170), assets.panel);
    crate::game_screen::borders(
        &mut screen,
        assets,
        session,
        crate::game_screen::Hover::Nothing,
    );
    crate::game_screen::parts_bin(&mut screen, assets, font, session);
    crate::game_screen::start_control(&mut screen, assets, session, 0);

    // draw_machine_info_panel (1dff:37fd).
    bordered_panel(&mut screen, assets, (0x20, 0x20, 0x220, 0x158));
    let heading = if session.freeform {
        "FREEFORM MODE".to_string()
    } else {
        format!("PUZZLE {}: {}", session.level, session.title)
    };
    label_bar(&mut screen, assets, font, &heading, 0x3c, 0x27, 0x1bc);
    viewport_frame(
        &mut screen,
        assets,
        (0x110, 0x48, 0x100, 0xa0),
        assets.panel,
    );
    let objects = session.editor.machine()?;
    let preview = crate::machine::machine_preview(resources, &session.name, &objects)?;
    for y in 0x48..0xe8usize {
        let row = y * 640 * 4;
        let (a, b) = (row + 0x110 * 4, row + 0x210 * 4);
        screen.rgba[a..b].copy_from_slice(&preview.rgba[a..b]);
    }
    inset(&mut screen, assets, (0x110, 0xff, 0x100, 0x4c));
    let text = if session.freeform {
        "You can create any type of machine that you wish to in freeform mode."
    } else {
        view.description
    };
    font.wrapped(&mut screen, text, 0x114, 0x104, 0xf8, assets.shadow);

    // The control inset and its switches; frame + 1 is the pressed image.
    inset(&mut screen, assets, (0x2c, 0x42, 0xd0, 0x109));
    let cp = &assets.cp;
    let p = |c: Control| usize::from(view.pressed == Some(c));
    blit(&mut screen, &cp[16 + p(Control::Play)], 0x3a, 0x5b, ALL);
    blit(&mut screen, &cp[18 + p(Control::Quit)], 0xd8, 0x60, ALL);
    blit(&mut screen, &cp[31 + p(Control::Restart)], 0xbc, 0x5c, ALL);
    blit(
        &mut screen,
        &cp[41 + p(Control::SelectPuzzle)],
        0x6d,
        0x85,
        ALL,
    );
    if session.freeform {
        blit(&mut screen, &cp[33 + p(Control::Load)], 0x96, 0x8c, ALL);
        blit(&mut screen, &cp[29 + p(Control::Load)], 0xa6, 0x8b, ALL);
        blit(&mut screen, &cp[35 + p(Control::Save)], 200, 0x8c, ALL);
        blit(&mut screen, &cp[29 + p(Control::Save)], 0xd8, 0x8b, ALL);
    } else {
        blit(&mut screen, &cp[27 + p(Control::Freeform)], 0x39, 0x86, ALL);
    }
    // draw_launch_panel_parts_list (1dff:3adf): the volume box.
    for x in (0x84..0xb4).step_by(8) {
        for y in (0x5f..0x78).step_by(8) {
            blit(&mut screen, &cp[43], x, y, ALL);
        }
    }
    blit(&mut screen, &cp[37 + p(Control::VolumeUp)], 0x58, 0x5d, ALL);
    blit(
        &mut screen,
        &cp[39 + p(Control::VolumeDown)],
        0x58,
        0x6f,
        ALL,
    );
    blit(&mut screen, &cp[20], 0x6e, 0x60, ALL);
    const MARKS: [i32; 6] = [133, 136, 142, 148, 155, 163];
    for level in 1..=usize::from(view.volume.min(6)) {
        blit(
            &mut screen,
            &cp[20 + level],
            MARKS[level - 1],
            0x69 - 2 * (level as i32 - 1),
            ALL,
        );
    }
    // The meters (draw_launch_panel_gravity/air_pressure_meter) and labels.
    for (frame, x, y) in [(7, 65, 200), (9, 61, 229), (8, 65, 276), (9, 61, 305)] {
        blit(&mut screen, &cp[frame], x, y, ALL);
    }
    blit(
        &mut screen,
        &cp[6],
        61 + i32::from(session.gravity) * 160 / 512,
        224,
        ALL,
    );
    blit(
        &mut screen,
        &cp[6],
        61 + i32::from(session.air) * 160 / 128,
        300,
        ALL,
    );
    blit(&mut screen, &cp[3], 0x53, 0x42, ALL);
    blit(&mut screen, &cp[5], 100, 0xb2, ALL);
    blit(&mut screen, &cp[4], 0x5b, 0xfe, ALL);

    let (edit_roll, run_roll) = session.rolls();
    crate::game_screen::status_page(
        &mut screen,
        assets,
        session.score,
        (session.edit_bonus, edit_roll),
        (session.run_bonus, run_roll),
    );
    let options = Options {
        smooth_motion: view.smooth_motion,
        puzzle_hints: view.puzzle_hints,
        music: view.music,
        effects: view.effects,
        speed: view.speed,
    };
    draw_options(
        &mut screen,
        assets,
        font,
        &options,
        view.pressed,
        view.solution_name,
    );
    Ok(screen)
}

/// The option control under the cursor; unlike `control_at` this is safe
/// on the game screen, where the panel's other controls are not shown.
pub fn option_at(cursor: (i16, i16)) -> Option<Control> {
    control_at(cursor).filter(|c| {
        matches!(
            c,
            Control::Qol | Control::Solutions | Control::Music | Control::Effects | Control::Speed
        )
    })
}

/// The port's option row: three switches and the speed slider.
pub fn draw_options(
    screen: &mut Screen,
    assets: &Assets,
    font: &Font,
    view: &Options,
    pressed: Option<Control>,
    solution_name: Option<&str>,
) {
    for (control, clip, on) in [
        (Control::Qol, QOL_BUTTON, view.smooth_motion),
        (Control::Music, MUSIC, view.music),
        (Control::Effects, EFFECTS, view.effects),
    ] {
        let label = option_label(control, on);
        let span = (font.width(label) + 7) & !7;
        let x = (clip.0 + clip.2 - span - 0x18) / 2;
        button(
            screen,
            assets,
            font,
            label,
            x,
            clip.1,
            pressed == Some(control),
        );
    }
    if view.puzzle_hints {
        button(
            screen,
            assets,
            font,
            "HINTS",
            551,
            448,
            pressed == Some(Control::Solutions),
        );
        let mut name = solution_name.unwrap_or("Select solution").to_string();
        if font.width(&name) > 108 {
            while font.width(&format!("{name}...")) > 108 {
                name.pop();
            }
            name.push_str("...");
        }
        font.draw(
            screen,
            &name,
            580 - font.width(&name) / 2,
            470,
            assets.yellow,
        );
    }
    let y = 0x1cb;
    font.draw(screen, "SPEED", 360, y - 3, assets.yellow);
    let (left, right) = SPEED_TRACK;
    let track = assets.gray;
    fill(screen, (left, y, right + 1, y + 2), track);
    for notch in 0..crate::progress::SPEEDS.len() as u8 {
        let x = speed_x(notch);
        fill(screen, (x, y - 3, x + 1, y + 5), track);
    }
    let knob = &assets.cp[6];
    blit(
        screen,
        knob,
        speed_x(view.speed) - knob.width as i32 / 2,
        y - knob.height as i32 / 2 + 1,
        ALL,
    );
    let label = crate::progress::SPEEDS[usize::from(view.speed)].1;
    font.draw(screen, label, right + 16, y - 3, assets.yellow);
}

/// Typed text for a field (handle_file_dialog_text_input / password entry).
pub fn edit_text(text: &mut String, typed: &str, limit: usize) {
    for c in typed.chars() {
        match c {
            '\u{8}' => {
                text.pop();
            }
            c if !c.is_control() && text.chars().count() < limit => {
                text.push(c.to_ascii_uppercase())
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests;

/// Each option has a full-width click target, including its ON/OFF label.
pub fn qol_hit(cursor: (i16, i16)) -> Option<usize> {
    if !(120..520).contains(&cursor.0) {
        return None;
    }
    (0..crate::progress::QOL_OPTIONS.len()).find(|&row| {
        let y = qol_row_y(row);
        (y..y + 20).contains(&i32::from(cursor.1))
    })
}
fn qol_row_y(row: usize) -> i32 {
    if row < 3 {
        122 + row as i32 * 20
    } else {
        210 + (row as i32 - 3) * 20
    }
}
#[test]
fn category_headers_are_not_switches_and_all_rows_hit_their_own_option() {
    assert_eq!(qol_hit((130, 84)), None);
    assert_eq!(qol_hit((130, 192)), None);
    assert_eq!(qol_hit((320, 405)), None);
    for row in 0..crate::progress::QOL_OPTIONS.len() {
        assert_eq!(qol_hit((480, (qol_row_y(row) + 10) as i16)), Some(row));
    }
}
pub const QOL_DONE: Clip = (264, 408, 376, 436);

pub fn draw_qol(
    screen: &mut Screen,
    assets: &Assets,
    font: &Font,
    progress: &crate::progress::Progress,
    graphics: &GraphicsMenu,
    cursor: Option<(i16, i16)>,
) {
    bordered_panel(screen, assets, (104, 48, 536, 442));
    label_bar(screen, assets, font, "THE INCREDIBLE MACHINE", 120, 60, 400);
    fill(screen, (132, 58, 152, 78), assets.shadow);
    blit(screen, &crate::branding::icon(16), 134, 60, ALL);
    fill(screen, (120, 80, 520, 396), assets.gray);
    label_bar(screen, assets, font, "GRAPHICS", 120, 80, 400);
    label_bar(screen, assets, font, "QUALITY OF LIFE", 120, 188, 400);
    for (row, &(option, label)) in crate::progress::QOL_OPTIONS.iter().enumerate() {
        let y = qol_row_y(row);
        if !graphics.overlay_open() && cursor.and_then(qol_hit) == Some(row) {
            fill(screen, (120, y, 520, y + 20), assets.dark_gray);
        }
        font.draw(screen, label, 130, y + 8, assets.shadow);
        font.draw(
            screen,
            if progress.qol_value(option) {
                "ON"
            } else {
                "OFF"
            },
            474,
            y + 8,
            assets.shadow,
        );
    }
    button(screen, assets, font, "DONE", 280, 412, false);
    label_bar(
        screen,
        assets,
        font,
        crate::branding::EDITION,
        400,
        412,
        104,
    );
    graphics_menu::draw_graphics(screen, assets, font, progress, graphics);
}

pub const SOLUTIONS_BUTTON: Clip = (520, 448, 640, 480);
const SOLUTION_ACTIONS: [(Clip, SolutionAction); 9] = [
    ((332, 294, 620, 316), SolutionAction::Name),
    ((332, 322, 418, 348), SolutionAction::Save),
    ((428, 322, 514, 348), SolutionAction::Update),
    ((524, 322, 620, 348), SolutionAction::Delete),
    ((332, 352, 418, 378), SolutionAction::Hint),
    ((428, 352, 526, 378), SolutionAction::Load),
    ((532, 352, 620, 378), SolutionAction::Done),
    ((332, 268, 358, 292), SolutionAction::Up),
    ((594, 268, 620, 292), SolutionAction::Down),
];
pub const SOLUTIONS_PANEL: Clip = (320, 236, 632, 432);
pub fn solution_panel(menu: &crate::solutions::SolutionMenu) -> Clip {
    if menu.manage {
        SOLUTIONS_PANEL
    } else {
        (280, 336, 632, 432)
    }
}
pub fn solution_hit(
    menu: &crate::solutions::SolutionMenu,
    cursor: (i16, i16),
) -> Option<SolutionAction> {
    if !menu.manage {
        return [
            ((292, 368, 318, 392), SolutionAction::Up),
            ((606, 368, 632, 392), SolutionAction::Down),
            ((292, 400, 352, 426), SolutionAction::Hint),
            ((362, 400, 458, 426), SolutionAction::Solve),
            ((466, 400, 540, 426), SolutionAction::Manage),
            ((550, 400, 632, 426), SolutionAction::Done),
        ]
        .iter()
        .find(|(clip, _)| inside(*clip, cursor))
        .map(|(_, action)| *action);
    }
    SOLUTION_ACTIONS
        .iter()
        .find(|(clip, _)| inside(*clip, cursor))
        .map(|(_, action)| *action)
}
pub fn draw_solutions(
    screen: &mut Screen,
    assets: &Assets,
    font: &Font,
    menu: &crate::solutions::SolutionMenu,
) {
    let shift = if menu.manage { 0 } else { 100 };
    let left = if menu.manage { 332 } else { 292 };
    let right = if menu.manage { 594 } else { 606 };
    let width = if menu.manage { 288 } else { 328 };
    bordered_panel(screen, assets, solution_panel(menu));
    fill(screen, (left - 8, 260 + shift, 628, 428), assets.gray);
    label_bar(
        screen,
        assets,
        font,
        if menu.status.starts_with("All items") {
            "ALL ITEMS PLACED"
        } else {
            "HINTS / SOLUTIONS"
        },
        left,
        244 + shift,
        width,
    );
    let caption = menu
        .selected
        .and_then(|i| {
            menu.entries
                .get(i)
                .map(|(name, _)| format!("Selected: {name}"))
        })
        .unwrap_or_else(|| "No saved solutions".into());
    let shorten = |text: &str, width: i32| {
        let mut s = text.to_string();
        if font.width(&s) <= width {
            return s;
        }
        while font.width(&format!("{s}...")) > width {
            s.pop();
        }
        format!("{s}...")
    };
    button(screen, assets, font, "<", left, 268 + shift, false);
    font.draw(
        screen,
        &shorten(&caption, width - 66),
        left + 32,
        277 + shift,
        assets.shadow,
    );
    button(screen, assets, font, ">", right, 268 + shift, false);
    if !menu.manage {
        for (label, x) in [
            ("1 HINT", 292),
            ("FULL SOLVE", 362),
            ("MANAGE", 466),
            ("CLOSE", 550),
        ] {
            button(screen, assets, font, label, x, 400, false);
        }
        return;
    }
    inset(screen, assets, (332, 294, 288, 22));
    let name = if menu.name.is_empty() && !menu.editing_name {
        "New solution name".into()
    } else {
        format!("{}{}", menu.name, if menu.editing_name { "_" } else { "" })
    };
    font.draw(screen, &shorten(&name, 270), 340, 302, assets.shadow);
    for (label, x, y) in [
        ("SAVE AS", 332, 322),
        ("UPDATE", 428, 322),
        ("DELETE", 524, 322),
        ("1 HINT", 332, 352),
        ("LOAD ALL", 428, 352),
        ("CLOSE", 532, 352),
    ] {
        button(screen, assets, font, label, x, y, false);
        if menu.selected_is_default() && matches!(label, "UPDATE" | "DELETE") {
            let width = font.width(label);
            let span = (width + 7) & !7;
            font.draw(
                screen,
                label,
                x + ((span - width) >> 1) + 8,
                y + 4,
                [128, 128, 128, 255],
            );
        }
    }
    font.wrapped(screen, &menu.status, 332, 390, 284, assets.shadow);
}
