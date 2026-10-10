use super::*;
fn fixture(tag: &str) -> PathBuf {
    let path = std::env::temp_dir().join(format!("tim-import-{tag}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&path);
    fs::create_dir_all(&path).unwrap();
    path
}
fn original() -> PathBuf {
    crate::test_support::game_data_root().into()
}
fn zip(path: &Path, names: Vec<(String, Vec<u8>)>) {
    let mut writer = zip::ZipWriter::new(fs::File::create(path).unwrap());
    let options = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);
    for (name, bytes) in names {
        writer.start_file(name, options).unwrap();
        writer.write_all(&bytes).unwrap();
    }
    writer.finish().unwrap();
}
#[test]
fn nested_zip_needs_no_wavs_or_executables_and_preserves_saves() {
    let path = fixture("nested");
    let input = path.join("original.zip");
    zip(
        &input,
        FILES
            .iter()
            .map(|n| {
                (
                    format!("DOS/Games/TIM/{}", n.to_lowercase()),
                    fs::read(original().join(n)).unwrap(),
                )
            })
            .collect(),
    );
    let root = import(&input, &path.join("portable")).unwrap();
    assert!(FILES.iter().all(|name| root.join(name).is_file()));
    assert!(!root.join("TIM.EXE").exists());
    assert!(
        root.join("SAVES/SOLUTIONS/PUZZLE20/Default solution #1.TIM")
            .exists()
    );
    assert_eq!(verify_audio(&root).unwrap().len(), 4);
    fs::create_dir_all(root.join("SAVES")).unwrap();
    fs::write(root.join("SAVES/keep.TIM"), b"player save").unwrap();
    assert_eq!(import(&input, &path.join("portable")).unwrap(), root);
    assert_eq!(
        fs::read(root.join("SAVES/keep.TIM")).unwrap(),
        b"player save"
    );
    assert_eq!(selected_game(&path.join("portable")), Some(root));
    fs::remove_dir_all(path).unwrap();
}
#[test]
fn bad_zip_cannot_publish_or_replace_existing_game() {
    let path = fixture("bad");
    let storage = path.join("portable");
    let root = import(&original(), &storage).unwrap();
    for (name, entries) in [
        ("traversal", vec![("../RESOURCE.001".into(), vec![1])]),
        ("missing", vec![("RESOURCE.001".into(), vec![1])]),
        (
            "invalid",
            FILES.iter().map(|n| (n.to_string(), vec![1])).collect(),
        ),
    ] {
        let input = path.join(format!("{name}.zip"));
        zip(&input, entries);
        assert!(import(&input, &storage).is_err());
        assert_eq!(selected_game(&storage), Some(root.clone()));
        assert!(!storage.join("RESOURCE.001").exists());
        assert!(fs::read_dir(storage.join("games")).unwrap().all(|e| {
            !e.unwrap()
                .file_name()
                .to_string_lossy()
                .starts_with(".import")
        }));
    }
    fs::remove_dir_all(path).unwrap();
}
#[test]
fn case_insensitive_folder_import_and_relative_portable_selection() {
    let path = fixture("folder");
    let input = path.join("source");
    fs::create_dir(&input).unwrap();
    for name in FILES {
        fs::copy(original().join(name), input.join(name.to_lowercase())).unwrap();
    }
    let storage = path.join("portable");
    import(&input, &storage).unwrap();
    let moved = path.join("moved-portable");
    fs::rename(storage, &moved).unwrap();
    assert!(selected_game(&moved).unwrap().starts_with(&moved));
    assert_eq!(
        verify_audio(&selected_game(&moved).unwrap()).unwrap().len(),
        4
    );
    fs::remove_dir_all(path).unwrap();
}
