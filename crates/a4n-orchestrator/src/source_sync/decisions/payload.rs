use a4n_core::error::AppError;
use serde::de::DeserializeOwned;
use serde_json::Value;

use crate::galgame::GalgamePersistScene;

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
