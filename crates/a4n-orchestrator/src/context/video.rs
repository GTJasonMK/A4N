//! Video 领域私有 owner。
//!
//! 持有 `DbHandle`，封装视频提示词、生成图/视频记录与素材库相关数据库行为。
//! LLM/预览/流水线编排仍在 `video/` 模块，但 DB 只经本 owner。

use a4n_core::error::AppError;
use a4n_core::models::{
    ai::{generated_image, generated_video},
    chapter::{chapter_video_prompt, chapter_video_timeline},
};
use a4n_core::source_spans::source_text_hash as source_content_sha256;
use a4n_core::video_execution::VideoSegmentExecutionRecipeV1;
use a4n_core::video_execution::VideoTimelineGap;
use a4n_db::DbHandle;
use a4n_db::generation::{
    RecordCompletedGenerationRunWithCheckpointInput, RecordStartedGenerationRunInput,
};
use a4n_db::video::{
    CreateGeneratedImageRecordDraftInput, CreateGeneratedVideoRecordDraftInput,
    UpdateGeneratedImageRecordInput, UpdateGeneratedVideoRecordInput,
};
use a4n_db::video_prompt::{ChapterVideoPromptPatch, VideoPromptDownstreamDependencies};
use sea_orm::DatabaseConnection;
use serde_json::{Value, json};
use std::collections::BTreeSet;

use super::OrchestratorContext;
use crate::video::video_db_error;
use crate::workflow::{
    UpsertWorkflowCheckpointInput, WORKFLOW_STATUS_CANCELLED, WORKFLOW_STATUS_FAILED,
    WORKFLOW_STATUS_INTERRUPTED, WORKFLOW_STATUS_RUNNING, WORKFLOW_STATUS_SUPERSEDED,
    WorkflowProgressEventInput,
};

fn terminal_workflow_status_label(status: &str) -> &'static str {
    match status {
        WORKFLOW_STATUS_CANCELLED => "取消",
        WORKFLOW_STATUS_FAILED => "失败",
        WORKFLOW_STATUS_SUPERSEDED => "被取代",
        _ => "结束",
    }
}

/// Video 领域 application owner。
#[derive(Clone)]
pub(super) struct VideoApplication {
    database: DbHandle,
}

impl VideoApplication {
    pub(super) fn new(database: DbHandle) -> Self {
        Self { database }
    }

    fn connection(&self) -> &DatabaseConnection {
        self.database.connection()
    }

    pub(super) async fn video_asset_project_ids_record(
        &self,
        project_id: i64,
    ) -> Result<Vec<i64>, AppError> {
        a4n_db::video::video_asset_project_ids_record(self.connection(), project_id)
            .await
            .map_err(video_db_error)
    }

    pub(super) async fn validated_video_reference_image_records(
        &self,
        project_id: i64,
        chapter_id: i64,
        reference_image_ids_json: Option<&Value>,
    ) -> Result<Vec<generated_image::Model>, AppError> {
        a4n_db::video::validated_video_reference_image_records(
            self.database.connection(),
            project_id,
            chapter_id,
            reference_image_ids_json,
        )
        .await
        .map_err(video_db_error)
    }

    pub(super) async fn ensure_video_reference_images_available_record(
        &self,
        project_id: i64,
        chapter_id: i64,
        reference_image_ids_json: Option<&Value>,
    ) -> Result<(), AppError> {
        a4n_db::video::ensure_video_reference_images_available_record(
            self.connection(),
            project_id,
            chapter_id,
            reference_image_ids_json,
        )
        .await
        .map_err(video_db_error)
    }

    pub(super) async fn generated_images_for_video_prompt_references(
        &self,
        project_id: i64,
        chapter_id: i64,
    ) -> Result<Vec<generated_image::Model>, AppError> {
        a4n_db::video::generated_images_for_video_prompt_references(
            self.connection(),
            project_id,
            chapter_id,
        )
        .await
        .map_err(video_db_error)
    }

    pub(super) async fn generated_images_by_ids(
        &self,
        image_ids: &[i64],
    ) -> Result<Vec<generated_image::Model>, AppError> {
        a4n_db::video::generated_images_by_ids(self.connection(), image_ids)
            .await
            .map_err(video_db_error)
    }

    pub(super) async fn generated_images_for_project(
        &self,
        project_id: i64,
    ) -> Result<Vec<generated_image::Model>, AppError> {
        a4n_db::video::generated_images_for_project(self.connection(), project_id)
            .await
            .map_err(AppError::Database)
    }

    pub(super) async fn generated_images_for_chapter(
        &self,
        project_id: i64,
        chapter_id: i64,
    ) -> Result<Vec<generated_image::Model>, AppError> {
        a4n_db::video::generated_images_for_chapter(self.connection(), project_id, chapter_id)
            .await
            .map_err(AppError::Database)
    }

    pub(super) async fn ensure_image_chapter_belongs_to_project(
        &self,
        project_id: i64,
        chapter_id: i64,
    ) -> Result<(), AppError> {
        a4n_db::video::ensure_image_chapter_belongs_to_project_record(
            self.connection(),
            project_id,
            chapter_id,
        )
        .await
        .map_err(video_db_error)
    }

    pub(super) async fn video_reference_candidate_image_records(
        &self,
        project_ids: &[i64],
        chapter_id: i64,
    ) -> Result<Vec<generated_image::Model>, AppError> {
        a4n_db::video::video_reference_candidate_image_records(
            self.connection(),
            project_ids,
            chapter_id,
        )
        .await
        .map_err(video_db_error)
    }

    pub(super) async fn generated_image_by_id(
        &self,
        image_id: i64,
    ) -> Result<Option<generated_image::Model>, AppError> {
        a4n_db::video::generated_image_by_id(self.connection(), image_id)
            .await
            .map_err(AppError::Database)
    }

    pub(super) async fn toggle_generated_image_favorite_record(
        &self,
        image_id: i64,
    ) -> Result<generated_image::Model, AppError> {
        a4n_db::video::toggle_generated_image_favorite_record(self.connection(), image_id)
            .await
            .map_err(video_db_error)
    }

    pub(super) async fn update_generated_image_record(
        &self,
        image_id: i64,
        input: UpdateGeneratedImageRecordInput,
    ) -> Result<generated_image::Model, AppError> {
        a4n_db::video::update_generated_image_record(self.connection(), image_id, input)
            .await
            .map_err(video_db_error)
    }

    pub(super) async fn delete_generated_image_record_with_cleanup(
        &self,
        image_id: i64,
    ) -> Result<(), AppError> {
        a4n_db::video::delete_generated_image_record_with_cleanup(self.connection(), image_id)
            .await
            .map_err(video_db_error)
    }

    pub(super) async fn delete_manga_page_image_records(
        &self,
        project_id: i64,
        chapter_id: i64,
        page_number: i32,
        preserve_image_id: Option<i64>,
    ) -> Result<Vec<String>, AppError> {
        a4n_db::video::delete_manga_page_image_records(
            self.connection(),
            project_id,
            chapter_id,
            page_number,
            preserve_image_id,
        )
        .await
        .map_err(video_db_error)
    }

    pub(super) async fn video_tail_frame_images(
        &self,
    ) -> Result<Vec<generated_image::Model>, AppError> {
        a4n_db::video::video_tail_frame_images(self.connection())
            .await
            .map_err(video_db_error)
    }

    pub(super) async fn create_generated_image_record(
        &self,
        input: CreateGeneratedImageRecordDraftInput,
    ) -> Result<generated_image::Model, AppError> {
        a4n_db::video::create_generated_image_record(self.connection(), input)
            .await
            .map_err(video_db_error)
    }

