#[test]
fn rejects_unknown_top_level_blueprint_field() {
    let mut value: serde_json::Value = serde_json::from_str(&valid_blueprint()).unwrap();
    value["legacy_notes"] = json!("旧字段");

    let err = parse_blueprint_json(&value.to_string(), 30)
        .expect_err("unknown top-level fields must fail");

    assert!(err.to_string().contains("未知字段 legacy_notes"));
}

#[test]
fn rejects_unknown_character_field() {
    let mut value: serde_json::Value = serde_json::from_str(&valid_blueprint()).unwrap();
    value["characters"][0]["power_tags"] = json!(["记忆"]);

    let err = parse_blueprint_json(&value.to_string(), 30)
        .expect_err("unknown character fields must fail");

    assert!(err.to_string().contains("未知字段 power_tags"));
}

#[test]
fn rejects_old_relationship_alias_fields() {
    let raw = valid_blueprint().replace("\"character_a\":\"林澈\"", "\"character_from\":\"林澈\"");

    assert!(parse_blueprint_json(&raw, 30).is_err());
}

#[test]
fn rejects_unknown_world_setting_nested_field() {
    let mut value: serde_json::Value = serde_json::from_str(&valid_blueprint()).unwrap();
    value["world_setting"]["key_locations"][0]["legacy_function"] = json!("旧字段");

    let err = parse_blueprint_json(&value.to_string(), 30)
        .expect_err("unknown world setting fields must fail");

    assert!(err.to_string().contains("未知字段 legacy_function"));
}

#[test]
fn rejects_thin_world_setting_schema() {
    let raw = valid_blueprint().replace(
        r#""premise":"星潮灾变后，记忆成为可被封存、审校和买卖的公共资源。","#,
        "",
    );

    assert!(parse_blueprint_json(&raw, 30).is_err());
}

#[test]
fn forward_blueprint_still_requires_complete_world_arrays() {
    let mut value: serde_json::Value = serde_json::from_str(&valid_blueprint()).unwrap();
    value["world_setting"]["factions"] = json!([]);

    let err = parse_blueprint_json(&value.to_string(), 30)
        .expect_err("forward blueprint must keep the complete-world contract");

    assert!(err.to_string().contains("world_setting.factions"));
}

#[test]
fn rejects_character_without_rich_arc_stages() {
    let raw =
        valid_blueprint().replace(r#""core_wound":"妹妹失踪时自己选择遵守流程而非救人","#, "");

    assert!(parse_blueprint_json(&raw, 30).is_err());
}

#[test]
fn rejects_character_without_canonical_identity_fields() {
    let mut value: serde_json::Value = serde_json::from_str(&valid_blueprint()).unwrap();
    let character = value["characters"][1].as_object_mut().unwrap();
    character.remove("gender");
    character.remove("age");
    let raw = value.to_string();

    let err = parse_blueprint_json(&raw, 30).expect_err("missing gender and age must fail");

    assert!(err.to_string().contains("gender"));
}

#[test]
fn rejects_relationship_without_dynamic_evolution() {
    let raw = valid_blueprint().replace(r#""story_function":"把个人寻亲线推向制度真相线","#, "");

    assert!(parse_blueprint_json(&raw, 30).is_err());
}

#[test]
fn rejects_relationship_without_strength() {
    let mut value: serde_json::Value = serde_json::from_str(&valid_blueprint()).unwrap();
    value["relationships"][0]
        .as_object_mut()
        .unwrap()
        .remove("strength");
    let raw = value.to_string();

    let err = parse_blueprint_json(&raw, 30).expect_err("missing strength must fail");

    assert!(err.to_string().contains("strength"));
}

#[test]
fn rejects_noncanonical_relationship_type() {
    let raw = valid_blueprint().replace("\"relationship_type\":\"rival\"", "\"relationship_type\":\"rivalry\"");

    let err = parse_blueprint_json(&raw, 30)
        .expect_err("relationship_type outside the canonical enum must fail");

    assert!(err.to_string().contains("rivalry"));
    assert!(err.to_string().contains("family, friend, enemy, lover, rival, mentor, other"));
}
