use super::*;

#[test]
fn controls_and_meters_follow_the_native_elements() {
    assert_eq!(control_at((0x40, 0x60)), Some(Control::Play));
    assert_eq!(control_at((0x150, 0x80)), Some(Control::Play));
    assert_eq!(control_at((0xe0, 0x68)), Some(Control::Quit));
    assert_eq!(control_at((0x60, 0x60)), Some(Control::VolumeUp));
    assert_eq!(control_at((0x60, 0x75)), Some(Control::VolumeDown));
    assert_eq!(control_at((0x80, 220)), Some(Control::TopMeter));
    assert_eq!(meter_value(0x43, 0x200), 0);
    assert_eq!(meter_value(0x43 + 0xa0, 0x200), 0x200);
    assert_eq!(meter_value(0x43 + 0x50, 0x80), 0x40);
    assert_eq!(SelectPuzzle::new(30).first, 22);
    let select = SelectPuzzle::new(1);
    assert_eq!(select.hit((0x40, 0x4c + 25)), Some(SelectHit::Entry(3)));
    let mut name = String::new();
    edit_text(&mut name, "my mach\u{8}", 13);
    assert_eq!(name, "MY MAC");
    let dialog = FileDialog {
        name: "my.mac".into(),
        ..FileDialog::open(FileMode::Save, std::path::Path::new("/nonexistent"))
    };
    assert_eq!(dialog.file_name().as_deref(), Some("MY.TIM"));
}