    pub(super) async fn create_generated_image_record_with_generation_run_checkpoint(
        &self,
        input: CreateGeneratedImageRecordDraftInput,
        generation_run: RecordCompletedGenerationRunWithCheckpointInput,
    ) -> Result<generated_image::Model, AppError> {
        a4n_db::video::create_generated_image_record_with_generation_run_checkpoint(
            self.connection(),
            input,
            generation_run,
        )
        .await
        .map_err(video_db_error)
    }

    pub(super) async fn create_generated_video_record(
        &self,
        input: CreateGeneratedVideoRecordDraftInput,
    ) -> Result<generated_video::Model, AppError> {
        a4n_db::video::create_generated_video_record(self.connection(), input)
            .await
            .map_err(video_db_error)
    }

    pub(super) async fn create_generated_video_record_with_generation_run(
        &self,
        input: CreateGeneratedVideoRecordDraftInput,
        generation_run: RecordStartedGenerationRunInput,
    ) -> Result<generated_video::Model, AppError> {
        a4n_db::video::create_generated_video_record_with_generation_run(
            self.connection(),
            input,
            generation_run,
        )
        .await
        .map_err(video_db_error)
    }

    pub(super) async fn create_prompt_generated_video_record_with_generation_run(
        &self,
        input: CreateGeneratedVideoRecordDraftInput,
        generation_run: RecordStartedGenerationRunInput,
        prompt_id: i64,
    ) -> Result<generated_video::Model, AppError> {
        a4n_db::video::create_prompt_generated_video_record_with_generation_run(
            self.connection(),
            input,
            generation_run,
            prompt_id,
        )
        .await
        .map_err(video_db_error)
    }

    pub(super) async fn generated_videos_for_chapter(
        &self,
        project_id: i64,
        chapter_id: i64,
    ) -> Result<Vec<generated_video::Model>, AppError> {
        a4n_db::video::generated_videos_for_chapter(self.connection(), project_id, chapter_id)
            .await
            .map_err(AppError::Database)
    }

    pub(super) async fn generated_video_by_id(
        &self,
        video_id: i64,
    ) -> Result<Option<generated_video::Model>, AppError> {
        a4n_db::video::generated_video_by_id(self.connection(), video_id)
            .await
            .map_err(AppError::Database)
    }

    pub(super) async fn generated_videos_by_provider_job_id(
        &self,
        project_id: Option<i64>,
        provider_job_id: &str,
    ) -> Result<Vec<generated_video::Model>, AppError> {
        a4n_db::video::generated_videos_by_provider_job_id(
            self.connection(),
            project_id,
            provider_job_id,
        )
        .await
        .map_err(AppError::Database)
    }

    pub(super) async fn toggle_generated_video_favorite_record(
        &self,
        video_id: i64,
    ) -> Result<generated_video::Model, AppError> {
        a4n_db::video::toggle_generated_video_favorite_record(self.connection(), video_id)
            .await
            .map_err(video_db_error)
    }

    pub(super) async fn update_generated_video_record(
        &self,
        video_id: i64,
        input: UpdateGeneratedVideoRecordInput,
    ) -> Result<generated_video::Model, AppError> {
        a4n_db::video::update_generated_video_record(self.connection(), video_id, input)
            .await
            .map_err(video_db_error)
    }

    pub(super) async fn update_generated_video_duration_record(
        &self,
        video_id: i64,
        duration_secs: f32,
    ) -> Result<generated_video::Model, AppError> {
        a4n_db::video::update_generated_video_duration_record(
            self.connection(),
            video_id,
            duration_secs,
        )
        .await
        .map_err(video_db_error)
    }

    pub(super) async fn delete_generated_video_record_with_cleanup(
        &self,
        video_id: i64,
    ) -> Result<(), AppError> {
        a4n_db::video::delete_generated_video_record_with_cleanup(self.connection(), video_id)
            .await
            .map_err(video_db_error)
    }

    pub(super) async fn delete_missing_generated_video_references_record(
        &self,
        video_id: i64,
    ) -> Result<u64, AppError> {
        a4n_db::video::delete_missing_generated_video_references_record(self.connection(), video_id)
            .await
            .map_err(video_db_error)
    }

    pub(super) async fn missing_video_reference_project_ids(
        &self,
        video_id: i64,
    ) -> Result<Vec<i64>, AppError> {
        a4n_db::video::missing_video_reference_project_ids(self.connection(), video_id)
            .await
            .map_err(video_db_error)
    }

    pub(super) async fn update_generated_video_waiting_status_and_prompt_record(
        &self,
        video_id: i64,
        prompt_id: Option<i64>,
        status: &str,
    ) -> Result<generated_video::Model, AppError> {
        a4n_db::video::update_generated_video_waiting_status_and_prompt_record(
            self.connection(),
            video_id,
            prompt_id,
            status,
        )
        .await
        .map_err(video_db_error)
    }

    pub(super) async fn mark_generated_video_error_status_and_prompt_record(
        &self,
        video_id: i64,
        prompt_id: Option<i64>,
        status: &str,
        message: &str,
    ) -> Result<generated_video::Model, AppError> {
        a4n_db::video::mark_generated_video_error_status_and_prompt_record(
            self.connection(),
            video_id,
            prompt_id,
            status,
            message,
        )
        .await
        .map_err(video_db_error)
    }

    pub(super) async fn mark_generated_video_job_and_prompt_record(
        &self,
        video_id: i64,
        provider_job_id: &str,
        metadata_json: Option<Value>,
        prompt_id: Option<i64>,
    ) -> Result<generated_video::Model, AppError> {
        a4n_db::video::mark_generated_video_job_and_prompt_record(
            self.connection(),
            video_id,
            provider_job_id,
            metadata_json,
            prompt_id,
        )
        .await
        .map_err(video_db_error)
    }

    pub(super) async fn mark_generated_video_completed_and_link_prompt_record(
        &self,
        video_id: i64,
        file_path: &str,
        thumbnail_path: Option<&str>,
        file_size: i64,
        metadata_json: Option<Value>,
        prompt_id: Option<i64>,
    ) -> Result<generated_video::Model, AppError> {
        a4n_db::video::mark_generated_video_completed_and_link_prompt_record(
            self.connection(),
            video_id,
            file_path,
            thumbnail_path,
            file_size,
            metadata_json,
            prompt_id,
        )
        .await
        .map_err(video_db_error)
    }

    pub(super) async fn chapter_video_prompt_by_id(
        &self,
        prompt_id: i64,
    ) -> Result<Option<chapter_video_prompt::Model>, AppError> {
        a4n_db::video::chapter_video_prompt_by_id(self.connection(), prompt_id)
            .await
            .map_err(video_db_error)
    }

    pub(super) async fn chapter_video_prompts_for_chapter(
        &self,
        project_id: i64,
        chapter_id: i64,
    ) -> Result<Vec<chapter_video_prompt::Model>, AppError> {
        a4n_db::video::chapter_video_prompts_for_chapter(self.connection(), project_id, chapter_id)
            .await
            .map_err(AppError::Database)
    }

    pub(super) async fn chapter_video_prompts_for_batch_generation(
        &self,
        prompt_ids: &[i64],
    ) -> Result<Vec<chapter_video_prompt::Model>, AppError> {
        a4n_db::video::chapter_video_prompts_for_batch_generation(self.connection(), prompt_ids)
            .await
            .map_err(video_db_error)
    }

    pub(super) async fn update_chapter_video_prompt_with_policy(
        &self,
        prompt_id: i64,
        input: ChapterVideoPromptPatch,
        allow_downstream: bool,
    ) -> Result<chapter_video_prompt::Model, AppError> {
        a4n_db::video_prompt::update_chapter_video_prompt_with_policy(
            self.connection(),
            prompt_id,
            input,
            allow_downstream,
        )
        .await
        .map_err(video_db_error)
    }

