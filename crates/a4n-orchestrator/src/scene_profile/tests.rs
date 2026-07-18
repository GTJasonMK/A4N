use super::{
    normalize_movement_type_value, normalize_scene_entity_state_key_value,
    normalize_scene_entity_type_value, normalize_scene_type_value,
};
use a4n_core::error::AppError;

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
