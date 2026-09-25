use std::fmt::Display;

use a4n_domain::project::ChapterDerivedCleanupArtifacts;
use a4n_error::AppError;
use sea_orm::DatabaseTransaction;

pub async fn cleanup_chapter_derived_data_in(
    db: &DatabaseTransaction,
    project_id: i64,
    chapter_id: i64,
    chapter_number: i32,
    preserve_chapter_analysis_workflow_run_id: Option<i64>,
) -> Result<ChapterDerivedCleanupArtifacts, AppError> {
    let changed =
        a4n_db::fact_feedback::invalidate_feedback_sources(db, project_id, chapter_number)
            .await
            .map_err(|error| owner_error("撤回失效正文事实", error))?;
    let mut artifacts = ChapterDerivedCleanupArtifacts {
        blueprint_facts_changed: changed.blueprint_changed,
        character_facts_changed: changed.characters_changed,
        changed_foreshadowing_ids: changed.foreshadowing_ids,
        ..Default::default()
    };
    a4n_db::chapter_visual_plan::invalidate_live_chapter_visual_plans_for_source_change(
        db, project_id, chapter_id,
    )
    .await
    .map_err(|error| owner_error("失效章节视觉方案", error))?;
    a4n_db::text_anchor::delete_chapter_content_profile_evidence_records(
        db, project_id, chapter_id,
    )
    .await
    .map_err(|error| owner_error("清理章节正文证据", error))?;
    a4n_db::project::delete_automatic_chapter_outlines_for_source_change(
        db,
        project_id,
        chapter_number,
    )
    .await
    .map_err(AppError::Database)?;

    let profiles = a4n_db::project::protagonist_profiles_for_project_in(db, project_id)
        .await
        .map_err(|error| owner_error("读取角色档案", error))?;
    // Profile records are also indexed; event deletion alone leaves their old current state searchable.
    artifacts.character_facts_changed |= !profiles.is_empty();
    for profile in profiles {
        let deleted = a4n_db::character::retract_profile_chapter_facts(
            db,
            project_id,
            profile.id,
            chapter_number,
        )
        .await
        .map_err(|error| owner_error("撤回角色章节事实", error))?;
        artifacts
            .stale_character_state_ids
            .extend(deleted.state_index_ids);
        artifacts
            .stale_attribute_event_ids
            .extend(deleted.attribute_event_ids);
    }
    artifacts.scene_records_changed =
        a4n_db::scene_profile::delete_chapter_scene_sync_records(db, project_id, chapter_id, false)
            .await
            .map_err(|error| owner_error("清理章节场景档案", error))?;
    a4n_db::workflow::invalidate_chapter_analysis_after_source_change(
        db,
        project_id,
        chapter_number,
        preserve_chapter_analysis_workflow_run_id,
    )
    .await
    .map_err(|error| owner_error("失效后续章节分析派生 checkpoint", error))?;
    Ok(artifacts)
}

fn owner_error(operation: &str, error: impl Display) -> AppError {
    AppError::InvalidState {
        current: error.to_string(),
        operation: operation.to_string(),
    }
}