    pub(super) async fn update_chapter_video_segment_recipe(
        &self,
        prompt_id: i64,
        recipe: VideoSegmentExecutionRecipeV1,
    ) -> Result<Vec<VideoSegmentExecutionRecipeV1>, AppError> {
        a4n_db::video_prompt::update_chapter_video_segment_recipe(
            self.connection(),
            prompt_id,
            recipe,
        )
        .await
        .map_err(video_db_error)
    }

    pub(super) async fn update_chapter_video_prompt_with_generation_run_checkpoint(
        &self,
        prompt_id: i64,
        input: ChapterVideoPromptPatch,
        metadata_json: Option<Value>,
        allow_downstream: bool,
        generation_run: RecordCompletedGenerationRunWithCheckpointInput,
    ) -> Result<chapter_video_prompt::Model, AppError> {
        a4n_db::video_prompt::update_chapter_video_prompt_with_generation_run_checkpoint(
            self.connection(),
            prompt_id,
            input,
            metadata_json,
            allow_downstream,
            generation_run,
        )
        .await
        .map_err(video_db_error)
    }

    pub(super) async fn delete_chapter_video_prompt_with_policy(
        &self,
        prompt_id: i64,
        allow_downstream: bool,
    ) -> Result<(), AppError> {
        a4n_db::video_prompt::delete_chapter_video_prompt_with_policy(
            self.connection(),
            prompt_id,
            allow_downstream,
        )
        .await
        .map_err(video_db_error)
    }

    pub(super) async fn delete_chapter_video_prompts_with_policy(
        &self,
        prompt_ids: &[i64],
        allow_downstream: bool,
    ) -> Result<u64, AppError> {
        a4n_db::video_prompt::delete_chapter_video_prompts_with_policy(
            self.connection(),
            prompt_ids,
            allow_downstream,
        )
        .await
        .map_err(video_db_error)
    }

    pub(super) async fn prompt_downstream_dependencies(
        &self,
        prompt_id: i64,
    ) -> Result<VideoPromptDownstreamDependencies, AppError> {
        a4n_db::video_prompt::prompt_downstream_dependencies(self.connection(), prompt_id)
            .await
            .map_err(video_db_error)
    }

    pub(super) async fn ensure_frame_prompt_image_generation_allowed(
        &self,
        prompt_id: i64,
        generation_source: &str,
    ) -> Result<(), AppError> {
        a4n_db::video_prompt::ensure_frame_prompt_image_generation_allowed(
            self.connection(),
            prompt_id,
            generation_source,
        )
        .await
        .map_err(video_db_error)
    }

    pub(super) async fn effective_frame_prompt_image_reference_ids(
        &self,
        prompt_id: i64,
        generation_source: &str,
        requested_reference_image_ids: &[i64],
    ) -> Result<Vec<i64>, AppError> {
        a4n_db::video_prompt::effective_frame_prompt_image_reference_ids(
            self.connection(),
            prompt_id,
            generation_source,
            requested_reference_image_ids,
        )
        .await
        .map_err(video_db_error)
    }

    pub(super) async fn ensure_chapter_video_prompt_stage_mutable(
        &self,
        prompt: &chapter_video_prompt::Model,
        operation: &str,
    ) -> Result<(), AppError> {
        a4n_db::video_prompt::ensure_chapter_video_prompt_stage_mutable(
            self.connection(),
            prompt,
            operation,
        )
        .await
        .map_err(video_db_error)
    }

    pub(super) async fn ensure_chapter_video_prompt_stage_mutable_ignoring(
        &self,
        prompt: &chapter_video_prompt::Model,
        operation: &str,
        ignored_prompt_ids: &BTreeSet<i64>,
    ) -> Result<(), AppError> {
        a4n_db::video_prompt::ensure_chapter_video_prompt_stage_mutable_ignoring(
            self.connection(),
            prompt,
            operation,
            ignored_prompt_ids,
        )
        .await
        .map_err(video_db_error)
    }

    pub(super) async fn chapter_video_prompt_has_generated_frame_image(
        &self,
        prompt: &chapter_video_prompt::Model,
    ) -> Result<bool, AppError> {
        a4n_db::video_prompt::chapter_video_prompt_has_generated_frame_image(
            self.connection(),
            prompt,
        )
        .await
        .map_err(video_db_error)
    }

    pub(super) async fn chapter_video_timeline_by_scope(
        &self,
        project_id: i64,
        chapter_id: i64,
        workflow_mode: &str,
    ) -> Result<Option<chapter_video_timeline::Model>, AppError> {
        a4n_db::video::chapter_video_timeline_by_scope(
            self.connection(),
            project_id,
            chapter_id,
            workflow_mode,
        )
        .await
        .map_err(video_db_error)
    }

    pub(super) async fn chapter_video_timeline_snapshot(
        &self,
        timeline_id: i64,
    ) -> Result<a4n_db::video::VideoTimelineSnapshot, AppError> {
        a4n_db::video::chapter_video_timeline_snapshot(self.connection(), timeline_id)
            .await
            .map_err(video_db_error)
    }

    pub(super) async fn ensure_chapter_video_timeline(
        &self,
        project_id: i64,
        chapter_id: i64,
        workflow_mode: &str,
    ) -> Result<chapter_video_timeline::Model, AppError> {
        a4n_db::video::ensure_chapter_video_timeline(
            self.connection(),
            project_id,
            chapter_id,
            workflow_mode,
        )
        .await
        .map_err(video_db_error)
    }

    pub(super) async fn reconcile_native_video_timeline(
        &self,
        timeline_id: i64,
        keys: &[a4n_core::video_execution::NativeVideoTimelineKey],
    ) -> Result<a4n_db::video::VideoTimelineSnapshot, AppError> {
        a4n_db::video::reconcile_native_timeline(self.connection(), timeline_id, keys)
            .await
            .map_err(video_db_error)
    }

    pub(super) async fn commit_inserted_video_timeline_item(
        &self,
        input: a4n_db::video::CommitInsertedTimelineItemInput,
    ) -> Result<(), AppError> {
        a4n_db::video::commit_inserted_timeline_item(self.connection(), input)
            .await
            .map_err(video_db_error)
    }

    pub(super) async fn move_inserted_video_timeline_item(
        &self,
        timeline_id: i64,
        expected_revision: i64,
        item_id: i64,
        gap: VideoTimelineGap,
    ) -> Result<chapter_video_timeline::Model, AppError> {
        a4n_db::video::move_inserted_timeline_item_persisted(
            self.connection(),
            timeline_id,
            expected_revision,
            item_id,
            gap,
        )
        .await
        .map_err(video_db_error)
    }

    pub(super) async fn delete_inserted_video_timeline_item(
        &self,
        timeline_id: i64,
        expected_revision: i64,
        item_id: i64,
    ) -> Result<chapter_video_timeline::Model, AppError> {
        a4n_db::video::delete_inserted_timeline_item(
            self.connection(),
            timeline_id,
            expected_revision,
            item_id,
        )
        .await
        .map_err(video_db_error)
    }
}

// OrchestratorContext 委托：供 video/ 编排与 runtime facade 使用。
impl OrchestratorContext {
    pub(crate) async fn chapter_video_timeline_by_scope(
        &self,
        project_id: i64,
        chapter_id: i64,
        workflow_mode: &str,
    ) -> Result<Option<chapter_video_timeline::Model>, AppError> {
        self.video
            .chapter_video_timeline_by_scope(project_id, chapter_id, workflow_mode)
            .await
    }

    pub(crate) async fn chapter_video_timeline_snapshot(
        &self,
        timeline_id: i64,
    ) -> Result<a4n_db::video::VideoTimelineSnapshot, AppError> {
        self.video
            .chapter_video_timeline_snapshot(timeline_id)
            .await
    }

