//! 项目生成态重置与状态回退清理。

use a4n_db_models::models::project::novel_project;
use a4n_domain::project::{CleanupAction, ProjectKind, ProjectStatus, rollback_actions};
use chrono::{NaiveDateTime, Utc};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, DbErr, EntityTrait, PaginatorTrait,
    QueryFilter, Set, TransactionTrait,
};

use crate::character;
use crate::generation;
use crate::project::delete::{
    GeneratedImageDeleteScope, delete_chapter_generation_outputs, delete_project_profile_scene_data,
};
use crate::project::{self as project_db, PROJECT_KIND_TEXT, ProjectTextAnchorDeleteScope};

mod types;
pub use types::{ProjectResetDbError, ProjectStatusRollbackOutcome};

mod galgame;
use galgame::reset_galgame_generated_data_in_tx;

mod characters;
use characters::reset_character_profiles_for_project;

mod rollback;
pub use rollback::rollback_project_status_record;

#[cfg(test)]
mod tests;

use a4n_domain::project::ProjectStatusHistoryInput;
use a4n_domain::project_reset::{
    ProjectGeneratedDataResetOutcome, ProjectGeneratedDataResetResult,
};

/// 重置项目生成态（含事务与状态写回）。事件发布仍由调用方负责。
pub async fn reset_project_generated_data_record(
    db: &DatabaseConnection,
    project_id: i64,
) -> Result<ProjectGeneratedDataResetOutcome, ProjectResetDbError> {
    use a4n_db_models::models::chapter::{chapter, chapter_outline, part_outline};
    use a4n_db_models::models::character::foreshadowing_index;
    use a4n_db_models::models::project::{blueprint_version, import_review_queue};

    let tx = db.begin().await?;
    let project = project_db::project_by_id_for_update(&tx, project_id)
        .await?
        .ok_or(ProjectResetDbError::ProjectNotFound { project_id })?;
    crate::generation::require_project_lifecycle_gate(
        &tx,
        project_id,
        crate::generation::PROJECT_GATE_RESETTING,
    )
    .await
    .map_err(|error| ProjectResetDbError::Database {
        source: sea_orm::DbErr::Custom(error.to_string()),
    })?;

    if project.project_kind == ProjectKind::Galgame.as_str() {
        let candidate_media_paths = project_db::project_generated_media_paths(&tx, project_id)
            .await
            .map_err(ProjectResetDbError::from)?;
        return reset_galgame_generated_data_in_tx(tx, project, candidate_media_paths).await;
    }
    if project.project_kind != PROJECT_KIND_TEXT {
        return Err(ProjectResetDbError::UnsupportedProjectKind {
            project_kind: project.project_kind,
        });
    }
    let candidate_media_paths = project_db::project_generated_media_paths(&tx, project_id)
        .await
        .map_err(ProjectResetDbError::from)?;

    let previous_status = ProjectStatus::parse_storage(&project.status).map_err(|_| {
        ProjectResetDbError::InvalidState {
            current: project.status.clone(),
            operation: "重置项目生成数据".to_string(),
        }
    })?;
    let now = Utc::now().naive_utc();

    let optimization_cleanup =
        project_db::delete_project_optimization_tree(&tx, project_id).await?;
    let preserved_blueprint_versions = blueprint_version::Entity::find()
        .filter(blueprint_version::Column::ProjectId.eq(project_id))
        .count(&tx)
        .await?;

    project_db::delete_project_chapter_adaptation_records(
        &tx,
        project_id,
        project_db::ProjectChapterAdaptationDeleteScope::CurrentProjectContent,
    )
    .await?;
    let chapter_generation_cleanup =
        delete_chapter_generation_outputs(&tx, project_id, GeneratedImageDeleteScope::Project)
            .await?;
    let generation_media_cleanup =
        project_db::delete_project_generation_media_graph(&tx, project_id).await?;
    let profile_scene_cleanup = delete_project_profile_scene_data(&tx, project_id).await?;
    crate::fact_feedback::reset_feedback_coverage(&tx, project_id).await?;

    let deleted_chapters = chapter::Entity::delete_many()
        .filter(chapter::Column::ProjectId.eq(project_id))
        .exec(&tx)
        .await?
        .rows_affected;
    let deleted_chapter_outlines = chapter_outline::Entity::delete_many()
        .filter(chapter_outline::Column::ProjectId.eq(project_id))
        .exec(&tx)
        .await?
        .rows_affected;
    let deleted_part_outlines = part_outline::Entity::delete_many()
        .filter(part_outline::Column::ProjectId.eq(project_id))
        .exec(&tx)
        .await?
        .rows_affected;

    let blueprint_cleanup = project_db::delete_project_blueprints(&tx, project_id, false).await?;

    let character_profile_reset =
        reset_character_profiles_for_project(&tx, project_id, now).await?;

    let deleted_foreshadowing = foreshadowing_index::Entity::delete_many()
        .filter(foreshadowing_index::Column::ProjectId.eq(project_id))
        .exec(&tx)
        .await?
        .rows_affected;
    let text_anchor_cleanup = project_db::delete_project_text_anchors(
        &tx,
        project_id,
        ProjectTextAnchorDeleteScope::CurrentProjectContent,
    )
    .await?;
    let deleted_import_reviews = import_review_queue::Entity::delete_many()
        .filter(import_review_queue::Column::ProjectId.eq(project_id))
        .exec(&tx)
        .await?
        .rows_affected;

    let conversation_cleanup = project_db::delete_project_conversations(&tx, project_id).await?;
    let task_cleanup = project_db::delete_project_task_queue_records(&tx, project_id).await?;

    let mut active: novel_project::ActiveModel = project.into();
    active.status = Set(ProjectStatus::Draft.as_str().to_string());
    active.current_chapter = Set(0);
    active.total_chapters = Set(None);
    active.raw_blueprint = Set(None);
    active.import_analysis_progress = Set(None);
    active.avatar_svg = Set(None);
    active.avatar_animal = Set(None);
    active.updated_at = Set(now);
    active.update(&tx).await?;
    project_db::mark_derived_projects_root_updated_in_record(&tx, project_id, now).await?;

    if previous_status != ProjectStatus::Draft {
        project_db::insert_project_status_history(
            &tx,
            ProjectStatusHistoryInput {
                project_id,
                from_status: previous_status,
                to_status: ProjectStatus::Draft,
                reason: "项目生成数据重置".to_string(),
                changed_by: "system".to_string(),
                changed_at: now,
            },
        )
        .await?;
    }

    let media_paths = crate::media::schedule_unreferenced_media_cleanup(
        &tx,
        project_id,
        &candidate_media_paths,
        format!("project-generated-data-reset:{project_id}"),
    )
    .await?;
    // gate 保持 resetting；Host 清理 RAG 成功后再完成，崩溃时由同一 gate 接续。
    tx.commit().await?;

    Ok(ProjectGeneratedDataResetOutcome {
        result: ProjectGeneratedDataResetResult {
            deleted_galgame_scripts: 0,
            deleted_galgame_scenes: 0,
            deleted_galgame_lines: 0,
            deleted_galgame_choices: 0,
            deleted_chapters,
            deleted_chapter_outlines,
            deleted_part_outlines,
            deleted_chapter_derived_rows: chapter_generation_cleanup.deleted_manga_prompts
                + chapter_generation_cleanup.deleted_video_prompts
                + chapter_generation_cleanup.deleted_chapter_scenes,
            deleted_current_blueprints: blueprint_cleanup.deleted_current_blueprints,
            deleted_blueprint_characters: blueprint_cleanup.deleted_blueprint_characters,
            deleted_blueprint_relationships: blueprint_cleanup.deleted_blueprint_relationships,
            preserved_blueprint_versions,
            reset_character_profiles: character_profile_reset.reset_character_profiles,
            deleted_character_profiles: character_profile_reset.deleted_character_profiles,
            deleted_character_snapshots: character_profile_reset.deleted_character_snapshots,
            deleted_character_timeline_rows: character_profile_reset
                .deleted_character_timeline_rows,
            deleted_character_portraits: character_profile_reset.deleted_character_portraits,
            deleted_object_profiles: profile_scene_cleanup.deleted_object_profiles,
            deleted_foreshadowing,
            deleted_scene_profiles: profile_scene_cleanup.deleted_scene_profiles,
            deleted_scene_occurrences: profile_scene_cleanup.deleted_scene_occurrences,
            deleted_scene_transitions: profile_scene_cleanup.deleted_scene_transitions,
            deleted_scene_entity_states: profile_scene_cleanup.deleted_scene_entity_states,
            deleted_text_anchors: text_anchor_cleanup.deleted_text_anchors,
            deleted_profile_evidence: text_anchor_cleanup.deleted_profile_evidence,
            deleted_import_reviews,
            deleted_conversations: conversation_cleanup.deleted_conversations,
            deleted_conversation_messages: conversation_cleanup.deleted_messages,
            deleted_optimization_sessions: optimization_cleanup.deleted_sessions,
            deleted_optimization_session_messages: optimization_cleanup.deleted_session_messages,
            deleted_optimization_turns: optimization_cleanup.deleted_turns,
            deleted_optimization_events: optimization_cleanup.deleted_events,
            deleted_optimization_patch_sets: optimization_cleanup.deleted_patch_sets,
            deleted_optimization_patches: optimization_cleanup.deleted_patches,
            deleted_generated_images: chapter_generation_cleanup.deleted_generated_images,
            deleted_generated_videos: chapter_generation_cleanup.deleted_generated_videos,
            deleted_generation_specs: generation_media_cleanup.deleted_generation_specs,
            deleted_prompt_artifacts: generation_media_cleanup.deleted_prompt_artifacts,
            deleted_media_assets: generation_media_cleanup.deleted_media_assets,
            deleted_media_provenance: generation_media_cleanup.deleted_media_provenance,
            deleted_media_bindings: generation_media_cleanup.deleted_media_bindings,
            deleted_tasks: task_cleanup.deleted_tasks,
            deleted_task_dependencies: task_cleanup.deleted_task_dependencies,
        },
        previous_status,
        media_paths,
    })
}
