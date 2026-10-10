use super::*;

#[test]
fn passwords_round_trip_score_codes() {
    let root = crate::test_support::game_data_root();
    let resources = crate::resources::Resources::load(root).unwrap();
    let passwords = Passwords::load(&resources).unwrap();
    assert_eq!(passwords.word(2), Some("SIERRA"));
    let password = passwords.generate(1, 697).unwrap();
    // 697 + 0x100000 = 0x1002B9; 697 * ('S'+'I'+'E') = 697 * 225.
    assert_eq!(
        password,
        format!(
            "SIERRA-ZZ2B9{}",
            radix(697 * 225, 34).replace('0', "Z").replace('O', "Y")
        )
    );
    assert_eq!(passwords.decode(&password), Some(Ok((2, 697))));
    assert_eq!(passwords.decode("dynamix"), Some(Ok((3, 0))));
    assert_eq!(passwords.decode("SIERRA-ZZ2B9AAA"), Some(Err(2)));
    assert_eq!(passwords.decode("NOPE"), None);
}

#[test]
fn graphics_presets_survive_restart_and_invalid_options_use_defaults() {
    let root = std::env::temp_dir().join(format!("tim-graphics-config-{}", std::process::id()));
    std::fs::create_dir_all(&root).unwrap();
    let mut p = Progress::default();
    p.resolution = 7;
    p.save(&root).unwrap();
    assert_eq!(Progress::load(&root), p);
    std::fs::write(
        options_path(&root),
        "resolution=0x0\ntheme=Custom\ncustom_color=378C28\n",
    )
    .unwrap();
    let restored = Progress::load(&root);
    assert_eq!(restored.resolution, 2);
    std::fs::remove_dir_all(root).unwrap();
}