    pub(crate) async fn ensure_chapter_video_timeline(
        &self,
        project_id: i64,
        chapter_id: i64,
        workflow_mode: &str,
    ) -> Result<chapter_video_timeline::Model, AppError> {
        self.video
            .ensure_chapter_video_timeline(project_id, chapter_id, workflow_mode)
            .await
    }

    pub(crate) async fn reconcile_native_video_timeline(
        &self,
        timeline_id: i64,
        keys: &[a4n_core::video_execution::NativeVideoTimelineKey],
    ) -> Result<a4n_db::video::VideoTimelineSnapshot, AppError> {
        self.video
            .reconcile_native_video_timeline(timeline_id, keys)
            .await
    }

    pub(crate) async fn commit_inserted_video_timeline_item(
        &self,
        input: a4n_db::video::CommitInsertedTimelineItemInput,
    ) -> Result<(), AppError> {
        self.video.commit_inserted_video_timeline_item(input).await
    }

    pub(crate) async fn move_inserted_video_timeline_item(
        &self,
        timeline_id: i64,
        expected_revision: i64,
        item_id: i64,
        gap: VideoTimelineGap,
    ) -> Result<chapter_video_timeline::Model, AppError> {
        self.video
            .move_inserted_video_timeline_item(timeline_id, expected_revision, item_id, gap)
            .await
    }

    pub(crate) async fn delete_inserted_video_timeline_item(
        &self,
        timeline_id: i64,
        expected_revision: i64,
        item_id: i64,
    ) -> Result<chapter_video_timeline::Model, AppError> {
        self.video
            .delete_inserted_video_timeline_item(timeline_id, expected_revision, item_id)
            .await
    }

    pub async fn ensure_video_capacity(
        &self,
        config_id: i64,
        max_concurrent: i32,
    ) -> Result<(), AppError> {
        crate::video::validate_video_max_concurrent(max_concurrent)
            .map_err(AppError::InvalidArgument)?;
        let limit = max_concurrent as usize;
        let used = self
            .active_video_ids_for_config_reconciled(config_id)
            .await?
            .len();
        if used >= limit {
            return Err(AppError::InvalidArgument(format!(
                "当前视频配置已有 {used} 个进行中的任务，已达到并发上限 {limit}"
            )));
        }
        Ok(())
    }

    pub async fn ensure_created_video_capacity(
        &self,
        config_id: i64,
        max_concurrent: i32,
        video_id: i64,
    ) -> Result<(), AppError> {
        crate::video::validate_video_max_concurrent(max_concurrent)
            .map_err(AppError::InvalidArgument)?;
        let limit = max_concurrent as usize;
        let active = self
            .active_video_ids_for_config_reconciled(config_id)
            .await?;
        if active.len() <= limit || active.iter().take(limit).any(|id| *id == video_id) {
            return Ok(());
        }
        let existing_active_count = active.iter().filter(|id| **id != video_id).count();
        Err(AppError::InvalidArgument(format!(
            "当前视频配置已有 {existing_active_count} 个进行中的任务，已达到并发上限 {limit}"
        )))
    }

    async fn active_video_ids_for_config_reconciled(
        &self,
        config_id: i64,
    ) -> Result<Vec<i64>, AppError> {
        let active_ids = self
            .active_generated_video_ids_for_config(config_id)
            .await?;
        let mut retained = Vec::with_capacity(active_ids.len());
        for video_id in active_ids {
            if self.reconcile_interrupted_provider_video(video_id).await? {
                continue;
            }
            retained.push(video_id);
        }
        Ok(retained)
    }

    async fn reconcile_interrupted_provider_video(&self, video_id: i64) -> Result<bool, AppError> {
        let Some(video) = self.video.generated_video_by_id(video_id).await? else {
            return Ok(true);
        };
        if video
            .provider_job_id
            .as_deref()
            .map(str::trim)
            .unwrap_or("")
            .is_empty()
        {
            return Ok(false);
        }
        if self
            .reconcile_terminal_canvas_workflow_video(&video)
            .await?
        {
            return Ok(true);
        }
        let Some(run) = self
            .latest_workflow_run_by_business_key(&format!("video_provider_job:{video_id}"))
            .await?
        else {
            return Ok(false);
        };
        if run.status != WORKFLOW_STATUS_INTERRUPTED {
            return Ok(false);
        }
        let message = run
            .error_message
            .as_deref()
            .unwrap_or("视频 provider 轮询已中断，可稍后恢复轮询");
        self.mark_generated_video_interrupted_validated(
            video.id,
            crate::video::prompt_id_from_generated_video(&video),
            message,
        )
        .await?;
        Ok(true)
    }

    async fn reconcile_terminal_canvas_workflow_video(
        &self,
        video: &generated_video::Model,
    ) -> Result<bool, AppError> {
        let Some(workflow_run_id) = video
            .metadata_json
            .as_ref()
            .and_then(|metadata| metadata.get("canvas"))
            .and_then(|canvas| canvas.get("workflow_run_id"))
            .and_then(serde_json::Value::as_i64)
        else {
            return Ok(false);
        };
        let Some(run) = self
            .workflow_run_model_optional_by_id(workflow_run_id)
            .await?
        else {
            return Ok(false);
        };
        if !matches!(
            run.status.as_str(),
            WORKFLOW_STATUS_CANCELLED | WORKFLOW_STATUS_FAILED | WORKFLOW_STATUS_SUPERSEDED
        ) {
            return Ok(false);
        }
        let message = format!(
            "所属画布运行 #{} 已{}",
            run.id,
            terminal_workflow_status_label(&run.status)
        );
        self.mark_generated_video_interrupted_validated(
            video.id,
            crate::video::prompt_id_from_generated_video(video),
            &message,
        )
        .await?;
        Ok(true)
    }

    pub async fn generated_images_for_video_prompt_references(
        &self,
        project_id: i64,
        chapter_id: i64,
    ) -> Result<Vec<generated_image::Model>, AppError> {
        self.video
            .generated_images_for_video_prompt_references(project_id, chapter_id)
            .await
    }

    pub async fn generated_images_by_ids(
        &self,
        image_ids: &[i64],
    ) -> Result<Vec<generated_image::Model>, AppError> {
        self.video.generated_images_by_ids(image_ids).await
    }

    pub async fn generated_images_for_project(
        &self,
        project_id: i64,
    ) -> Result<Vec<generated_image::Model>, AppError> {
        self.video.generated_images_for_project(project_id).await
    }

    pub async fn generated_images_for_chapter(
        &self,
        project_id: i64,
        chapter_id: i64,
    ) -> Result<Vec<generated_image::Model>, AppError> {
        self.video
            .generated_images_for_chapter(project_id, chapter_id)
            .await
    }

    pub async fn generated_images_for_valid_chapter(
        &self,
        project_id: i64,
        chapter_id: i64,
    ) -> Result<Vec<generated_image::Model>, AppError> {
        self.video
            .ensure_image_chapter_belongs_to_project(project_id, chapter_id)
            .await?;
        self.video
            .generated_images_for_chapter(project_id, chapter_id)
            .await
    }

    pub async fn video_asset_project_ids(&self, project_id: i64) -> Result<Vec<i64>, AppError> {
        self.video.video_asset_project_ids_record(project_id).await
    }

    pub async fn video_reference_candidate_images(
        &self,
        project_ids: &[i64],
        chapter_id: i64,
    ) -> Result<Vec<generated_image::Model>, AppError> {
        self.video
            .video_reference_candidate_image_records(project_ids, chapter_id)
            .await
    }

    pub async fn ensure_video_reference_images_available(
        &self,
        project_id: i64,
        chapter_id: i64,
        reference_image_ids_json: Option<&Value>,
    ) -> Result<(), AppError> {
        self.video
            .ensure_video_reference_images_available_record(
                project_id,
                chapter_id,
                reference_image_ids_json,
            )
            .await
    }

