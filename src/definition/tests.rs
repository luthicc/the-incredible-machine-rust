use super::*;
#[test]
fn archive_ids_round_trip_and_reject_out_of_range_values() {
    for id in 0..PartDefinition::COUNT as u16 {
        assert_eq!(PartDefinition::try_from(id).unwrap().id(), id);
    }
    assert!(PartDefinition::try_from(58).is_err());
    assert!(PartDefinition::try_from(u16::MAX).is_err());
}
