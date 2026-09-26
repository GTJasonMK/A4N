use a4n_domain::generation::{GenerationRunTraceTarget, RecordCompletedGenerationRunInput};
use serde_json::{Map, Value, json};

use super::super::CharacterSyncWriteSummary;
use crate::character::CharacterDbError;

pub(in crate::character::sync_persistence) fn enrich_character_sync_generation_run(
    generation_run: &mut RecordCompletedGenerationRunInput,
    project_id: i64,
    profile_id: i64,
    write_summary: &CharacterSyncWriteSummary,
    state_index_count: usize,
) -> Result<(), CharacterDbError> {
    if generation_run.project_id != project_id {
        return Err(CharacterDbError::InvalidArgument(
            "角色同步 generation run project_id 与项目不一致".to_string(),
        ));
    }
    if !generation_run.targets.is_empty() {
        return Err(CharacterDbError::InvalidArgument(
            "角色同步 generation run 目标由 DB 层绑定，调用方不能预填 targets".to_string(),
        ));
    }
    generation_run.targets = vec![GenerationRunTraceTarget {
        target_kind: "character_profile_record".to_string(),
        target_table: "protagonist_profile".to_string(),
        target_id: profile_id,
    }];

    let mut response_summary = generation_run
        .response_summary_json
        .take()
        .unwrap_or_else(|| Value::Object(Map::new()));
    let object = response_summary.as_object_mut().ok_or_else(|| {
        CharacterDbError::InvalidArgument(
            "角色同步 generation run response_summary_json 必须是 JSON 对象".to_string(),
        )
    })?;
    object.insert(
        "changes_applied".to_string(),
        json!(write_summary.changes_applied),
    );
    object.insert(
        "behaviors_recorded".to_string(),
        json!(write_summary.behaviors_recorded),
    );
    object.insert(
        "deletions_marked".to_string(),
        json!(write_summary.deletions_marked),
    );
    object.insert(
        "deletions_executed".to_string(),
        json!(write_summary.deletions_executed),
    );
    object.insert(
        "protected_attributes".to_string(),
        json!(write_summary.protected_attributes),
    );
    object.insert(
        "state_indexes_upserted".to_string(),
        json!(state_index_count),
    );
    generation_run.response_summary_json = Some(response_summary);
    Ok(())
}
