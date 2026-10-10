use super::*;

#[test]
fn every_definition_has_ported_update_coverage() {
    let mut unimplemented = 0;
    for id in 0..Part::COUNT as u16 {
        let part = Part::try_from(id).unwrap();
        let coverage = update_coverage(part);
        if matches!(coverage, UpdateCoverage::Unimplemented(_)) {
            unimplemented += 1;
        }
        assert!(
            !matches!(coverage, UpdateCoverage::Absent),
            "definition {id} has a native update slot"
        );
    }
    // The monkey (272c:2d40) was the last native update body ported.
    assert_eq!(unimplemented, 0, "a native update callback is not ported");
    assert_eq!(
        update_coverage(Part::BowlingBall),
        UpdateCoverage::NativeNoOp
    );
    assert_eq!(
        update_coverage(Part::Seesaw),
        UpdateCoverage::Partial(NativeCallback {
            segment: 0x272c,
            offset: 0x420f
        })
    );
}
