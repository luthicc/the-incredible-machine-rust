use super::*;

fn setup(tag: &str) -> (Resources, Frontend, PathBuf) {
    let root = crate::test_support::game_data_root();
    let resources = Resources::load(root).unwrap();
    let dir = std::env::temp_dir().join(format!("tim-frontend-{tag}-{}", std::process::id()));
    let _ = crate::storage::remove_dir_all(&dir);
    crate::storage::create_dir_all(&dir).unwrap();
    crate::storage::copy(root.join("CATOMATC.TIM"), dir.join("CATOMATC.TIM")).unwrap();
    let front = Frontend::new(&resources, dir.clone(), Some(1)).unwrap();
    (resources, front, dir)
}

fn click(front: &mut Frontend, resources: &Resources, at: (i16, i16)) {
    front.panel_press(at);
    front.panel_release(resources, at);
}

fn centre((x0, y0, x1, y1): crate::game_screen::Clip) -> (i16, i16) {
    (((x0 + x1) / 2) as i16, ((y0 + y1) / 2) as i16)
}

#[test]
fn defaults_are_read_only_and_hints_can_be_disabled() {
    use crate::menus::SolutionAction::*;
    let (resources, mut front, dir) = setup("read-only-defaults");
    front.open_solutions();
    assert!(front.solutions.selected_is_default());
    let original = front.solutions.selected_bytes().unwrap();
    front.solution_action(&resources, Update);
    assert!(front.solutions.status.contains("read-only"));
    front.solution_action(&resources, Delete);
    front.solution_action(&resources, Delete);
    assert_eq!(front.solutions.selected_bytes().unwrap(), original);
    front.solutions.name = "My version".into();
    front.solution_action(&resources, Save);
    assert_eq!(front.solutions.selected_name(), Some("My version"));
    assert!(!front.solutions.selected_is_default());
    front.solution_action(&resources, Update);
    assert_eq!(front.solutions.status, "Solution saved.");
    front.solutions.open = false;
    front.progress.puzzle_hints = false;
    let board = front.session.snapshot_bytes();
    front.open_solutions();
    assert!(!front.solutions.open);
    assert_eq!(front.session.snapshot_bytes(), board);
    crate::storage::remove_dir_all(dir).unwrap();
}

#[test]
fn solution_menu_selects_without_spoilers_and_reveals_one_item() {
    use crate::definition::PartDefinition as Def;
    use crate::menus::SolutionAction::*;
    let (resources, mut front, dir) = setup("solution-menu");
    front.open_solutions();
    assert!(!front.solutions.manage);
    front.solution_action(&resources, Manage);
    assert!(front.solutions.manage);
    for cursor in [(104, 104), (200, 104)] {
        assert!(front.session.editor.take_definition(Def::Incline));
        front.session.editor.hover(cursor);
        front.session.editor.click(cursor);
        assert!(front.session.editor.held.is_none());
    }
    let default_count = front.solutions.entries.len();
    front.solutions.name = "First".into();
    front.solution_action(&resources, Save);
    assert_eq!(front.solutions.status, "Solution saved.");
    front.solutions.name = "Alternative".into();
    front.solution_action(&resources, Save);
    assert_eq!(front.solutions.entries.len(), default_count + 2);
    front.session = PuzzleSession::load(&resources, 1, 0).unwrap();
    let fresh = front.session.snapshot_bytes();
    front.solutions.selected = front.solutions.entries.iter().position(|e| e.0 == "First");
    assert_eq!(front.session.snapshot_bytes(), fresh);
    front.solution_action(&resources, Hint);
    assert!(front.solutions.open);
    assert_eq!(front.solutions.selected_name(), Some("First"));
    assert_eq!(front.session.editor.bin_count(Def::Incline), 2);
    front.open_solutions();
    front.solution_action(&resources, Hint);
    assert_eq!(front.session.editor.bin_count(Def::Incline), 1);
    front.open_solutions();
    let before = front.session.snapshot_bytes();
    front.solution_action(&resources, Hint);
    assert_eq!(front.session.snapshot_bytes(), before);
    assert!(front.solutions.status.contains("already in place"));
    front.solution_action(&resources, Load);
    assert!(
        crate::solutions::directory(&dir, 1)
            .join("Before load 1.TIM")
            .exists()
    );
    front.open_solutions();
    let count = front.solutions.entries.len();
    front.solution_action(&resources, Delete);
    assert_eq!(front.solutions.entries.len(), count);
    front.solution_action(&resources, Delete);
    assert_eq!(front.solutions.entries.len(), count - 1);
    crate::storage::remove_dir_all(dir).unwrap();
}

