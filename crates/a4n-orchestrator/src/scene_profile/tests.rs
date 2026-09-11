use super::{
    normalize_movement_type_value, normalize_scene_entity_state_key_value,
    normalize_scene_entity_type_value, normalize_scene_profile_record_input,
    normalize_scene_type_value,
};
use a4n_api_contract::profile::SceneProfileInput;
use a4n_error::AppError;
use serde_json::json;

fn valid_scene_profile_input() -> SceneProfileInput {
    SceneProfileInput {
        name: "旧车站".to_string(),
        aliases: vec![],
        scene_type: "building".to_string(),
        importance: "primary".to_string(),
        parent_scene_id: None,
        base_description: "停运车站".to_string(),
        visual_description: "昏暗站厅".to_string(),
        spatial_layout: "站厅连接通道".to_string(),
        atmosphere: "紧张".to_string(),
        entry_points_json: Some(json!(["正门"])),
        rules_json: Some(json!(["午夜关闭"])),
        first_seen_chapter: 1,
        last_seen_chapter: 1,
        evidence: vec!["林岚进入旧车站".to_string()],
        image_prompt_seed: "雨夜旧车站".to_string(),
    }
}

#[test]
fn scene_profile_rejects_non_array_entry_points_and_rules() {
    let mut invalid_entry_points = valid_scene_profile_input();
    invalid_entry_points.entry_points_json = Some(json!({"name": "正门"}));
    let error = normalize_scene_profile_record_input(invalid_entry_points)
        .expect_err("non-array entry points must remain visible");
    assert!(matches!(error, AppError::InvalidArgument(message) if message.contains("场景出入口")));

    let mut invalid_rules = valid_scene_profile_input();
    invalid_rules.rules_json = Some(json!({"access": "需机械钥匙"}));
    let error = normalize_scene_profile_record_input(invalid_rules)
        .expect_err("non-array rules must remain visible");
    assert!(matches!(error, AppError::InvalidArgument(message) if message.contains("场景规则")));
}

#[test]
fn manual_scene_profile_allows_name_only() {
    let mut input = valid_scene_profile_input();
    input.scene_type.clear();
    input.importance.clear();
    input.base_description.clear();
    input.visual_description.clear();
    input.spatial_layout.clear();
    input.atmosphere.clear();
    input.entry_points_json = None;
    input.rules_json = None;
    input.evidence.clear();
    input.image_prompt_seed.clear();

    let input = normalize_scene_profile_record_input(input)
        .expect("manual profile should accept sparse known data");
    assert_eq!(input.name, "旧车站");
    assert!(input.scene_type.is_empty());
    assert!(
        input
            .entry_points_json
            .as_array()
            .is_some_and(Vec::is_empty)
    );
    assert!(input.rules_json.as_array().is_some_and(Vec::is_empty));
    assert!(input.evidence_json.as_array().is_some_and(Vec::is_empty));
}

#[test]
fn normalize_scene_type_accepts_canonical_values_only() {
    assert_eq!(normalize_scene_type_value(" room ").unwrap(), "room");
    assert_eq!(normalize_scene_type_value("route").unwrap(), "route");
    for alias in ["corridor", "hallway", "classroom", "医院", "荒野"] {
        let err =
            normalize_scene_type_value(alias).expect_err("scene type aliases must remain visible");
        assert!(
            matches!(err, AppError::InvalidArgument(message) if message.contains("场景类型无效")),
            "{alias} should be rejected"
        );
    }
}

#[test]
fn normalize_scene_type_rejects_unknown_values() {
    let err =
        normalize_scene_type_value("memory").expect_err("unknown scene type should remain visible");
    assert!(matches!(err, AppError::InvalidArgument(message) if message.contains("场景类型无效")));
}

#[test]
fn normalize_movement_type_accepts_canonical_values_only() {
    assert_eq!(normalize_movement_type_value(" travel ").unwrap(), "travel");
    assert_eq!(
        normalize_movement_type_value("hidden_passage").unwrap(),
        "hidden_passage"
    );
    for alias in ["move", "walk", "switch", "secret-passage", "进入"] {
        let err = normalize_movement_type_value(alias)
            .expect_err("movement type aliases must remain visible");
        assert!(
            matches!(err, AppError::InvalidArgument(message) if message.contains("位移类型无效")),
            "{alias} should be rejected"
        );
    }
}

#[test]
fn normalize_movement_type_rejects_unknown_values() {
    let err = normalize_movement_type_value("dream_merge")
        .expect_err("unknown movement type should remain visible");
    assert!(matches!(err, AppError::InvalidArgument(message) if message.contains("位移类型无效")));
}

#[test]
fn normalize_scene_entity_type_accepts_canonical_values_only() {
    assert_eq!(
        normalize_scene_entity_type_value(" character ").unwrap(),
        "character"
    );
    assert_eq!(
        normalize_scene_entity_type_value("object").unwrap(),
        "object"
    );
    for alias in ["person", "item", "plot-event", "角色", "道具"] {
        let err = normalize_scene_entity_type_value(alias)
            .expect_err("entity type aliases must remain visible");
        assert!(
            matches!(err, AppError::InvalidArgument(message) if message.contains("实体类型无效")),
            "{alias} should be rejected"
        );
    }
}

#[test]
fn normalize_scene_entity_type_rejects_unknown_values() {
    let err = normalize_scene_entity_type_value("memory")
        .expect_err("unknown entity type should remain visible");
    assert!(matches!(err, AppError::InvalidArgument(message) if message.contains("实体类型无效")));
}

#[test]
fn normalize_scene_entity_state_key_accepts_canonical_values_only() {
    assert_eq!(
        normalize_scene_entity_state_key_value(" position ").unwrap(),
        "position"
    );
    assert_eq!(
        normalize_scene_entity_state_key_value("event_progress").unwrap(),
        "event_progress"
    );
    for alias in [
        "location",
        "held-by",
        "status",
        "movement",
        "relationship",
        "plot-progress",
        "位置",
    ] {
        let err = normalize_scene_entity_state_key_value(alias)
            .expect_err("entity state key aliases must remain visible");
        assert!(
            matches!(err, AppError::InvalidArgument(message) if message.contains("实体状态键无效")),
            "{alias} should be rejected"
        );
    }
}

#[test]
fn normalize_scene_entity_state_key_rejects_unknown_values() {
    let err = normalize_scene_entity_state_key_value("memory")
        .expect_err("unknown entity state key should remain visible");
    assert!(
        matches!(err, AppError::InvalidArgument(message) if message.contains("实体状态键无效"))
    );
}
