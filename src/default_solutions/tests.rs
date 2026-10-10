use super::*;
#[test]
fn legacy_default_names_migrate_preserving_selection_and_user_edits() {
    let root = std::env::temp_dir().join(format!("tim-default-migration-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    let dir = crate::solutions::directory(&root, 4);
    fs::create_dir_all(&dir).unwrap();
    let bytes = LAYOUTS.iter().find(|s| s.0 == 4).unwrap().2;
    fs::write(dir.join("Saved solution.TIM"), bytes).unwrap();
    fs::write(dir.join("SELECTED"), "Saved solution.TIM").unwrap();
    fs::write(root.join("SAVES/SOLUTIONS/DEFAULTS-v1"), "installed").unwrap();
    let edited = crate::solutions::directory(&root, 5);
    fs::create_dir_all(&edited).unwrap();
    fs::write(edited.join("Saved solution.TIM"), "my changed layout").unwrap();
    seed(&root).unwrap();
    assert_eq!(
        fs::read(dir.join("Default solution #1.TIM")).unwrap(),
        bytes
    );
    assert!(!dir.join("Saved solution.TIM").exists());
    assert_eq!(
        fs::read_to_string(dir.join("SELECTED")).unwrap(),
        "Default solution #1.TIM"
    );
    assert_eq!(
        fs::read(edited.join("Saved solution.TIM")).unwrap(),
        b"my changed layout"
    );
    assert!(edited.join("Default solution #1.TIM").exists());
    fs::remove_dir_all(root).unwrap();
}
#[test]
fn bundled_layouts_match_original_puzzles_and_seed_without_filling_board() {
    let original = crate::test_support::game_data_root();
    let resources = crate::resources::Resources::load(original).unwrap();
    for &(level, name, bytes) in LAYOUTS {
        let (_, target) = crate::machine::parse_collection(bytes, false).unwrap();
        let base = crate::machine::collection_objects(
            &resources,
            &crate::control_panel::level_name(level),
        )
        .unwrap();
        crate::solutions::validate(&base, &target)
            .unwrap_or_else(|e| panic!("Puzzle {level}, {name}: {e}"));
    }
    let root = std::env::temp_dir().join(format!("tim-default-hints-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    seed(&root).unwrap();
    assert!(!root.join("SAVES/PUZZLE20.TIM").exists());
    let mut front = crate::frontend::Frontend::new(&resources, root.clone(), Some(20)).unwrap();
    let fresh = crate::puzzle::PuzzleSession::load(&resources, 20, 0).unwrap();
    assert_eq!(front.session.snapshot_bytes(), fresh.snapshot_bytes());
    assert_eq!(front.solutions.selected_name(), Some("Default solution #1"));
    let count = front.session.editor.objects.len();
    front.open_solutions();
    front.solution_action(&resources, crate::menus::SolutionAction::Hint);
    assert_eq!(front.session.editor.objects.len(), count);
    assert_ne!(front.session.snapshot_bytes(), fresh.snapshot_bytes());
    let selected = front.solutions.selected.unwrap();
    let path = front.solutions.entries[selected].1.clone();
    fs::write(&path, b"user edited this solution").unwrap();
    seed(&root).unwrap();
    assert_eq!(fs::read(&path).unwrap(), b"user edited this solution");
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn all_bundled_solutions_reach_their_actual_puzzle_goal() {
    use crate::puzzle::{Phase, PuzzleSession};
    let root = crate::test_support::game_data_root();
    let resources = crate::resources::Resources::load(root).unwrap();
    assert_eq!(LAYOUTS.len(), 87);
    for &(level, name, bytes) in LAYOUTS {
        let mut session = PuzzleSession::load(&resources, level, 0).unwrap();
        session.restore(bytes).unwrap();
        session.start();
        for _ in 0..10_000 {
            session.tick();
            if !session.running() {
                break;
            }
        }
        assert_eq!(
            session.phase,
            Phase::Complete,
            "Puzzle {level}, {name}: {:?}",
            session.error
        );
    }
}

#[test]
fn refreshed_defaults_replace_old_library_and_preserve_user_layouts_and_board() {
    let root = std::env::temp_dir().join(format!("tim-default-refresh-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    let dir = crate::solutions::directory(&root, 1);
    fs::create_dir_all(&dir).unwrap();
    fs::write(root.join("SAVES/SOLUTIONS/DEFAULTS-v2"), b"installed").unwrap();
    fs::write(dir.join("Default solution #1.TIM"), b"old default").unwrap();
    fs::write(dir.join("Default solution #2.TIM"), b"old alternative").unwrap();
    fs::write(dir.join("My solution.TIM"), b"user layout").unwrap();
    fs::write(dir.join("SELECTED"), b"Default solution #2.TIM").unwrap();
    let board = crate::progress::puzzle_save_path(&root, 1);
    fs::write(&board, b"current board").unwrap();
    seed(&root).unwrap();
    assert_eq!(
        fs::read(dir.join("Default solution #1.TIM")).unwrap(),
        LAYOUTS[0].2
    );
    assert_eq!(
        fs::read(dir.join("Previous Default solution #1.TIM")).unwrap(),
        b"old default"
    );
    assert_eq!(
        fs::read(dir.join("Previous Default solution #2.TIM")).unwrap(),
        b"old alternative"
    );
    assert_eq!(
        fs::read(dir.join("My solution.TIM")).unwrap(),
        b"user layout"
    );
    assert_eq!(
        fs::read_to_string(dir.join("SELECTED")).unwrap(),
        "Default solution #1.TIM"
    );
    assert_eq!(fs::read(board).unwrap(), b"current board");
    fs::remove_dir_all(root).unwrap();
}
