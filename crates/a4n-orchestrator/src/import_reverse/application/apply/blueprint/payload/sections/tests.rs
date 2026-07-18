use super::validate_reverse_blueprint_sections;
use crate::import_reverse::application::apply::blueprint::test_support::canonical_payload;

#[test]
fn accepts_canonical_payload() {
    let data = canonical_payload();

    validate_reverse_blueprint_sections(&data, data.get("world_setting").unwrap())
        .expect("canonical reverse blueprint sections should pass");
}

#[test]
fn rejects_thin_character_payload() {
    let mut data = canonical_payload();
    data["characters"][0]
        .as_object_mut()
        .unwrap()
        .remove("arc_stages");

    let err = validate_reverse_blueprint_sections(&data, data.get("world_setting").unwrap())
        .expect_err("missing arc_stages must fail");

    assert!(err.contains("arc_stages"));
}

#[test]
fn rejects_missing_relationship_strength() {
    let mut data = canonical_payload();
    data["relationships"][0]
        .as_object_mut()
        .unwrap()
        .remove("strength");

    let err = validate_reverse_blueprint_sections(&data, data.get("world_setting").unwrap())
        .expect_err("missing strength must fail");

    assert!(err.contains("strength"));
}