    pub async fn validated_video_reference_images(
        &self,
        project_id: i64,
        chapter_id: i64,
        reference_image_ids_json: Option<&Value>,
    ) -> Result<Vec<generated_image::Model>, AppError> {
        self.video
            .validated_video_reference_image_records(
                project_id,
                chapter_id,
                reference_image_ids_json,
            )
            .await
    }

    pub async fn generated_image_by_id(
        &self,
        image_id: i64,
    ) -> Result<Option<generated_image::Model>, AppError> {
        self.video.generated_image_by_id(image_id).await
    }

    pub async fn toggle_generated_image_favorite(
        &self,
        image_id: i64,
    ) -> Result<generated_image::Model, AppError> {
        self.video
            .toggle_generated_image_favorite_record(image_id)
            .await
    }

    pub async fn update_generated_image_record(
        &self,
        image_id: i64,
        input: UpdateGeneratedImageRecordInput,
    ) -> Result<generated_image::Model, AppError> {
        self.video
            .update_generated_image_record(image_id, input)
            .await
    }

    pub async fn delete_manga_page_image_records(
        &self,
        project_id: i64,
        chapter_id: i64,
        page_number: i32,
        preserve_image_id: Option<i64>,
    ) -> Result<Vec<String>, AppError> {
        self.video
            .delete_manga_page_image_records(project_id, chapter_id, page_number, preserve_image_id)
            .await
    }

    pub async fn video_tail_frame_images(&self) -> Result<Vec<generated_image::Model>, AppError> {
        self.video.video_tail_frame_images().await
    }

    pub async fn create_generated_image_record(
        &self,
        input: CreateGeneratedImageRecordDraftInput,
    ) -> Result<generated_image::Model, AppError> {
        self.video.create_generated_image_record(input).await
    }

    pub async fn create_generated_image_record_with_generation_run_checkpoint(
        &self,
        input: CreateGeneratedImageRecordDraftInput,
        generation_run: RecordCompletedGenerationRunWithCheckpointInput,
    ) -> Result<generated_image::Model, AppError> {
        self.video
            .create_generated_image_record_with_generation_run_checkpoint(input, generation_run)
            .await
    }

    pub async fn create_video_record(
        &self,
        input: CreateGeneratedVideoRecordDraftInput,
    ) -> Result<generated_video::Model, AppError> {
        self.video.create_generated_video_record(input).await
    }

    /// 内部写路径：无 project_id 门禁；对外请用 `*_validated`。
    pub(crate) async fn create_video_record_with_generation_run(
        &self,
        input: CreateGeneratedVideoRecordDraftInput,
        generation_run: RecordStartedGenerationRunInput,
    ) -> Result<generated_video::Model, AppError> {
        self.video
            .create_generated_video_record_with_generation_run(input, generation_run)
            .await
    }

    /// 创建视频记录并挂 generation_run：校验 project_id。
    pub async fn create_video_record_with_generation_run_validated(
        &self,
        input: CreateGeneratedVideoRecordDraftInput,
        generation_run: RecordStartedGenerationRunInput,
    ) -> Result<generated_video::Model, AppError> {
        crate::video::ensure_positive_project_id_for_video(input.project_id)
            .map_err(AppError::InvalidArgument)?;
        self.create_video_record_with_generation_run(input, generation_run)
            .await
    }

    /// 内部写路径：无 project/prompt 门禁；对外请用 `*_validated`。
    pub(crate) async fn create_prompt_video_record_with_generation_run(
        &self,
        input: CreateGeneratedVideoRecordDraftInput,
        generation_run: RecordStartedGenerationRunInput,
        prompt_id: i64,
    ) -> Result<generated_video::Model, AppError> {
        self.video
            .create_prompt_generated_video_record_with_generation_run(
                input,
                generation_run,
                prompt_id,
            )
            .await
    }

    /// 章节提示词视频创建：校验 project_id 与 prompt_id。
    pub async fn create_prompt_video_record_with_generation_run_validated(
        &self,
        input: CreateGeneratedVideoRecordDraftInput,
        generation_run: RecordStartedGenerationRunInput,
        prompt_id: i64,
    ) -> Result<generated_video::Model, AppError> {
        crate::video::ensure_positive_project_id_for_video(input.project_id)
            .map_err(AppError::InvalidArgument)?;
        crate::video::ensure_positive_video_prompt_id(prompt_id)
            .map_err(AppError::InvalidArgument)?;
        self.create_prompt_video_record_with_generation_run(input, generation_run, prompt_id)
            .await
    }

    pub async fn generated_videos_for_chapter(
        &self,
        project_id: i64,
        chapter_id: i64,
    ) -> Result<Vec<generated_video::Model>, AppError> {
        self.video
            .generated_videos_for_chapter(project_id, chapter_id)
            .await
    }

    pub async fn generated_video_by_id(
        &self,
        video_id: i64,
    ) -> Result<Option<generated_video::Model>, AppError> {
        self.video.generated_video_by_id(video_id).await
    }

    pub async fn resolve_video_config_for_provider_job(
        &self,
        project_id: Option<i64>,
        provider_kind: &str,
        provider_job_id: &str,
    ) -> Result<a4n_core::models::ai::video_gen_config::Model, AppError> {
        let provider_job_id = provider_job_id.trim();
        if provider_job_id.is_empty() {
            return Err(AppError::InvalidArgument(
                "provider_job_id 不能为空".to_string(),
            ));
        }
        let provider_kind = provider_kind.trim();
        if provider_kind.is_empty() {
            return Err(AppError::InvalidArgument(
                "provider_kind 不能为空".to_string(),
            ));
        }
        let videos = self
            .video
            .generated_videos_by_provider_job_id(project_id, provider_job_id)
            .await?;
        let video = match videos.as_slice() {
            [] => {
                return Err(AppError::NotFound(format!(
                    "找不到 provider job 本地 identity: {provider_job_id}"
                )));
            }
            [video] => video,
            _ => {
                return Err(AppError::InvalidState {
                    current: "duplicate_provider_job_identity".to_string(),
                    operation: "恢复视频 provider job".to_string(),
                });
            }
        };
        let config_id = crate::video::provider_poll_config_id(video)
            .map_err(AppError::ConfigError)
            .and_then(|id| {
                id.ok_or_else(|| AppError::InvalidState {
                    current: "missing_video_config_identity".to_string(),
                    operation: "恢复视频 provider job".to_string(),
                })
            })?;
        let config = self.video_config_by_id(config_id).await?;
        let capabilities =
            a4n_video::video_provider_capabilities(&config.backend).ok_or_else(|| {
                AppError::ConfigError(format!("未知视频生成后端: {}", config.backend))
            })?;
        if !capabilities.asynchronous {
            return Err(AppError::InvalidState {
                current: "synchronous_video_backend".to_string(),
                operation: "恢复异步视频 provider job".to_string(),
            });
        }
        // Existing persisted checkpoints use the domain marker `video`; new
        // checkpoints may carry the concrete backend and must match exactly.
        if provider_kind != "video" && provider_kind != config.backend {
            return Err(AppError::InvalidState {
                current: format!("provider_kind={provider_kind}"),
                operation: format!("恢复 backend={}", config.backend),
            });
        }
        Ok(config)
    }

    pub async fn toggle_generated_video_favorite(
        &self,
        video_id: i64,
    ) -> Result<generated_video::Model, AppError> {
        self.video
            .toggle_generated_video_favorite_record(video_id)
            .await
    }

    pub async fn update_generated_video_record(
        &self,
        video_id: i64,
        input: UpdateGeneratedVideoRecordInput,
    ) -> Result<generated_video::Model, AppError> {
        self.video
            .update_generated_video_record(video_id, input)
            .await
    }

