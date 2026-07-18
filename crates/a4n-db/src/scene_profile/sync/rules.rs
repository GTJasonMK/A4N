use serde_json::{Value, json};

use super::super::{SceneProfileDbError, SceneProfileDbResult, SceneSyncRecordResult};

pub(super) fn validate_scene_sync_generation_run_scope(
    project_id: i64,
    generation_run_project_id: i64,
    targets_are_empty: bool,
) -> SceneProfileDbResult<()> {
    if generation_run_project_id != project_id {
        return Err(SceneProfileDbError::InvalidArgument(
            "场景同步生成运行记录 project_id 与项目不一致".to_string(),
        ));
    }
    if !targets_are_empty {
        return Err(SceneProfileDbError::InvalidArgument(
            "场景同步生成运行记录目标由场景服务绑定，调用方不能预填 targets".to_string(),
        ));
    }
    Ok(())
}

pub(super) fn scene_sync_generation_response_summary(
    summary: Option<Value>,
    result: &SceneSyncRecordResult,
) -> Option<Value> {
    let generated = json!({
        "profile_ids": result.profile_ids,
        "occurrence_count": result.occurrence_count,
        "transition_count": result.transition_count,
        "entity_state_count": result.entity_state_count,
    });
    match summary {
        Some(Value::Object(mut map)) => {
            if let Some(generated) = generated.as_object() {
                for (key, value) in generated {
                    map.insert(key.clone(), value.clone());
                }
            }
            Some(Value::Object(map))
        }
        Some(value) => Some(json!({
            "value": value,
            "scene_sync": generated,
        })),
        None => Some(generated),
    }
}
