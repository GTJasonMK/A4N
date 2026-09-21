use super::validate_reverse_blueprint_sections;
use crate::import_reverse::application::apply::blueprint::test_support::canonical_payload;

#[test]
fn accepts_canonical_payload() {
    let data = canonical_payload();

    validate_reverse_blueprint_sections(&data, data.get("world_setting").unwrap())
        .expect("canonical reverse blueprint sections should pass");
}

#[test]
fn accepts_sparse_existing_materials_without_inventing_entities() {
    let mut data = canonical_payload();
    data["world_setting"]["key_locations"] = serde_json::json!([]);
    data["world_setting"]["factions"] = serde_json::json!([]);
    data["world_setting"]["history_timeline"] = serde_json::json!([]);
    data["world_setting"]["mysteries_and_foreshadowing"] = serde_json::json!([]);
    data["characters"].as_array_mut().unwrap().truncate(2);
    data["relationships"] = serde_json::json!([]);

    validate_reverse_blueprint_sections(&data, data.get("world_setting").unwrap())
        .expect("existing materials may contain fewer than three known characters");
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
