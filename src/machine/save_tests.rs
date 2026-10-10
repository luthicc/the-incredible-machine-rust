use super::*;

#[test]
fn rope_curve_matches_native_threshold_and_quadratic() {
    let a = (10, 20);
    let b = (110, 60);
    for slack in [-10, 0, 4] {
        assert_eq!(curved_rope_segments(a, b, slack), vec![(a, b)]);
    }
    let curve = curved_rope_segments(a, b, 10);
    assert_eq!(curved_rope_segments(a, b, 5).len(), 16);
    assert_eq!(curve.len(), 16);
    assert_eq!(curve[0].0, a);
    assert_eq!(curve[7].1, (60, 45));
    assert_eq!(curve[15].1, b);
    assert_eq!(linked_point_distance((0, 0), (100, 40)), 115);
}