#[test]
fn full_solve_loads_selected_layout_and_runs_to_real_goal() {
    let (resources, mut front, dir) = setup("full-solve");
    front.load_puzzle(&resources, 20, 0);
    let bytes = include_bytes!("../default_solutions/data/20-019.TIM").to_vec();
    front.solutions.name = "Complete solution".into();
    front.solutions.save_new(&dir, 20, &bytes).unwrap();
    front.open_solutions();
    assert_eq!(front.solutions.selected_name(), Some("Complete solution"));
    assert_eq!(
        crate::menus::solution_hit(&front.solutions, (400, 410)),
        Some(crate::menus::SolutionAction::Solve)
    );
    front.solution_action(&resources, crate::menus::SolutionAction::Solve);
    assert!(front.session.running());
    assert_eq!(front.view, View::Game);
    assert!(!front.solutions.open);
    for _ in 0..600 {
        front.session.tick();
        if front.session.phase == Phase::Complete {
            break;
        }
    }
    assert_eq!(front.session.phase, Phase::Complete);
    front.update_progress();
    assert!(front.progress.completed.contains(&20));
    let successful = crate::solutions::directory(&dir, 20).join("Last successful run.TIM");
    assert_eq!(crate::storage::read(&successful).unwrap(), bytes);
    // Editing and autosaving an unfinished board must preserve the last win.
    front.session = PuzzleSession::load(&resources, 20, 0).unwrap();
    front.autosave();
    front.update_progress();
    assert_eq!(crate::storage::read(&successful).unwrap(), bytes);
    front.solutions.refresh(&dir, 20);
    assert!(
        front
            .solutions
            .entries
            .iter()
            .any(|e| e.0 == "Last successful run")
    );
    assert!(
        crate::solutions::directory(&dir, 20)
            .join("Before load 1.TIM")
            .exists()
    );
    crate::storage::remove_dir_all(dir).unwrap();
}

#[test]
fn option_switches_and_speed_slider_persist_beside_tim_cfg() {
    let (resources, mut front, dir) = setup("options");
    let font = crate::font::Font::load(&resources).unwrap();
    for (control, clip) in [
        (Control::Qol, crate::menus::QOL_BUTTON),
        (Control::Music, crate::menus::MUSIC),
        (Control::Effects, crate::menus::EFFECTS),
    ] {
        for on in [true, false] {
            let span = (font.width(crate::menus::option_label(control, on)) + 7) & !7;
            assert!(
                span + 0x18 <= clip.2 - clip.0,
                "{control:?} {on} overflows its button"
            );
        }
        assert_eq!(crate::menus::control_at(centre(clip)), Some(control));
    }
    assert!(front.progress.smooth_motion && front.progress.music && front.progress.effects);
    click(&mut front, &resources, centre(crate::menus::QOL_BUTTON));
    assert!(front.qol_open);
    front
        .progress
        .toggle_qol(crate::progress::QolOption::SmoothMotion);
    front.save_progress();
    front.qol_open = false;
    click(&mut front, &resources, centre(crate::menus::MUSIC));
    let saved = Progress::load(&dir);
    assert!(!saved.smooth_motion && !saved.music && saved.effects);
    click(&mut front, &resources, centre(crate::menus::EFFECTS));
    assert!(!Progress::load(&dir).effects);
    for &(option, _) in &crate::progress::QOL_OPTIONS {
        let before = front.progress.qol_value(option);
        front.progress.toggle_qol(option);
        front.save_progress();
        assert_eq!(Progress::load(&dir).qol_value(option), !before);
    }
    // TIM.CFG keeps the original's two words.
    assert_eq!(crate::storage::read(dir.join("TIM.CFG")).unwrap().len(), 4);

    // The slider follows a drag and keeps the notch released off it.
    let (left, right) = crate::menus::SPEED_TRACK;
    let y = 0x1cb;
    assert_eq!(front.progress.speed, crate::progress::NORMAL_SPEED);
    front.panel_press((left as i16, y));
    assert_eq!(front.progress.speed, 0);
    front.panel_drag((right as i16, y));
    let last = crate::progress::SPEEDS.len() as u8 - 1;
    assert_eq!(front.progress.speed, last);
    front.panel_release(&resources, (right as i16 + 40, y + 30));
    assert_eq!(Progress::load(&dir).speed, last);
    // 3X of the slider's 1X, which is 3/4 of the PIT-derived rate.
    assert_eq!(Progress::load(&dir).speed(), 2.25);
    assert_eq!(Progress::default().speed(), 0.75);
}

