use a4n_core::error::AppError;
#[cfg(test)]
use a4n_core::models::source_sync::source_sync_conflict;
use serde::de::DeserializeOwned;
use serde_json::Value;

use crate::galgame::GalgamePersistScene;

#[cfg(test)]
pub(in crate::source_sync) fn ensure_decision_targets_generation_spec(
    conflict: &source_sync_conflict::Model,
    generation_spec_id: i64,
    strategy: &str,
) -> Result<(), AppError> {
    let expected = generation_spec_id_for_conflict(conflict, strategy)?;
    if generation_spec_id == expected {
        Ok(())
    } else {
        Err(AppError::InvalidArgument(format!(
            "{strategy} 决策目标生成意图不匹配：冲突 {} 影响 generation_spec={}，payload 指向 generation_spec={}",
            conflict.id, expected, generation_spec_id
        )))
    }
}

#[cfg(test)]
pub(in crate::source_sync) fn generation_spec_id_for_conflict(
    conflict: &source_sync_conflict::Model,
    strategy: &str,
) -> Result<i64, AppError> {
    if conflict.conflict_scope == "generation_spec" {
        return Ok(conflict.target_id);
    }
    if conflict.conflict_scope == "text_anchor" && conflict.target_kind == "segment" {
        return Ok(conflict.target_id);
    }
    if let Some(generation_spec_id) = conflict
        .local_summary_json
        .get("generation_spec_id")
        .and_then(Value::as_i64)
        .filter(|id| *id > 0)
    {
        return Ok(generation_spec_id);
    }
    Err(AppError::InvalidArgument(format!(
        "{strategy} 决策要求冲突绑定生成意图，但冲突 {} 未提供 generation_spec_id",
        conflict.id
    )))
}

pub(in crate::source_sync) fn parse_decision_payload<T>(value: &Value) -> Result<T, AppError>
where
    T: DeserializeOwned,
{
    serde_json::from_value(value.clone())
        .map_err(|err| AppError::InvalidArgument(format!("同步决策 payload 无效: {err}")))
}

pub(in crate::source_sync) fn source_sync_galgame_scene_write_record(
    scene: GalgamePersistScene,
) -> a4n_db::source_sync::SourceSyncGalgameSceneWriteRecordInput {
    a4n_db::source_sync::SourceSyncGalgameSceneWriteRecordInput {
        title: scene.title,
        location: scene.location,
        time_of_day: scene.time_of_day,
        participants_json: scene.participants_json,
        background_note: scene.background_note,
        lines: scene
            .lines
            .into_iter()
            .map(
                |line| a4n_db::source_sync::SourceSyncGalgameSceneLineWriteRecordInput {
                    line_index: line.line_index,
                    line_type: line.line_type,
                    speaker_profile_id: line.speaker_profile_id,
                    text: line.text,
                    expression: line.expression,
                    pose: line.pose,
                    position: line.position,
                    presentation_json: line.presentation_json,
                    text_anchor_id: line.text_anchor_id,
                },
            )
            .collect(),
    }
}