    pub async fn update_generated_video_duration_record(
        &self,
        video_id: i64,
        duration_secs: f32,
    ) -> Result<generated_video::Model, AppError> {
        let updated = self
            .video
            .update_generated_video_duration_record(video_id, duration_secs)
            .await?;
        self.sync_project_asset_lineage_after_commit(updated.project_id, "同步视频实际时长")
            .await;
        Ok(updated)
    }

    pub async fn delete_generated_image_record_with_cleanup(
        &self,
        image_id: i64,
    ) -> Result<(), AppError> {
        self.video
            .delete_generated_image_record_with_cleanup(image_id)
            .await
    }

    pub async fn delete_generated_video_record_with_cleanup(
        &self,
        video_id: i64,
    ) -> Result<(), AppError> {
        self.video
            .delete_generated_video_record_with_cleanup(video_id)
            .await
    }

    pub async fn delete_missing_generated_video_references(
        &self,
        video_id: i64,
    ) -> Result<u64, AppError> {
        self.video
            .delete_missing_generated_video_references_record(video_id)
            .await
    }

    pub async fn missing_video_reference_project_ids(
        &self,
        video_id: i64,
    ) -> Result<Vec<i64>, AppError> {
        if let Some(video) = self.video.generated_video_by_id(video_id).await? {
            return Ok(vec![video.project_id]);
        }
        self.video
            .missing_video_reference_project_ids(video_id)
            .await
    }

    /// 内部写路径：无状态字面量门禁；对外请用 `*_validated`。
    pub(crate) async fn update_generated_video_waiting_status_and_prompt(
        &self,
        video_id: i64,
        prompt_id: Option<i64>,
        status: &str,
    ) -> Result<generated_video::Model, AppError> {
        self.video
            .update_generated_video_waiting_status_and_prompt_record(video_id, prompt_id, status)
            .await
    }

    /// 通用 error 状态写路径（导入失败等无 generation_run 合同场景）。
    /// provider 失败请用 `mark_generated_video_failed_validated`；中断请用 `mark_generated_video_interrupted_validated`。
    pub async fn mark_generated_video_error_status_and_prompt(
        &self,
        video_id: i64,
        prompt_id: Option<i64>,
        status: &str,
        message: &str,
    ) -> Result<generated_video::Model, AppError> {
        let updated = self
            .video
            .mark_generated_video_error_status_and_prompt_record(
                video_id, prompt_id, status, message,
            )
            .await?;
        if status == "failed" {
            self.sync_project_asset_lineage_after_commit(
                updated.project_id,
                "同步失败视频资产谱系",
            )
            .await;
        }
        Ok(updated)
    }

    /// 内部写路径：无 job id 门禁；对外请用 `*_validated`。
    pub(crate) async fn mark_generated_video_job_and_prompt(
        &self,
        video_id: i64,
        provider_job_id: &str,
        metadata_json: Option<Value>,
        prompt_id: Option<i64>,
    ) -> Result<generated_video::Model, AppError> {
        self.video
            .mark_generated_video_job_and_prompt_record(
                video_id,
                provider_job_id,
                metadata_json,
                prompt_id,
            )
            .await
    }

    /// provider 异步 job 提交后落库：校验 video_id 与非空 job id。
    /// Host 仍负责后续 wait/poll 与 workflow 钩子。
    pub async fn mark_generated_video_job_and_prompt_validated(
        &self,
        video_id: i64,
        provider_job_id: &str,
        metadata_json: Option<Value>,
        prompt_id: Option<i64>,
    ) -> Result<generated_video::Model, AppError> {
        crate::video::ensure_positive_generated_video_id(video_id)
            .map_err(AppError::InvalidArgument)?;
        let provider_job_id = crate::video::required_provider_job_id(provider_job_id)
            .map_err(AppError::InvalidArgument)?;
        self.mark_generated_video_job_and_prompt(
            video_id,
            provider_job_id,
            metadata_json,
            prompt_id,
        )
        .await
    }

    pub async fn mark_generated_video_completed_and_link_prompt(
        &self,
        video_id: i64,
        file_path: &str,
        thumbnail_path: Option<&str>,
        file_size: i64,
        metadata_json: Option<Value>,
        prompt_id: Option<i64>,
    ) -> Result<generated_video::Model, AppError> {
        let updated = self
            .video
            .mark_generated_video_completed_and_link_prompt_record(
                video_id,
                file_path,
                thumbnail_path,
                file_size,
                metadata_json,
                prompt_id,
            )
            .await?;
        self.sync_project_asset_lineage_after_commit(updated.project_id, "同步完成视频资产谱系")
            .await;
        Ok(updated)
    }

    /// poll/失败路径：校验后将视频标 failed，并 fail 关联 generation_run。
    /// Host 仍负责 workflow 钩子与文件清理。
    pub async fn mark_generated_video_failed_validated(
        &self,
        video_id: i64,
        prompt_id: Option<i64>,
        message: &str,
    ) -> Result<generated_video::Model, AppError> {
        crate::video::ensure_positive_generated_video_id(video_id)
            .map_err(AppError::InvalidArgument)?;
        let message = crate::video::required_generated_video_failure_message(message)
            .map_err(AppError::InvalidArgument)?;
        let updated = self
            .mark_generated_video_error_status_and_prompt(
                video_id,
                prompt_id,
                crate::video::GENERATED_VIDEO_ERROR_STATUS_FAILED,
                message,
            )
            .await?;
        let runs = self
            .generation
            .fail_generation_runs_for_target("generated_video", video_id, message.to_string(), None)
            .await?;
        crate::video::ensure_provider_generation_run_updated(
            video_id,
            runs.len(),
            "标记 provider 视频生成失败",
        )
        .map_err(AppError::InvalidArgument)?;
        Ok(updated)
    }

    /// provider 提交前由用户取消：视频、提示词和关联 generation_run 统一标 cancelled。
    pub async fn mark_generated_video_cancelled_validated(
        &self,
        video_id: i64,
        prompt_id: Option<i64>,
        message: &str,
    ) -> Result<generated_video::Model, AppError> {
        crate::video::ensure_positive_generated_video_id(video_id)
            .map_err(AppError::InvalidArgument)?;
        let message = crate::video::required_generated_video_failure_message(message)
            .map_err(AppError::InvalidArgument)?;
        let updated = self
            .mark_generated_video_error_status_and_prompt(
                video_id,
                prompt_id,
                crate::video::GENERATED_VIDEO_STATUS_CANCELLED,
                message,
            )
            .await?;
        let runs = self
            .generation
            .cancel_generation_runs_for_target("generated_video", video_id, message.to_string())
            .await?;
        crate::video::ensure_provider_generation_run_updated(
            video_id,
            runs.len(),
            "取消 provider 视频生成",
        )
        .map_err(AppError::InvalidArgument)?;
        Ok(updated)
    }

    /// poll 等待态：仅允许 pending/running，再更新视频与提示词状态。
    /// Host 仍负责 workflow running 钩子。
    pub async fn update_generated_video_waiting_status_validated(
        &self,
        video_id: i64,
        prompt_id: Option<i64>,
        status: &str,
    ) -> Result<generated_video::Model, AppError> {
        crate::video::ensure_positive_generated_video_id(video_id)
            .map_err(AppError::InvalidArgument)?;
        crate::video::validate_generated_video_waiting_status(status)
            .map_err(AppError::InvalidArgument)?;
        self.update_generated_video_waiting_status_and_prompt(video_id, prompt_id, status)
            .await
    }

    /// 中断轮询/取消：校验后标 interrupted（不强制 generation_run）。
    /// Host 仍负责 workflow interrupted 钩子。
    pub async fn mark_generated_video_interrupted_validated(
        &self,
        video_id: i64,
        prompt_id: Option<i64>,
        message: &str,
    ) -> Result<generated_video::Model, AppError> {
        crate::video::ensure_positive_generated_video_id(video_id)
            .map_err(AppError::InvalidArgument)?;
        let message = crate::video::required_generated_video_failure_message(message)
            .map_err(AppError::InvalidArgument)?;
        self.mark_generated_video_error_status_and_prompt(
            video_id,
            prompt_id,
            crate::video::GENERATED_VIDEO_ERROR_STATUS_INTERRUPTED,
            message,
        )
        .await
    }