#[test]
fn puzzles_reopen_as_last_built_and_restart_clears_them() {
    let (resources, mut front, dir) = setup("saves");
    // Build on puzzle 1: take an incline and place it in the open.
    let editor = &mut front.session.editor;
    let bin_before = editor.bin_groups().iter().map(|g| g.count).sum::<usize>();
    editor.take_from_bin(2);
    let part = editor.held.as_ref().unwrap().index;
    editor.hover((100, 101));
    editor.click((100, 101));
    let placed = (
        editor.objects[part].definition,
        editor.objects[part].x,
        editor.objects[part].y,
    );
    front.load_puzzle(&resources, 2, 0);
    assert!(crate::progress::puzzle_save_path(&dir, 1).exists());
    front.load_puzzle(&resources, 1, 0);
    let editor = &front.session.editor;
    assert!(
        editor
            .objects
            .iter()
            .any(|o| o.list != crate::machine::ObjectList::Inventory
                && (o.definition, o.x, o.y) == placed)
    );
    // The bin came back one part short.
    assert_eq!(
        editor.bin_groups().iter().map(|g| g.count).sum::<usize>(),
        bin_before - 1
    );
    // A fresh Frontend (a new game launch) reopens it too.
    let reopened = Frontend::new(&resources, dir.clone(), Some(1)).unwrap();
    assert!(
        reopened
            .session
            .editor
            .objects
            .iter()
            .any(|o| (o.definition, o.x, o.y) == placed
                && o.list != crate::machine::ObjectList::Inventory)
    );
    // RESTART clears the machine and its save.
    front.answer(&resources, true, After::Restart);
    assert!(
        !front
            .session
            .editor
            .objects
            .iter()
            .any(|o| o.list != crate::machine::ObjectList::Inventory
                && (o.definition, o.x, o.y) == placed)
    );
    assert!(!crate::progress::puzzle_save_path(&dir, 1).exists());
}

#[test]
fn launch_reopens_the_last_unlocked_puzzle() {
    let (resources, mut front, dir) = setup("last");
    front.progress.reached = 3;
    front.load_puzzle(&resources, 3, 0);
    assert_eq!(Progress::load(&dir).last_puzzle, 3);
    let relaunched = Frontend::new(&resources, dir.clone(), None).unwrap();
    assert_eq!(relaunched.session.level, 3);
    // A puzzle beyond the one reached (an edited config) is not opened.
    let mut progress = Progress::load(&dir);
    progress.reached = 2;
    progress.save(&dir).unwrap();
    assert_eq!(
        Frontend::new(&resources, dir.clone(), None)
            .unwrap()
            .session
            .level,
        2
    );
}

#[test]
fn solving_a_puzzle_marks_it_completed() {
    let (_resources, mut front, dir) = setup("completed");
    front.session.phase = Phase::Complete;
    front.update_progress();
    assert!(front.progress.completed.contains(&1));
    let saved = Progress::load(&dir);
    assert!(saved.completed.contains(&1));
    assert!(saved.reached >= 2);
    assert!(crate::progress::puzzle_save_path(&dir, 1).exists());
}

