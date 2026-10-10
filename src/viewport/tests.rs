use super::*;
#[test]
fn native_pixels_ignore_stretch_and_map_centered_pointer_at_each_dpi() {
    for dpi in [1.0, 2.0] {
        let v = Viewport::presentation((1600.0, 900.0), dpi, (640, 480), true, false);
        assert_eq!(v.scale * dpi, 1.0);
        assert_eq!(v.scale_y * dpi, 1.0);
        assert_eq!(v.prescale, 1);
        assert_eq!(v.source_point((800.0, 450.0)), Some((320.0, 240.0)));
        assert_eq!(v.source_point((v.origin.0 - 1.0, v.origin.1)), None);
    }
}
#[test]
fn fills_the_window_and_maps_the_pointer() {
    let v = Viewport::fit((960.0, 720.0), 1.0, (640, 480));
    assert_eq!((v.scale, v.origin, v.prescale), (1.5, (0.0, 0.0), 2));
    assert_eq!(v.source_point((324.0, 144.0)), Some((216.0, 96.0)));
    assert_eq!(v.source_point((960.0, 600.0)), None);
    // Wider than 4:3: pillarboxed, centred.
    let wide = Viewport::fit((1600.0, 900.0), 1.0, (640, 480));
    assert_eq!((wide.scale, wide.origin), (1.875, (200.0, 0.0)));
    assert_eq!(wide.source_point((199.0, 10.0)), None);
    let exact = Viewport::fit((1280.0, 960.0), 1.0, (640, 480));
    assert_eq!((exact.scale, exact.prescale), (2.0, 2));
}
#[test]
fn dpi_small_and_minimized_windows() {
    let v = Viewport::fit((960.0, 720.0), 2.0, (640, 480));
    assert_eq!((v.scale, v.prescale), (1.5, 3));
    assert_eq!(v.origin, (0.0, 0.0));
    let small = Viewport::fit((320.0, 240.0), 1.0, (640, 480));
    assert_eq!((small.scale, small.prescale), (0.5, 1));
    assert_eq!(
        Viewport::fit((0.0, 0.0), 1.0, (640, 480)).source_point((0.0, 0.0)),
        None
    );
}
#[test]
fn stretched_viewport_maps_each_axis_to_the_full_window() {
    let v = Viewport::configured((1600.0, 900.0), 1.0, (640, 480), true);
    assert_eq!((v.scale, v.scale_y, v.origin), (2.5, 1.875, (0.0, 0.0)));
    assert_eq!(v.source_point((800.0, 450.0)), Some((320.0, 240.0)));
    assert_eq!(v.source_point((1597.5, 898.125)), Some((639.0, 479.0)));
}
#[test]
fn upscale_repeats_pixels() {
    let rgba = [1, 1, 1, 1, 2, 2, 2, 2];
    let out = upscale(&rgba, 2, 2);
    assert_eq!(out.len(), 32);
    assert_eq!(
        &out[..16],
        &[1, 1, 1, 1, 1, 1, 1, 1, 2, 2, 2, 2, 2, 2, 2, 2]
    );
    assert_eq!(&out[16..], &out[..16]);
}
#[test]
fn region_zoom_keeps_the_point_fixed_stays_inside_and_eases_out() {
    let mut zoom = Zoom::new((8.0, 8.0, 568.0, 360.0));
    assert_eq!(zoom.map((100.0, 100.0)), (100.0, 100.0));
    zoom.zoom_at((100.0, 100.0), 2.0);
    let (x, y) = zoom.map((100.0, 100.0));
    assert!((x - 100.0).abs() < 1e-3 && (y - 100.0).abs() < 1e-3);
    // Outside the region nothing moves.
    assert_eq!(zoom.map((600.0, 400.0)), (600.0, 400.0));
    // Panning stops at the region edge.
    zoom.pan((10_000.0, 10_000.0));
    assert_eq!(zoom.map((8.0, 8.0)), (8.0, 8.0));
    for _ in 0..200 {
        zoom.ease_out(0.9);
    }
    assert_eq!(zoom, Zoom::new((8.0, 8.0, 568.0, 360.0)));
}
