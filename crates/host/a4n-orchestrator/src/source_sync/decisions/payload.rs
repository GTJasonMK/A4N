use a4n_error::AppError;
use serde::de::DeserializeOwned;
use serde_json::{Map, Value};

use crate::galgame::GalgamePersistScene;

pub(in crate::source_sync) fn parse_decision_payload<T>(
    value: &Map<String, Value>,
) -> Result<T, AppError>
where
    T: DeserializeOwned,
{
    serde_json::from_value(Value::Object(value.clone()))
        .map_err(|err| AppError::InvalidArgument(format!("同步决策 payload 无效: {err}")))
}

pub(in crate::source_sync) fn source_sync_galgame_scene_write_record(
    scene: GalgamePersistScene,
) -> Result<a4n_db::source_sync::SourceSyncGalgameSceneWriteRecordInput, AppError> {
    let source = scene
        .adaptation_source
        .ok_or_else(|| AppError::InvalidArgument("源同步场景缺少本地改编来源".into()))?;
    Ok(
        a4n_db::source_sync::SourceSyncGalgameSceneWriteRecordInput {
            adaptation_plan_id: source.plan_id,
            adaptation_plan_revision: source.plan_revision,
            adaptation_unit_id: source.unit_id,
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
        },
    )
}