#[test]
fn panel_controls_open_their_dialogs_and_act() {
    let (resources, mut front, dir) = setup("controls");
    click(&mut front, &resources, (0x40, 0x60));
    assert_eq!(front.view, View::Game);
    front.view = View::Panel;
    // The volume starts at the original's 6; up is capped there.
    click(&mut front, &resources, (0x60, 0x60));
    assert_eq!(front.progress.volume, 6);
    click(&mut front, &resources, (0x60, 0x75));
    assert_eq!(front.progress.volume, 5);
    assert_eq!(Progress::load(&dir).volume, 5);
    // Meters refuse outside freeform.
    click(&mut front, &resources, (0x80, 220));
    assert!(
        matches!(&front.dialog, Some(Dialog::Message(m, After::Nothing)) if m.title == "CAN'T CHANGE GRAVITY")
    );
    front.type_text(&resources, "", true, false);
    assert!(front.dialog.is_none());
    // Quit asks first.
    click(&mut front, &resources, (0xe0, 0x68));
    assert!(matches!(
        front.dialog,
        Some(Dialog::Message(_, After::Quit))
    ));
    front.type_text(&resources, "", false, true);
    assert!(!front.quit);
    // Freeform: yes, then the meters and SAVE/LOAD work.
    click(&mut front, &resources, (0x40, 0x90));
    front.type_text(&resources, "", true, false);
    assert!(front.session.freeform);
    front.panel_press((0x43 + 0x50, 220));
    front.panel_release(&resources, (0x43 + 0x50, 220));
    assert_eq!(front.session.gravity, 0x100);
    click(&mut front, &resources, (0xd0, 0x98));
    let Some(Dialog::File(file)) = &front.dialog else {
        panic!("no save dialog")
    };
    assert_eq!(file.mode, FileMode::Save);
    assert!(file.files.iter().any(|f| f == "CATOMATC.TIM"));
    front.type_text(&resources, "mine", false, false);
    front.type_text(&resources, "", true, false);
    assert!(dir.join("MINE.TIM").exists());
    click(&mut front, &resources, (0xa0, 0x98));
    let Some(Dialog::File(file)) = front.dialog.clone() else {
        panic!("no load dialog")
    };
    let row = file.files.iter().position(|f| f == "CATOMATC.TIM").unwrap();
    let at = (0x44, 0x7c + 10 * row as i16 + 2);
    front.panel_press(at);
    front.panel_release(&resources, at);
    front.panel_press((0x50, 0x138));
    front.panel_release(&resources, (0x50, 0x138));
    assert!(front.dialog.is_none());
    let placed = front
        .session
        .editor
        .objects
        .iter()
        .filter(|o| o.list != crate::machine::ObjectList::Inventory)
        .count();
    assert_eq!(placed, 15, "CATOMATC.TIM has 15 parts");
    let _ = crate::storage::remove_dir_all(dir);
}

#[test]
fn select_puzzle_locks_unreached_puzzles_and_accepts_passwords() {
    let (resources, mut front, dir) = setup("select");
    front.progress.reached = 3;
    click(&mut front, &resources, (0x70, 0x90));
    assert!(matches!(front.dialog, Some(Dialog::Select(_))));
    // Puzzle 5 is locked; puzzle 3 is not.
    let row = |n: i16| (0x40, 0x4c + 10 * (n - 1) + 3);
    click(&mut front, &resources, row(5));
    assert!(matches!(&front.dialog, Some(Dialog::Message(m, _)) if m.title == "NEED PASSWORD"));
    front.type_text(&resources, "", true, false);
    assert!(matches!(front.dialog, Some(Dialog::Select(_))));
    click(&mut front, &resources, row(3));
    let password = front.passwords.generate(5, 1234).unwrap();
    click(&mut front, &resources, (0xa0, 0x140));
    front.type_text(&resources, &password, false, false);
    front.type_text(&resources, "", true, false);
    assert_eq!(front.progress.reached, 6);
    assert_eq!(front.session.score, 1234);
    click(&mut front, &resources, (0x200, 0x130));
    assert_eq!(front.session.level, 6);
    let _ = crate::storage::remove_dir_all(dir);
}

#[test]
fn unlock_all_persists_allows_last_level_and_preserves_earned_progress() {
    let (resources, mut front, dir) = setup("unlock-all");
    let last = front.level_count();
    front.progress.completed.insert(1);
    front
        .progress
        .toggle_qol(crate::progress::QolOption::UnlockAll);
    front.save_progress();
    assert_eq!(front.available_levels(), last);
    assert_eq!(front.progress.reached, 1);
    front.select_action(&resources, SelectPuzzle::new(1), SelectHit::Entry(last));
    assert!(matches!(front.dialog, Some(Dialog::Select(ref select)) if select.selected == last));
    front.select_action(&resources, SelectPuzzle::new(last), SelectHit::Done);
    assert_eq!(front.session.level, last);
    let mut relaunched = Frontend::new(&resources, dir.clone(), None).unwrap();
    assert_eq!(relaunched.session.level, last);
    assert!(relaunched.progress.unlock_all);
    assert_eq!(relaunched.progress.reached, 1);
    assert_eq!(
        relaunched
            .progress
            .completed
            .iter()
            .copied()
            .collect::<Vec<_>>(),
        vec![1]
    );
    relaunched
        .progress
        .toggle_qol(crate::progress::QolOption::UnlockAll);
    relaunched.save_progress();
    relaunched.select_action(&resources, SelectPuzzle::new(1), SelectHit::Entry(last));
    assert!(matches!(relaunched.dialog, Some(Dialog::Message(..))));
    assert_eq!(
        Frontend::new(&resources, dir.clone(), None)
            .unwrap()
            .session
            .level,
        1
    );
    crate::storage::remove_dir_all(dir).unwrap();
}
