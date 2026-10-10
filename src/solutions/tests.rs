use super::*;
fn resources() -> crate::resources::Resources {
    crate::resources::Resources::load(crate::test_support::game_data_root()).unwrap()
}
#[test]
fn puzzle4_tilted_seesaw_hints_complete_after_each_save_reload() {
    let res = resources();
    let bytes = crate::default_solutions::LAYOUTS
        .iter()
        .find(|s| s.0 == 4)
        .unwrap()
        .2;
    let (_, target) = crate::machine::parse_collection(bytes, false).unwrap();
    let mut current = crate::puzzle::PuzzleSession::load(&res, 4, 0).unwrap();
    let count = current.editor.objects.len();
    for _ in 0..4 {
        assert!(hint(&mut current.editor, &target).unwrap());
        current.restore(&current.snapshot_bytes()).unwrap();
        assert_eq!(current.editor.objects.len(), count);
    }
    assert!(!hint(&mut current.editor, &target).unwrap());
    let map = body_mapping(&current.editor.objects, &target);
    for (t, wanted) in target.iter().enumerate() {
        assert!(geometry(&current.editor.objects[map[t].unwrap()], wanted));
    }
}
#[test]
fn hints_reuse_placed_parts_and_survive_serialization() {
    let res = resources();
    let mut target = crate::puzzle::PuzzleSession::load(&res, 1, 0).unwrap();
    for cursor in [(104, 104), (200, 104)] {
        assert!(target.editor.take_definition(Def::Incline));
        target.editor.hover(cursor);
        assert!(target.editor.can_place());
        target.editor.click(cursor);
    }
    let (_, source) = crate::machine::parse_collection(&target.snapshot_bytes(), false).unwrap();
    let mut current = crate::puzzle::PuzzleSession::load(&res, 1, 0).unwrap();
    assert!(hint(&mut current.editor, &source).unwrap());
    let bin = current.editor.bin_count(Def::Incline);
    // Save/reopen changes object indices. The next hint must match the
    // first incline and consume just the second, not place a duplicate.
    current.restore(&current.snapshot_bytes()).unwrap();
    assert!(hint(&mut current.editor, &source).unwrap());
    assert_eq!(current.editor.bin_count(Def::Incline), bin - 1);
    let before = current.snapshot_bytes();
    assert!(!hint(&mut current.editor, &source).unwrap());
    assert_eq!(current.snapshot_bytes(), before);
    // Move an existing part off its answer, then correct it without
    // consuming another part from the bin.
    let i = current
        .editor
        .objects
        .iter()
        .position(|o| placed(o) && o.definition == Def::Incline && !locked(o))
        .unwrap();
    current.editor.objects[i].x += 32;
    let bin = current.editor.bin_count(Def::Incline);
    assert!(hint(&mut current.editor, &source).unwrap());
    assert_eq!(current.editor.bin_count(Def::Incline), bin);
}
#[test]
fn saved_rope_solution_can_be_revealed_one_item_at_a_time() {
    let res = resources();
    let bytes = include_bytes!("../default_solutions/data/20-019.TIM").to_vec();
    let (_, source) = crate::machine::parse_collection(&bytes, false).unwrap();
    let mut current = crate::puzzle::PuzzleSession::load(&res, 20, 0).unwrap();
    validate(&current.editor.objects, &source).unwrap();
    let count = current.editor.objects.len();
    let mut hints = 0;
    while hint(&mut current.editor, &source).unwrap() {
        hints += 1;
        assert!(hints <= count);
        current.restore(&current.snapshot_bytes()).unwrap();
        assert_eq!(current.editor.objects.len(), count);
    }
    assert!(hints > 2);
    assert_eq!(
        current
            .editor
            .objects
            .iter()
            .filter(|o| placed(o) && o.definition == Def::Rope)
            .count(),
        source
            .iter()
            .filter(|o| placed(o) && o.definition == Def::Rope)
            .count()
    );
    current.start();
    for _ in 0..30 {
        current.tick();
    }
    assert!(current.error.is_none());
}
#[test]
fn named_variants_do_not_overwrite_each_other() {
    let dir = std::env::temp_dir().join(format!("tim-solutions-{}", std::process::id()));
    let mut menu = SolutionMenu::default();
    menu.refresh(&dir, 1);
    menu.name = "First".into();
    menu.save_new(&dir, 1, b"first").unwrap();
    menu.name = "Second".into();
    menu.save_new(&dir, 1, b"second").unwrap();
    assert_eq!(menu.entries.len(), 2);
    assert_eq!(menu.selected_bytes().unwrap(), b"second");
    menu.name = "first".into();
    assert!(menu.save_new(&dir, 1, b"overwrite").is_err());
    menu.refresh(&dir, 2);
    assert!(menu.entries.is_empty());
    crate::storage::remove_dir_all(dir).unwrap();
}
