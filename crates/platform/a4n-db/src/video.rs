pub mod prompts;
pub use prompts::source_sync::with_video_plan_source_sync;
use sea_orm::{ConnectionTrait, DbErr};

pub use prompts::records::{
    ChapterVideoPromptCandidateRecord, ChapterVideoPromptTimelineRecord,
    chapter_video_boundary_frame_prompt_candidates,
    chapter_video_first_frame_chain_prompt_candidates, chapter_video_first_frame_prompt_candidates,
    chapter_video_prompt_by_id, chapter_video_prompt_ids_for_types,
    chapter_video_prompt_records_for_types, chapter_video_prompt_scope_by_id,
    chapter_video_prompt_segment_plan_records, chapter_video_prompt_timeline_records_for_chapter,
    chapter_video_prompt_video_binding_records_for_chapter, chapter_video_prompt_workflow_records,
    chapter_video_prompt_workflow_shot_records, chapter_video_prompts_for_batch_generation,
    chapter_video_prompts_for_chapter, chapter_video_storyboard_prompt_candidates,
    completed_shared_segment_plan_id_for_source_hash, update_chapter_video_prompt_record,
};
pub(crate) use prompts::records::{
    PromptDependencyRecord, chapter_video_prompt_dependency_records,
    chapter_video_prompt_previous_frame_candidates, chapter_video_prompt_record,
    chapter_video_prompt_sibling_dependency_records, chapter_video_prompt_sibling_records,
    chapter_video_prompt_sync_candidates, delete_chapter_video_prompt_records,
    ensure_chapter_video_prompt_not_running, frame_chapter_video_prompt_record,
    replace_chapter_video_prompt_record, update_chapter_video_prompt_metadata_record,
};

mod generated_images;
mod media_mapping;
pub(crate) use media_mapping::generated_image_record_from_model;
pub mod read_models;
pub(crate) use generated_images::validated_video_reference_image_records_batch;
pub(crate) use generated_images::video_project_record;
pub use generated_images::{
    ImagePublicationRequest, VideoProfileMaterialReadModel, VideoProfilePrimaryMaterialRow,
    VideoProfileRelevanceRow, VideoProjectContextReadModel, VideoProjectIdentity,
    VideoPromptBlueprintCharacterRecord, VideoPromptProtagonistProfileRecord,
    create_generated_image_record, create_image_publication_intent,
    delete_generated_image_record_with_cleanup, delete_manga_page_image_records,
    ensure_image_chapter_belongs_to_project_record, ensure_video_chapter_belongs_to_project_record,
    ensure_video_reference_images_available_record, generated_image_by_id,
    generated_image_ids_by_ids, generated_images_by_ids, generated_images_for_chapter,
    generated_images_for_project, generated_images_for_projects,
    generated_images_for_projects_chapter_by_id_desc, image_publication_by_id,
    image_publication_by_image_id, image_publications_for_recovery,
    image_publications_for_recovery_for_model, mark_image_publication_reclaimed,
    mark_image_publication_reclaiming, publish_image_publication,
    publish_image_publication_with_generation_run_checkpoint, update_generated_image_record,
    update_image_publication_metadata, validate_image_publication_request,
    validated_video_reference_image_records, video_asset_project_ids_record, video_chapter_record,
    video_library_chapter_image_records, video_library_explicit_image_records,
    video_profile_material_read_model, video_project_context_read_model,
    video_reference_candidate_image_records, video_tail_frame_images,
};
pub(crate) use generated_images::{
    clear_generated_image_references_record, generated_images_for_chapter_by_id_desc,
    reclaim_image_publications_for_deleted_images,
};

mod generated_videos;
pub(crate) use generated_videos::clear_generated_video_references_by_ids;
pub use generated_videos::{
    create_generated_video_record, create_generated_video_record_with_generation_run,
    create_prompt_generated_video_record_with_generation_run,
    delete_generated_video_record_with_cleanup, delete_missing_generated_video_references_record,
    generated_video_by_id, generated_videos_by_ids, generated_videos_by_provider_job_id,
    generated_videos_for_chapter, mark_generated_video_completed_and_link_prompt_record,
    mark_generated_video_error_status_and_prompt_record,
    mark_generated_video_job_and_prompt_record,
    mark_generated_videos_error_status_and_prompt_records, merge_video_provider_response_metadata,
    missing_video_reference_project_ids, recover_stale_generated_videos_without_provider_job,
    update_generated_video_record, update_generated_video_waiting_status_and_prompt_record,
    validate_video_metadata_update_preserves_system_fields,
};

mod segment_plan_draft;
pub use segment_plan_draft::{
    VideoSegmentPlanDraftCheckpoint, video_segment_plan_draft_checkpoints,
};

mod timeline;
pub use timeline::{
    CommitInsertedTimelineItemInput, TIMELINE_ACTIVE_STATUS, TIMELINE_DELETED_STATUS,
    TIMELINE_INSERTED_KIND, TIMELINE_NATIVE_KIND, TIMELINE_NEEDS_REANCHOR_STATUS,
    VideoTimelineSnapshot, active_timeline_order, chapter_video_timeline_by_scope,
    chapter_video_timeline_snapshot, commit_inserted_timeline_item, delete_inserted_timeline_item,
    ensure_chapter_video_timeline, inserted_timeline_item_by_workflow_run,
    move_inserted_timeline_item_persisted, reconcile_native_timeline,
};

