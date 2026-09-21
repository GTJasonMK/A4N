use serde_json::json;

use super::{optional_needs_part_outlines, validate_reverse_blueprint_top_level_keys};
use crate::import_reverse::application::apply::blueprint::test_support::canonical_payload;

#[test]
fn rejects_unknown_fields() {
    let mut data = canonical_payload();
    data["legacy_notes"] = json!("旧字段");

    let err = validate_reverse_blueprint_top_level_keys(&data)
        .expect_err("unknown reverse blueprint fields must fail");

    assert!(err.contains("未知字段 legacy_notes"));
}

#[test]
fn accepts_part_outline_flag() {
    let mut data = canonical_payload();
    data["needs_part_outlines"] = json!(true);

    validate_reverse_blueprint_top_level_keys(&data)
        .expect("documented needs_part_outlines field should pass top-level key validation");
}

#[test]
fn optional_part_outline_flag_rejects_non_bool_values() {
    let data = json!({"needs_part_outlines": null});

    let err = optional_needs_part_outlines(&data)
        .expect_err("needs_part_outlines must be an explicit bool when present");

    assert!(err.contains("必须是布尔值"));
}

#[test]
fn rejects_derived_fields() {
    let mut data = canonical_payload();
    data["needs_part_outlines"] = json!(false);
    data["chapters_per_part"] = json!(25);
    data["chapter_outline"] = json!([]);

    let err = validate_reverse_blueprint_top_level_keys(&data)
        .expect_err("derived reverse blueprint fields must stay system-owned");

    assert!(err.contains("未知字段"));
}