    /// 完成 provider 视频后：complete 关联 generation_run，并要求至少一条。
    pub async fn complete_generated_video_provider_runs_validated(
        &self,
        video: &generated_video::Model,
    ) -> Result<(), AppError> {
        crate::video::ensure_positive_generated_video_id(video.id)
            .map_err(AppError::InvalidArgument)?;
        let runs = self
            .generation
            .complete_generation_runs_for_target(
                "generated_video",
                video.id,
                Some(serde_json::json!({
                    "generated_video_id": video.id,
                    "file_path": video.file_path,
                    "thumbnail_path": video.thumbnail_path,
                    "file_size": video.file_size,
                    "status": video.status,
                    "provider_result": video.metadata_json,
                })),
            )
            .await?;
        crate::video::ensure_provider_generation_run_updated(
            video.id,
            runs.len(),
            "完成 provider 视频生成",
        )
        .map_err(AppError::InvalidArgument)?;
        Ok(())
    }

    pub async fn chapter_video_prompt(
        &self,
        prompt_id: i64,
    ) -> Result<chapter_video_prompt::Model, AppError> {
        self.video
            .chapter_video_prompt_by_id(prompt_id)
            .await?
            .ok_or_else(|| AppError::NotFound("视频提示词不存在".to_string()))
    }

    pub async fn chapter_video_prompts_for_chapter(
        &self,
        project_id: i64,
        chapter_id: i64,
    ) -> Result<Vec<chapter_video_prompt::Model>, AppError> {
        self.video
            .chapter_video_prompts_for_chapter(project_id, chapter_id)
            .await
    }

    pub async fn chapter_video_prompts_for_batch_generation(
        &self,
        prompt_ids: &[i64],
    ) -> Result<Vec<chapter_video_prompt::Model>, AppError> {
        self.video
            .chapter_video_prompts_for_batch_generation(prompt_ids)
            .await
    }

    pub async fn update_chapter_video_prompt_with_policy(
        &self,
        prompt_id: i64,
        input: ChapterVideoPromptPatch,
        allow_downstream: bool,
    ) -> Result<chapter_video_prompt::Model, AppError> {
        self.video
            .update_chapter_video_prompt_with_policy(prompt_id, input, allow_downstream)
            .await
    }

    pub async fn update_chapter_video_segment_recipe(
        &self,
        prompt_id: i64,
        recipe: VideoSegmentExecutionRecipeV1,
    ) -> Result<Vec<VideoSegmentExecutionRecipeV1>, AppError> {
        self.video
            .update_chapter_video_segment_recipe(prompt_id, recipe)
            .await
    }

    pub async fn update_chapter_video_prompt_with_generation_run_checkpoint(
        &self,
        prompt_id: i64,
        input: ChapterVideoPromptPatch,
        metadata_json: Option<Value>,
        allow_downstream: bool,
        generation_run: RecordCompletedGenerationRunWithCheckpointInput,
    ) -> Result<chapter_video_prompt::Model, AppError> {
        self.video
            .update_chapter_video_prompt_with_generation_run_checkpoint(
                prompt_id,
                input,
                metadata_json,
                allow_downstream,
                generation_run,
            )
            .await
    }

    pub async fn delete_chapter_video_prompt_checked(
        &self,
        prompt_id: i64,
        allow_downstream: bool,
    ) -> Result<(), AppError> {
        self.video
            .delete_chapter_video_prompt_with_policy(prompt_id, allow_downstream)
            .await
    }

    pub async fn delete_chapter_video_prompts_checked(
        &self,
        prompt_ids: &[i64],
        allow_downstream: bool,
    ) -> Result<u64, AppError> {
        self.video
            .delete_chapter_video_prompts_with_policy(prompt_ids, allow_downstream)
            .await
    }

    pub async fn prompt_downstream_dependencies(
        &self,
        prompt_id: i64,
    ) -> Result<VideoPromptDownstreamDependencies, AppError> {
        self.video.prompt_downstream_dependencies(prompt_id).await
    }

    pub async fn ensure_frame_prompt_image_generation_allowed(
        &self,
        prompt_id: i64,
        generation_source: &str,
    ) -> Result<(), AppError> {
        self.video
            .ensure_frame_prompt_image_generation_allowed(prompt_id, generation_source)
            .await
    }

    pub async fn effective_frame_prompt_image_reference_ids(
        &self,
        prompt_id: i64,
        generation_source: &str,
        requested_reference_image_ids: &[i64],
    ) -> Result<Vec<i64>, AppError> {
        self.video
            .effective_frame_prompt_image_reference_ids(
                prompt_id,
                generation_source,
                requested_reference_image_ids,
            )
            .await
    }

    pub(crate) async fn ensure_prompt_stage_mutable(
        &self,
        prompt: &chapter_video_prompt::Model,
        operation: &str,
    ) -> Result<(), AppError> {
        self.video
            .ensure_chapter_video_prompt_stage_mutable(prompt, operation)
            .await
    }

    pub(crate) async fn ensure_prompt_stage_mutable_ignoring(
        &self,
        prompt: &chapter_video_prompt::Model,
        operation: &str,
        ignored_prompt_ids: &BTreeSet<i64>,
    ) -> Result<(), AppError> {
        self.video
            .ensure_chapter_video_prompt_stage_mutable_ignoring(
                prompt,
                operation,
                ignored_prompt_ids,
            )
            .await
    }

    pub(crate) async fn frame_prompt_has_generated_image(
        &self,
        prompt: &chapter_video_prompt::Model,
    ) -> Result<bool, AppError> {
        self.video
            .chapter_video_prompt_has_generated_frame_image(prompt)
            .await
    }

    /// 章节视频提示词生成：校验后创建 workflow_run（RUNNING）。
    /// Host 仍负责 LLM 分阶段与提示词落库。
    pub async fn begin_video_prompt_workflow_validated(
        &self,
        workflow_kind: &str,
        project_id: i64,
        chapter_id: i64,
        workflow_mode: &str,
        workflow_step_key: &str,
        source: &crate::video::ChapterVideoSource,
        options: &crate::video::ChapterVideoPromptOptions,
        active_task_id: &str,
        root_project_id: i64,
        retry_of_task_id: Option<&str>,
    ) -> Result<a4n_core::models::generation::workflow_run::Model, AppError> {
        let (input, _fingerprint) = crate::video::video_prompt_workflow_run_input(
            workflow_kind,
            project_id,
            chapter_id,
            workflow_mode,
            workflow_step_key,
            source,
            options,
            active_task_id,
            root_project_id,
        )
        .map_err(AppError::InvalidArgument)?;
        self.claim_exclusive_workflow_run(crate::workflow::ClaimExclusiveWorkflowRunInput {
            run: input,
            resume_from_task_id: retry_of_task_id.map(str::to_owned),
        })
        .await
    }

    /// 视频提示词优化：校验后创建 workflow_run。
    pub async fn begin_video_prompt_optimization_workflow_validated(
        &self,
        project_id: i64,
        prompt_id: i64,
        chapter_id: i64,
        workflow_mode: &str,
        current_step_key: &str,
        input_fingerprint_json: serde_json::Value,
    ) -> Result<a4n_core::models::generation::workflow_run::Model, AppError> {
        let input = crate::video::video_prompt_optimization_workflow_run_input(
            project_id,
            prompt_id,
            chapter_id,
            workflow_mode,
            current_step_key,
            input_fingerprint_json,
        )
        .map_err(AppError::InvalidArgument)?;
        self.begin_workflow_run_validated(input).await
    }