#[derive(Debug, thiserror::Error)]
pub enum VideoDbError {
    #[error("{0}")]
    InvalidArgument(String),
    #[error("当前状态 {current} 不允许执行 {operation}")]
    InvalidState { current: String, operation: String },
    #[error("{0}")]
    NotFound(String),
    #[error("{0}")]
    Forbidden(String),
    #[error(transparent)]
    Database(#[from] DbErr),
}

impl From<crate::canvas::CanvasDbError> for VideoDbError {
    fn from(err: crate::canvas::CanvasDbError) -> Self {
        match err {
            crate::canvas::CanvasDbError::InvalidArgument(message) => {
                VideoDbError::InvalidArgument(message)
            }
            crate::canvas::CanvasDbError::NotFound(message) => VideoDbError::NotFound(message),
            crate::canvas::CanvasDbError::Forbidden(message) => VideoDbError::Forbidden(message),
            crate::canvas::CanvasDbError::VersionConflict {
                graph_id,
                expected_version,
                current_version,
            } => VideoDbError::InvalidState {
                current: format!(
                    "canvas_graph_version_conflict:graph_id={graph_id},expected={expected_version},current={current_version}"
                ),
                operation: "保存画布".to_string(),
            },
            crate::canvas::CanvasDbError::TextAssetVersionConflict {
                canvas_text_asset_id,
                expected_version,
                current_version,
            } => VideoDbError::InvalidState {
                current: format!(
                    "canvas_text_asset_version_conflict:asset_id={canvas_text_asset_id},expected={expected_version},current={current_version}"
                ),
                operation: "保存文本素材".to_string(),
            },
            crate::canvas::CanvasDbError::Database(err) => VideoDbError::Database(err),
        }
    }
}

impl From<crate::generation::GenerationDbError> for VideoDbError {
    fn from(err: crate::generation::GenerationDbError) -> Self {
        match err {
            crate::generation::GenerationDbError::InvalidArgument(message) => {
                VideoDbError::InvalidArgument(message)
            }
            crate::generation::GenerationDbError::NotFound(message) => {
                VideoDbError::NotFound(message)
            }
            crate::generation::GenerationDbError::Database(err) => VideoDbError::Database(err),
        }
    }
}

async fn lock_generated_media_project<C>(db: &C, project_id: i64) -> Result<(), VideoDbError>
where
    C: ConnectionTrait,
{
    let project_id = validate_positive_video_i64(project_id, "project_id")?;
    let project = crate::project::project_write_identity_for_update(db, project_id)
        .await?
        .ok_or_else(|| VideoDbError::NotFound(format!("项目不存在: {project_id}")))?;
    if project.lifecycle_gate != crate::generation::PROJECT_GATE_ACTIVE {
        return Err(VideoDbError::InvalidState {
            current: project.lifecycle_gate,
            operation: "写入生成媒体".to_string(),
        });
    }
    Ok(())
}

async fn lock_generated_media_project_for_update<C>(
    db: &C,
    project_id: i64,
) -> Result<(), VideoDbError>
where
    C: ConnectionTrait,
{
    let project_id = validate_positive_video_i64(project_id, "project_id")?;
    crate::project::project_write_identity_for_update(db, project_id)
        .await
        .map_err(VideoDbError::Database)?
        .ok_or_else(|| VideoDbError::NotFound(format!("项目不存在: {project_id}")))?;
    Ok(())
}

/// Reclaiming is the finalization path of a durable publication audit record.
/// Project deletion deliberately removes the project row only after leaving
/// that record behind, so a missing project must not strand the audit state.
async fn lock_image_publication_reclaim_project_for_update<C>(
    db: &C,
    project_id: i64,
) -> Result<(), VideoDbError>
where
    C: ConnectionTrait,
{
    let project_id = validate_positive_video_i64(project_id, "project_id")?;
    crate::project::project_write_identity_for_update(db, project_id)
        .await
        .map_err(VideoDbError::Database)?;
    Ok(())
}

pub(super) fn required_video_record_text(value: &str, field: &str) -> Result<String, VideoDbError> {
    let value = value.trim();
    if value.is_empty() {
        return Err(VideoDbError::InvalidArgument(format!("{field} 不能为空")));
    }
    Ok(value.to_string())
}

pub(super) fn normalize_optional_video_record_text(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToString::to_string)
}

pub(super) fn validate_positive_video_i64(value: i64, field: &str) -> Result<i64, VideoDbError> {
    if value <= 0 {
        return Err(VideoDbError::InvalidArgument(format!("{field} 必须大于 0")));
    }
    Ok(value)
}

pub(super) fn validate_positive_video_i32(value: i32, field: &str) -> Result<i32, VideoDbError> {
    if value <= 0 {
        return Err(VideoDbError::InvalidArgument(format!("{field} 必须大于 0")));
    }
    Ok(value)
}

pub(super) fn validate_positive_video_ids(values: &[i64], field: &str) -> Result<(), VideoDbError> {
    if let Some(value) = values.iter().find(|value| **value <= 0) {
        return Err(VideoDbError::InvalidArgument(format!(
            "{field} 必须全部大于 0: {value}"
        )));
    }
    Ok(())
}

mod source_context;
pub use source_context::{
    project_and_chapter_for_video_source, source_snapshot_chapter_for_video_source,
};

#[cfg(test)]
mod tests;
