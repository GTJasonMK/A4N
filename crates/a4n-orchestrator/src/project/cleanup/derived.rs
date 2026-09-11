use a4n_domain::project::ChapterDerivedCleanupArtifacts;
use a4n_error::AppError;
use sea_orm::{ConnectionTrait, TransactionTrait};

use super::cleanup_character_profile_timeline_for_chapter_in;

pub(crate) async fn cleanup_chapter_derived_data_in<C>(
    db: &C,
    project_id: i64,
    chapter_id: i64,
    chapter_number: i32,
    cleanup_foreshadowing: bool,
    prune_ai_scene_profiles: bool,
) -> Result<ChapterDerivedCleanupArtifacts, AppError>
where
    C: ConnectionTrait + TransactionTrait,
{
    let mut artifacts = ChapterDerivedCleanupArtifacts::default();
    a4n_db::chapter_visual_plan::invalidate_live_chapter_visual_plans_for_source_change(
        db, project_id, chapter_id,
    )
    .await
    .map_err(crate::chapter_visual_plan::visual_plan_db_error)?;
    a4n_db::text_anchor::delete_chapter_content_profile_evidence_records(
        db, project_id, chapter_id,
    )
    .await
    .map_err(crate::text_anchor::text_anchor_db_error)?;
    a4n_db::project::delete_automatic_chapter_outlines_for_source_change(
        db,
        project_id,
        chapter_number,
    )
    .await
    .map_err(AppError::Database)?;
    let character_artifacts =
        cleanup_character_profile_timeline_for_chapter_in(db, project_id, chapter_number).await?;
    artifacts
        .stale_profile_ids
        .extend(character_artifacts.stale_profile_ids);
    artifacts
        .stale_character_state_ids
        .extend(character_artifacts.stale_character_state_ids);
    artifacts
        .stale_attribute_event_ids
        .extend(character_artifacts.stale_attribute_event_ids);
    artifacts
        .media_file_paths
        .extend(character_artifacts.media_file_paths);

    artifacts.scene_records_changed = a4n_db::scene_profile::delete_chapter_scene_sync_records(
        db,
        project_id,
        chapter_id,
        prune_ai_scene_profiles,
    )
    .await
    .map_err(crate::scene_profile::scene_profile_db_error)?;

    if cleanup_foreshadowing {
        let changed_foreshadowing_ids =
            a4n_db::project::cleanup_foreshadowing_for_chapter(db, project_id, chapter_number)
                .await
                .map_err(crate::project::foreshadowing_db_error_to_app_error)?;
        artifacts
            .changed_foreshadowing_ids
            .extend(changed_foreshadowing_ids);
    }
    Ok(artifacts)
}
