use a4n_core::models::source_sync::{source_sync_conflict, source_sync_run};
use a4n_domain::source_sync::SourceSyncResolutionStrategy;
use sea_orm::DatabaseTransaction;
use serde_json::{Value, json};

use crate::source_sync::{
    SourceSyncDbError, SourceSyncGalgameSceneWriteRecordInput,
    apply_galgame_scene_regeneration_record, galgame_scene_regeneration_source_record,
    validate_prepared_galgame_scene_regeneration_record,
};

pub(super) struct RegenerateSceneDecisionInput<'a> {
    pub(super) galgame_scene_id: i64,
    pub(super) reason: &'a Option<String>,
    pub(super) expected_scene_index: i32,
    pub(super) expected_source_snapshot_chapter_id: i64,
    pub(super) generated_scene: &'a SourceSyncGalgameSceneWriteRecordInput,
    pub(super) stage_outputs_json: &'a Value,
}

pub(super) async fn apply_regenerate_scene_decision(
    tx: &DatabaseTransaction,
    run: &source_sync_run::Model,
    conflict: &source_sync_conflict::Model,
    input: RegenerateSceneDecisionInput<'_>,
) -> Result<Value, SourceSyncDbError> {
    let source =
        galgame_scene_regeneration_source_record(tx, run, conflict, input.galgame_scene_id).await?;
    validate_prepared_galgame_scene_regeneration_record(
        &source,
        input.expected_scene_index,
        input.expected_source_snapshot_chapter_id,
        input.generated_scene,
    )?;
    let result = apply_galgame_scene_regeneration_record(
        tx,
        run,
        conflict.id,
        source,
        input.generated_scene.clone(),
    )
    .await?;
    Ok(json!({
        "user_action": SourceSyncResolutionStrategy::RegenerateScene.as_str(),
        "galgame_scene_id": input.galgame_scene_id,
        "reason": input.reason,
        "result": {
            "galgame_scene_id": result.galgame_scene_id,
            "old_source_snapshot_chapter_id": result.old_source_snapshot_chapter_id,
            "new_source_snapshot_chapter_id": result.new_source_snapshot_chapter_id,
            "old_line_count": result.old_line_count,
            "new_line_count": result.new_line_count,
            "deleted_generation_spec_count": result.deleted_generation_spec_count,
            "deleted_portrait_generation_spec_count": result.deleted_portrait_generation_spec_count,
            "resolved_related_conflict_count": result.resolved_related_conflict_count,
            "mode": "llm_partial_regeneration",
            "stage_outputs": input.stage_outputs_json,
        },
    }))
}
