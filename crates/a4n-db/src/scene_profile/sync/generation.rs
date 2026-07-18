use sea_orm::{DatabaseConnection, TransactionTrait};

use super::super::{SceneProfileDbResult, SceneSyncRecordInput, SceneSyncRecordResult};
use super::replacement::replace_chapter_scene_sync_records;
use super::rules::{
    scene_sync_generation_response_summary, validate_scene_sync_generation_run_scope,
};

pub async fn replace_chapter_scene_sync_with_generation_run_records(
    db: &DatabaseConnection,
    project_id: i64,
    chapter_id: i64,
    chapter_number: i32,
    input: SceneSyncRecordInput,
    mut generation_run: crate::generation::RecordCompletedGenerationRunInput,
) -> SceneProfileDbResult<SceneSyncRecordResult> {
    validate_scene_sync_generation_run_scope(
        project_id,
        generation_run.project_id,
        generation_run.targets.is_empty(),
    )?;

    let txn = db.begin().await?;
    let result =
        replace_chapter_scene_sync_records(&txn, project_id, chapter_id, chapter_number, input)
            .await?;
    generation_run.targets = result
        .profile_ids
        .iter()
        .map(|id| crate::generation::GenerationRunTraceTarget {
            target_kind: "scene_profile_record".to_string(),
            target_table: "scene_profile".to_string(),
            target_id: *id,
        })
        .collect();
    if !generation_run.targets.is_empty() {
        generation_run.response_summary_json =
            scene_sync_generation_response_summary(generation_run.response_summary_json, &result);
        crate::generation::record_completed_generation_run(&txn, generation_run).await?;
    }
    txn.commit().await?;
    Ok(result)
}
