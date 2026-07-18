use serde_json::json;

use super::super::{SceneProfileDbError, SceneSyncRecordResult};
use super::rules::{
    scene_sync_generation_response_summary, validate_scene_sync_generation_run_scope,
};

fn result() -> SceneSyncRecordResult {
    SceneSyncRecordResult {
        profile_ids: vec![3, 5],
        profile_ref_ids: [("scene-a".to_string(), 3), ("scene-b".to_string(), 5)]
            .into_iter()
            .collect(),
        occurrence_count: 2,
        transition_count: 1,
        entity_state_count: 4,
    }
}

#[test]
fn generation_run_scope_rejects_project_mismatch_before_prefilled_targets() {
    assert!(matches!(
        validate_scene_sync_generation_run_scope(7, 8, false),
        Err(SceneProfileDbError::InvalidArgument(message))
            if message == "场景同步生成运行记录 project_id 与项目不一致"
    ));
    assert!(matches!(
        validate_scene_sync_generation_run_scope(7, 7, false),
        Err(SceneProfileDbError::InvalidArgument(message))
            if message == "场景同步生成运行记录目标由场景服务绑定，调用方不能预填 targets"
    ));
    assert!(validate_scene_sync_generation_run_scope(7, 7, true).is_ok());
}

#[test]
fn generation_response_summary_merges_object_fields() {
    let summary = scene_sync_generation_response_summary(
        Some(json!({"existing": true, "profile_ids": [99]})),
        &result(),
    );

    assert_eq!(
        summary,
        Some(json!({
            "existing": true,
            "profile_ids": [3, 5],
            "occurrence_count": 2,
            "transition_count": 1,
            "entity_state_count": 4,
        }))
    );
}

#[test]
fn generation_response_summary_wraps_non_object_and_builds_missing_summary() {
    assert_eq!(
        scene_sync_generation_response_summary(Some(json!(["legacy"])), &result()),
        Some(json!({
            "value": ["legacy"],
            "scene_sync": {
                "profile_ids": [3, 5],
                "occurrence_count": 2,
                "transition_count": 1,
                "entity_state_count": 4,
            }
        }))
    );
    assert_eq!(
        scene_sync_generation_response_summary(None, &result()),
        Some(json!({
            "profile_ids": [3, 5],
            "occurrence_count": 2,
            "transition_count": 1,
            "entity_state_count": 4,
        }))
    );
}