    /// 视频 provider 轮询/状态：校验后 upsert workflow_run（状态由调用方指定）。
    pub async fn begin_video_provider_workflow_validated(
        &self,
        video: &a4n_core::models::ai::generated_video::Model,
        job_id: &str,
        status: &str,
        can_resume: bool,
        error_message: Option<&str>,
    ) -> Result<a4n_core::models::generation::workflow_run::Model, AppError> {
        let input = crate::video::video_provider_workflow_run_input(
            video,
            job_id,
            status,
            can_resume,
            error_message,
        )
        .map_err(AppError::InvalidArgument)?;
        self.upsert_workflow_run(input).await
    }

    /// 视频 provider 进度：校验后创建/恢复 RUNNING workflow_run。
    pub async fn begin_video_provider_progress_workflow_validated(
        &self,
        video: &a4n_core::models::ai::generated_video::Model,
        job_id: &str,
    ) -> Result<a4n_core::models::generation::workflow_run::Model, AppError> {
        let business_key = crate::video::video_provider_workflow_business_key(video.id)
            .map_err(AppError::InvalidArgument)?;
        if let Some(run) = self
            .latest_workflow_run_by_business_key(&business_key)
            .await?
        {
            return self
                .mark_workflow_status(run.id, crate::workflow::WORKFLOW_STATUS_RUNNING, true, None)
                .await;
        }
        let input = crate::video::video_provider_progress_workflow_run_input(video, job_id)
            .map_err(AppError::InvalidArgument)?;
        self.begin_workflow_run_validated(input).await
    }

    /// provider 轮询进度及 Canvas 节点投影的唯一写入入口。
    pub async fn record_video_provider_progress_validated(
        &self,
        video: &generated_video::Model,
        current: u32,
        total: u32,
        message: &str,
    ) -> Result<i64, AppError> {
        if crate::video::is_terminal_generated_video_status(&video.status) {
            return Err(AppError::InvalidState {
                current: video.status.clone(),
                operation: "记录已终态视频 provider 进度".to_string(),
            });
        }
        let job_id = crate::video::normalized_provider_job_id(video.provider_job_id.as_deref())
            .ok_or_else(|| AppError::InvalidArgument("视频缺少 provider_job_id".to_string()))?;
        let run = self
            .begin_video_provider_progress_workflow_validated(video, job_id)
            .await?;
        self.record_workflow_progress_event(WorkflowProgressEventInput {
            workflow_run_id: run.id,
            task_id: None,
            current,
            total,
            message,
            meta: None,
        })
        .await?;

        let Some(canvas) = video
            .metadata_json
            .as_ref()
            .and_then(|metadata| metadata.get("canvas"))
        else {
            return Ok(run.id);
        };
        let workflow_run_id = canvas
            .get("workflow_run_id")
            .and_then(Value::as_i64)
            .filter(|id| *id > 0)
            .ok_or_else(|| {
                AppError::InvalidArgument("画布视频缺少有效 workflow_run_id".to_string())
            })?;
        let graph_id = canvas
            .get("graph_id")
            .and_then(Value::as_i64)
            .filter(|id| *id > 0)
            .ok_or_else(|| AppError::InvalidArgument("画布视频缺少有效 graph_id".to_string()))?;
        let node_id = canvas
            .get("node_id")
            .and_then(Value::as_i64)
            .filter(|id| *id > 0)
            .ok_or_else(|| AppError::InvalidArgument("画布视频缺少有效 node_id".to_string()))?;
        let step_key = format!("canvas_node:{node_id}");
        self.mark_workflow_current_step(workflow_run_id, &step_key)
            .await?;
        self.upsert_workflow_checkpoint(UpsertWorkflowCheckpointInput {
            workflow_run_id,
            step_key,
            step_order: 0,
            status: WORKFLOW_STATUS_RUNNING.to_string(),
            input_fingerprint_sha256: None,
            input_fingerprint_json: Some(json!({
                "generated_video_id": video.id,
                "provider_job_id": video.provider_job_id,
                "graph_id": graph_id,
                "node_id": node_id,
            })),
            checkpoint_json: None,
            artifacts_json: Some(json!({
                "data_type": "video",
                "generated_video_id": video.id,
                "status": video.status,
                "provider_job_id": video.provider_job_id,
            })),
            generation_run_ids_json: None,
            error_message: None,
            completed: false,
        })
        .await?;
        self.record_workflow_progress_event(WorkflowProgressEventInput {
            workflow_run_id,
            task_id: None,
            current,
            total,
            message,
            meta: None,
        })
        .await?;
        Ok(run.id)
    }

    /// 视频 provider workflow：统一写入 run、checkpoint 与 generation_run 关联。
    pub async fn mark_video_provider_workflow_validated(
        &self,
        video: &generated_video::Model,
        status: &str,
        can_resume: bool,
        error_message: Option<&str>,
    ) -> Result<(), AppError> {
        let Some(job_id) =
            crate::video::normalized_provider_job_id(video.provider_job_id.as_deref())
        else {
            return Ok(());
        };
        let fingerprint = crate::video::video_provider_workflow_fingerprint(video, job_id);
        let fingerprint_sha = source_content_sha256(&serde_json::to_string(&fingerprint)?);
        let artifacts = json!({
            "targets": [{
                "target_table": "generated_video",
                "target_id": video.id,
                "target_kind": "video_provider_job",
                "role": "provider_job",
                "provider_job_id": job_id,
            }],
        });
        let business_key = crate::video::video_provider_workflow_business_key(video.id)
            .map_err(AppError::InvalidArgument)?;
        let run = match self
            .latest_workflow_run_by_business_key(&business_key)
            .await?
        {
            Some(run)
                if run.status == status
                    && matches!(
                        status,
                        crate::workflow::WORKFLOW_STATUS_COMPLETED
                            | crate::workflow::WORKFLOW_STATUS_FAILED
                            | crate::workflow::WORKFLOW_STATUS_CANCELLED
                            | crate::workflow::WORKFLOW_STATUS_SUPERSEDED
                    ) =>
            {
                return Ok(());
            }
            Some(run) => run,
            None => {
                self.begin_video_provider_workflow_validated(
                    video,
                    job_id,
                    crate::workflow::WORKFLOW_STATUS_RUNNING,
                    true,
                    None,
                )
                .await?
            }
        };
        self.upsert_workflow_checkpoint(a4n_db::workflow::UpsertWorkflowCheckpointInput {
            workflow_run_id: run.id,
            step_key: "provider_job".to_string(),
            step_order: 1,
            status: status.to_string(),
            input_fingerprint_sha256: Some(fingerprint_sha.clone()),
            input_fingerprint_json: Some(fingerprint.clone()),
            checkpoint_json: Some(json!({
                "provider_job_id": job_id,
                "generated_video_id": video.id,
                "status": video.status,
            })),
            artifacts_json: Some(artifacts.clone()),
            generation_run_ids_json: None,
            error_message: error_message.map(str::to_string),
            completed: matches!(
                status,
                crate::workflow::WORKFLOW_STATUS_COMPLETED
                    | crate::workflow::WORKFLOW_STATUS_FAILED
            ),
        })
        .await?;
        self.attach_generation_runs_to_workflow_for_target(
            a4n_db::generation::AttachGenerationRunsToWorkflowInput {
                target_table: "generated_video".to_string(),
                target_id: video.id,
                workflow_run_id: run.id,
                call_key: Some("provider_job".to_string()),
                input_fingerprint_sha256: Some(fingerprint_sha),
                input_fingerprint_json: Some(fingerprint),
                artifact_refs_json: Some(artifacts),
            },
        )
        .await?;
        self.mark_workflow_status(run.id, status, can_resume, error_message)
            .await?;
        Ok(())
    }
}
