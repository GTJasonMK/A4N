use a4n_error::AppError;
use std::collections::{HashMap, HashSet};

use a4n_db_models::models::{
    ai::{generated_image, generated_video},
    chapter::chapter_video_prompt,
};
use a4n_domain::source_spans::metadata_source_content_sha256;
use a4n_domain::{
    AssemblyPostprocessOperation, CanvasFrameKind, EXTERNAL_IMPORT_MODEL,
    image::{
        IMAGE_GENERATION_SOURCE_VIDEO_TAIL_FRAME_CAPTURE, IMAGE_TYPE_CANVAS_FRAME,
        IMAGE_TYPE_VIDEO_TAIL_FRAME,
    },
    source_text_hash,
    video_workflow::{
        WORKFLOW_MODE_CANVAS_ASSEMBLY, WORKFLOW_MODE_CANVAS_TRIM, WORKFLOW_MODE_CANVAS_VIDEO,
        WORKFLOW_MODE_MIXED, is_assembly_video_workflow_mode, is_generated_video_workflow_mode,
    },
};
use a4n_provider_contract::VideoProviderConfig;
use a4n_video::{
    canvas_frame_timestamp, imported_chapter_video_metadata, trimmed_video_metadata,
    validate_video_trim_range,
};
use a4n_video_contract::{
    ImportedVideoProperties, VideoProviderHostPort, VideoProviderPollTarget,
    VideoSegmentWorkflowModeV1,
};
use async_trait::async_trait;
use chrono::Utc;
use serde_json::Value;
use tokio_util::sync::CancellationToken;

pub use a4n_api_contract::video::{
    AssemblyPushAction, SelectSegmentVideoInput, UpdateAssemblyPushStatusInput,
    UpdateGeneratedVideoInput,
};
use a4n_api_contract::{
    image::GeneratedImageSummary,
    video::{
        AssemblyPostprocessEditRecordInput, AssemblyPostprocessPlan, AssemblyPostprocessSourcePlan,
        AssemblyPushClipInput, CompleteVideoRecordInput, GeneratedVideoSummary,
        VideoProviderExecutionApplicationPort, VideoRecordCompletion, VideoRecordCompletionError,
        VideoTailFrameInput, VideoTailFrameSummary,
    },
};

use super::{
    ASSEMBLY_PUSH_STATUS_ACCEPTED, ASSEMBLY_PUSH_STATUS_PENDING, ASSEMBLY_PUSH_STATUS_REJECTED,
    AssemblyClipOrderAnchor, active_assembly_push_source_ids, assembly_clip_display_shot_number,
    assembly_clip_insert_after_video_id, assembly_clip_sort_order,
    assembly_insert_metadata_with_position, assembly_media_properties,
    assembly_push_metadata_with_status, assembly_push_records,
    ensure_insert_position_not_inside_pushed_prefix, is_assembly_clip_video_type,
    is_canvas_video_record, latest_non_rejected_push_index, next_assembly_insert_sort_order,
    parse_assembly_push_metadata, parse_postprocess_edit_metadata, postprocess_edit_timing,
    selected_assembly_push_end_index, tail_frame_metadata_belongs_to_video,
    validate_assembly_workflow_mode, validate_reference_media_import_options, video_shot_number,
};

/// Host-facing video record boundary. Record authorization, projection and
/// lifecycle validation stay in the video owner; Runtime supplies only the
/// tail-frame cleanup adapter required by deletion.
#[async_trait]
pub trait VideoRecordApplicationPort: Send + Sync {
    async fn create_imported_chapter_video_record(
        &self,
        project_id: i64,
        chapter_id: i64,
        prompt_id: i64,
        properties: ImportedVideoProperties,
        source_file_name: &str,
        source_extension: &str,
        imported_at: &str,
    ) -> Result<VideoImportRecordSummary, AppError>;

    async fn create_imported_reference_media_video_record(
        &self,
        project_id: i64,
        chapter_id: i64,
        properties: ImportedVideoProperties,
        source_file_name: &str,
        source_extension: &str,
        imported_at: &str,
        segment_plan_prompt_id: i64,
        target_source_shot_id: &str,
        target_segment_index: i32,
    ) -> Result<VideoImportRecordSummary, AppError>;

    async fn create_imported_assembly_insert_video_record(
        &self,
        project_id: i64,
        chapter_id: i64,
        workflow_mode: &str,
        insert_after_video_id: Option<i64>,
        properties: ImportedVideoProperties,
        source_file_name: &str,
        source_extension: &str,
        imported_at: &str,
    ) -> Result<VideoImportRecordSummary, AppError>;

    async fn create_imported_canvas_video_record(
        &self,
        project_id: i64,
        properties: ImportedVideoProperties,
        source_file_name: &str,
        source_extension: &str,
        imported_at: &str,
    ) -> Result<VideoImportRecordSummary, AppError>;

    async fn assembly_records(
        &self,
        project_id: i64,
        chapter_id: i64,
        workflow_mode: &str,
        operation: &str,
    ) -> Result<AssemblyVideoRecords, AppError>;

    async fn prepare_assembly_postprocess_edit(
        &self,
        push_video_id: i64,
        operation: AssemblyPostprocessOperation,
    ) -> Result<AssemblyPostprocessPlan, AppError>;

    async fn prepare_assembly_postprocess_source(
        &self,
        plan: &AssemblyPostprocessPlan,
        source_video_id: Option<i64>,
        has_uploaded_source: bool,
    ) -> Result<AssemblyPostprocessSourcePlan, AppError>;

    async fn create_assembly_postprocess_edit_record(
        &self,
        plan: AssemblyPostprocessPlan,
        input: AssemblyPostprocessEditRecordInput,
    ) -> Result<PreparedAssemblyPostprocessEditRecord, AppError>;

    async fn create_assembly_push_record(
        &self,
        project_id: i64,
        chapter_id: i64,
        workflow_mode: &str,
        clips: Vec<AssemblyPushClipInput>,
        end_video_id: Option<i64>,
    ) -> Result<PreparedAssemblyPushRecord, AppError>;

    async fn mark_error_status_and_prompt(
        &self,
        video_id: i64,
        prompt_id: Option<i64>,
        status: &str,
        message: &str,
    ) -> Result<GeneratedVideoSummary, AppError>;

    async fn playback_source(
        &self,
        user_id: i64,
        video_id: i64,
    ) -> Result<GeneratedVideoSummary, AppError>;

    async fn list_by_chapter(
        &self,
        user_id: i64,
        project_id: i64,
        chapter_id: i64,
    ) -> Result<Vec<GeneratedVideoSummary>, AppError>;

    async fn validated_reference_images(
        &self,
        user_id: i64,
        project_id: i64,
        chapter_id: i64,
        reference_image_ids_json: Option<&Value>,
    ) -> Result<Vec<GeneratedImageSummary>, AppError>;

    async fn create_video_tail_frame(
        &self,
        input: VideoTailFrameInput,
    ) -> Result<VideoTailFrameSummary, AppError>;

    async fn delete_video_tail_frame_assets(&self, video_id: i64) -> Result<(), AppError>;

    async fn prepare_canvas_assembly(
        &self,
        project_id: i64,
        video_ids: Vec<i64>,
        title: Option<&str>,
    ) -> Result<CanvasAssemblyPlan, AppError>;

    async fn create_canvas_assembly_record(
        &self,
        project_id: i64,
        plan: CanvasAssemblyPlan,
    ) -> Result<GeneratedVideoSummary, AppError>;

    async fn prepare_canvas_frame(
        &self,
        project_id: i64,
        video_id: i64,
        frame_kind: CanvasFrameKind,
        at_secs: Option<f32>,
    ) -> Result<CanvasFramePlan, AppError>;

    async fn create_canvas_frame_record(
        &self,
        plan: CanvasFramePlan,
        file_path: String,
        file_size: i64,
    ) -> Result<GeneratedImageSummary, AppError>;

    async fn update(
        &self,
        user_id: i64,
        video_id: i64,
        input: UpdateGeneratedVideoInput,
    ) -> Result<GeneratedVideoSummary, AppError>;

    async fn prepare_trim(
        &self,
        user_id: i64,
        video_id: i64,
        start_secs: f32,
        end_secs: f32,
    ) -> Result<VideoTrimPlan, AppError>;

    async fn create_trimmed_record(
        &self,
        plan: VideoTrimPlan,
    ) -> Result<GeneratedVideoSummary, AppError>;

    async fn complete(
        &self,
        input: CompleteVideoRecordInput,
    ) -> Result<VideoRecordCompletion, VideoRecordCompletionError>;

    async fn select_segment(
        &self,
        user_id: i64,
        project_id: i64,
        chapter_id: i64,
        video_id: i64,
        input: SelectSegmentVideoInput,
    ) -> Result<GeneratedVideoSummary, AppError>;

    async fn update_assembly_push_status(
        &self,
        user_id: i64,
        video_id: i64,
        input: UpdateAssemblyPushStatusInput,
    ) -> Result<GeneratedVideoSummary, AppError>;

    async fn delete(&self, user_id: i64, video_id: i64) -> Result<(), AppError>;

    async fn delete_record_with_cleanup(&self, video_id: i64) -> Result<(), AppError>;
}

#[derive(Debug, Clone)]
pub struct AssemblyVideoRecords {
    pub workflow_mode: String,
    pub clips: Vec<AssemblyClipRecord>,
}

#[derive(Debug, Clone)]
pub struct AssemblyClipRecord {
    pub video_id: i64,
    pub video_type: String,
    pub shot_number: i32,
    pub sort_order: f64,
    pub insert_after_video_id: Option<i64>,
    pub file_path: String,
    pub duration_secs: f32,
    pub width: i32,
    pub height: i32,
    pub fps: i32,
}

#[derive(Debug, Clone)]
pub struct PreparedAssemblyPushRecord {
    pub video: GeneratedVideoSummary,
    pub source_video_ids: Vec<i64>,
    pub push_index: i32,
}

#[derive(Debug, Clone)]
pub struct PreparedImportedChapterVideoRecord {
    pub video: generated_video::Model,
    pub prompt: chapter_video_prompt::Model,
}

#[derive(Debug, Clone)]
pub struct VideoImportRecordSummary {
    pub id: i64,
    pub project_id: i64,
    pub chapter_id: Option<i64>,
    pub video_type: String,
    pub workflow_mode: Option<String>,
    pub prompt_id: Option<i64>,
}

impl VideoImportRecordSummary {
    fn from_record(video: &generated_video::Model, prompt_id: Option<i64>) -> Self {
        Self {
            id: video.id,
            project_id: video.project_id,
            chapter_id: video.chapter_id,
            video_type: video.video_type.clone(),
            workflow_mode: video.workflow_mode.clone(),
            prompt_id,
        }
    }
}

#[derive(Debug, Clone)]
pub struct CanvasAssemblyPlan {
    pub sources: Vec<GeneratedVideoSummary>,
    pub title: String,
    pub width: i32,
    pub height: i32,
    pub duration_secs: f32,
    pub fps: i32,
}

#[derive(Debug, Clone)]
pub struct CanvasFramePlan {
    pub video: GeneratedVideoSummary,
    pub frame_kind: CanvasFrameKind,
    pub timestamp: f32,
}

#[derive(Debug, Clone)]
pub struct VideoTrimPlan {
    pub source: GeneratedVideoSummary,
    pub start_secs: f32,
    pub end_secs: f32,
    pub target_workflow_mode: String,
}

#[derive(Debug, Clone)]
pub struct PreparedAssemblyPostprocessEditRecord {
    pub video: GeneratedVideoSummary,
    pub base: GeneratedVideoSummary,
    pub operation: AssemblyPostprocessOperation,
    pub start_secs: f32,
    pub end_secs: f32,
}

#[derive(Debug, Clone)]
pub struct VideoGenerationRecordInput {
    pub project_id: i64,
    pub chapter_id: Option<i64>,
    pub video_type: String,
    pub workflow_mode: Option<String>,
    pub prompt: String,
    pub negative_prompt: Option<String>,
    pub model: String,
    pub width: i32,
    pub height: i32,
    pub duration_secs: f32,
    pub fps: i32,
    pub source_video_id: Option<i64>,
    pub source_image_id: Option<i64>,
    pub metadata_json: Option<Value>,
    pub provider_request_snapshot: Value,
    pub provider_backend: String,
    pub step_key: String,
    pub title: String,
    pub prompt_template_name: Option<String>,
    pub prompt_id: Option<i64>,
}

pub fn generated_video_summary(video: &generated_video::Model) -> GeneratedVideoSummary {
    GeneratedVideoSummary {
        id: video.id,
        project_id: video.project_id,
        chapter_id: video.chapter_id,
        video_type: video.video_type.clone(),
        workflow_mode: video.workflow_mode.clone(),
        file_path: video.file_path.clone(),
        thumbnail_path: video.thumbnail_path.clone(),
        prompt: video.prompt.clone(),
        negative_prompt: video.negative_prompt.clone(),
        model: video.model.clone(),
        width: video.width,
        height: video.height,
        duration_secs: video.duration_secs,
        fps: video.fps,
        file_size: video.file_size,
        source_video_id: video.source_video_id,
        source_image_id: video.source_image_id,
        status: video.status.clone(),
        provider_job_id: video.provider_job_id.clone(),
        error_message: video.error_message.clone(),
        metadata_json: video.metadata_json.clone(),
        timeline_item_id: video.timeline_item_id,
        is_favorite: video.is_favorite,
        created_at: video.created_at.to_string(),
        updated_at: video.updated_at.to_string(),
    }
}

impl crate::OrchestratorContext {
    pub(crate) async fn create_video_generation_record_validated(
        &self,
        input: VideoGenerationRecordInput,
    ) -> Result<generated_video::Model, a4n_error::AppError> {
        let video_record = a4n_db::video::CreateGeneratedVideoRecordDraftInput {
            project_id: input.project_id,
            chapter_id: input.chapter_id,
            video_type: input.video_type,
            workflow_mode: input.workflow_mode,
            prompt: input.prompt,
            negative_prompt: input.negative_prompt,
            model: input.model,
            width: input.width,
            height: input.height,
            duration_secs: input.duration_secs,
            fps: input.fps,
            source_video_id: input.source_video_id,
            source_image_id: input.source_image_id,
            provider_job_id: None,
            metadata_json: input.metadata_json,
        };
        let generation_run = crate::video::started_video_generation_run_input(
            &video_record,
            &input.step_key,
            &input.title,
            input.provider_backend,
            input.prompt_template_name.as_deref(),
            input.provider_request_snapshot,
        );
        match input.prompt_id {
            Some(prompt_id) => {
                self.create_prompt_video_record_with_generation_run_validated(
                    video_record,
                    generation_run,
                    prompt_id,
                )
                .await
            }
            None => {
                self.create_video_record_with_generation_run_validated(video_record, generation_run)
                    .await
            }
        }
    }

    pub async fn list_video_records_by_chapter_validated(
        &self,
        project_id: i64,
        chapter_id: i64,
    ) -> Result<Vec<generated_video::Model>, AppError> {
        self.project_chapter_video_source(project_id, chapter_id, "查看章节视频", false)
            .await?;
        self.generated_videos_for_chapter(project_id, chapter_id)
            .await
    }

    pub async fn select_segment_video_validated(
        &self,
        project_id: i64,
        chapter_id: i64,
        segment_plan_prompt_id: i64,
        target: (&str, i32),
        video_id: i64,
    ) -> Result<generated_video::Model, AppError> {
        self.project_chapter_video_source(project_id, chapter_id, "采用片段视频", true)
            .await?;
        let (plan_result, selected_result) = futures::join!(
            self.require_current_shared_segment_plan(
                project_id,
                chapter_id,
                segment_plan_prompt_id
            ),
            self.required_video_record(video_id),
        );
        let plan = plan_result?;
        let shot_number = a4n_video_contract::resolve_video_segment_number(
            plan.metadata_json.as_ref(),
            target.0,
            target.1,
        )
        .map_err(AppError::InvalidArgument)?;
        let mut recipe = a4n_video_contract::video_segment_recipe_from_metadata(
            plan.metadata_json.as_ref(),
            shot_number,
        )
        .map_err(AppError::InvalidArgument)?
        .ok_or_else(|| {
            AppError::InvalidArgument(format!("当前规划缺少片段 {shot_number} 的执行方案"))
        })?;
        let selected = selected_result?;
        let plan_identity = plan
            .metadata_json
            .as_ref()
            .and_then(|metadata| metadata.get("video_execution_identity"))
            .ok_or_else(|| AppError::InvalidArgument("当前片段规划缺少执行身份".to_string()))?;
        let expected_identity = super::prompt_generation::video_segment_execution_identity(
            plan_identity.clone(),
            Some(plan.id),
            Some(&recipe),
            target,
        )?;
        if selected.project_id != project_id
            || selected.chapter_id != Some(chapter_id)
            || selected.status != a4n_domain::video_workflow::VIDEO_STATUS_COMPLETED
            || selected
                .file_path
                .as_deref()
                .map(str::trim)
                .is_none_or(str::is_empty)
            || selected.workflow_mode.as_deref() != Some(recipe.workflow_mode.as_str())
            || !super::prompt_records::video_execution_segment_lineage_matches(
                selected.metadata_json.as_ref(),
                &expected_identity,
            )
        {
            return Err(AppError::InvalidArgument(format!(
                "视频 #{video_id} 不属于当前规划片段 {shot_number} 的所选执行方案"
            )));
        }
        recipe.adopted_video_id = Some(video_id);
        self.update_chapter_video_segment_recipe(plan.id, recipe)
            .await?;
        Ok(selected)
    }

    pub async fn update_video_record_validated(
        &self,
        video_id: i64,
        input: UpdateGeneratedVideoInput,
    ) -> Result<generated_video::Model, AppError> {
        if input.is_empty() {
            return Err(AppError::InvalidArgument(
                "视频更新内容不能为空".to_string(),
            ));
        }
        let existing = self.required_video_record(video_id).await?;
        let modifies_pushed_source = input.prompt.is_some() || input.negative_prompt.is_some();
        if modifies_pushed_source && self.video_record_is_pushed_source(&existing).await? {
            return Err(AppError::InvalidArgument(
                "该视频片段已推送到成片后处理，不能修改".to_string(),
            ));
        }

        let (negative_prompt, clear_negative_prompt) = match input.negative_prompt {
            Some(Some(value)) => (Some(value), false),
            Some(None) => (None, true),
            None => (None, false),
        };

        self.update_generated_video_record(
            video_id,
            a4n_db::video::UpdateGeneratedVideoRecordInput {
                prompt: input.prompt,
                negative_prompt,
                clear_negative_prompt,
                metadata_json: None,
                clear_metadata_json: false,
                is_favorite: input.is_favorite,
            },
        )
        .await
    }

    pub async fn delete_video_record_validated(&self, video_id: i64) -> Result<(), AppError> {
        let Some(existing) = self.generated_video_by_id(video_id).await? else {
            self.delete_missing_generated_video_references(video_id)
                .await?;
            return Ok(());
        };
        if self.video_record_is_pushed_source(&existing).await? {
            return Err(AppError::InvalidArgument(
                "该视频片段已推送到成片后处理，不能删除".to_string(),
            ));
        }
        if let Some(run) = self
            .latest_workflow_run_by_business_key(&format!("video_provider_job:{video_id}"))
            .await?
        {
            self.cancel_workflow_run_and_interrupt_videos(run.id, "关联视频已删除")
                .await?;
        }
        self.delete_video_tail_frame_assets(video_id).await?;
        self.delete_generated_video_record_with_cleanup(video_id)
            .await
    }

    pub async fn update_assembly_push_status_validated(
        &self,
        video_id: i64,
        input: UpdateAssemblyPushStatusInput,
    ) -> Result<generated_video::Model, AppError> {
        let existing = self.required_video_record(video_id).await?;
        if existing.video_type != a4n_domain::video_workflow::VIDEO_TYPE_ASSEMBLY_PUSH {
            return Err(AppError::InvalidArgument(format!(
                "视频 #{} 不是成片后处理推送批次",
                existing.id
            )));
        }
        let chapter_id = existing.chapter_id.ok_or_else(|| {
            AppError::InvalidArgument(format!("推送批次 #{} 缺少章节绑定", existing.id))
        })?;
        let workflow_mode = required_workflow_mode(&existing, "推送批次")?;
        let rows = self
            .generated_videos_for_chapter(existing.project_id, chapter_id)
            .await?;
        let pushes =
            assembly_push_records(&rows, workflow_mode).map_err(AppError::InvalidArgument)?;
        let latest_index =
            latest_non_rejected_push_index(pushes.iter().map(|push| push.status.as_str()))
                .ok_or_else(|| {
                    AppError::InvalidArgument("当前章节没有可管理的推送批次".to_string())
                })?;
        if pushes[latest_index].video.id != existing.id {
            return Err(AppError::InvalidArgument(
                "只能管理当前最新推送批次，不能跨推送操作".to_string(),
            ));
        }
        let next_status =
            next_assembly_push_status(input.action, pushes[latest_index].status.as_str())?;
        self.update_generated_video_record(
            existing.id,
            a4n_db::video::UpdateGeneratedVideoRecordInput {
                prompt: None,
                negative_prompt: None,
                clear_negative_prompt: false,
                metadata_json: Some(assembly_push_metadata_with_status(
                    existing.metadata_json,
                    next_status,
                )),
                clear_metadata_json: false,
                is_favorite: None,
            },
        )
        .await
    }

    pub async fn assembly_video_records_validated(
        &self,
        project_id: i64,
        chapter_id: i64,
        workflow_mode: &str,
        operation: &str,
    ) -> Result<AssemblyVideoRecords, AppError> {
        self.project_chapter_video_source(project_id, chapter_id, operation, false)
            .await?;
        let workflow_mode =
            validate_assembly_workflow_mode(workflow_mode).map_err(AppError::InvalidArgument)?;
        let rows = self
            .generated_videos_for_chapter(project_id, chapter_id)
            .await?;
        let (adopted_video_ids, timeline_order) = self
            .assembly_selection_for_workflow(project_id, chapter_id, workflow_mode, &rows)
            .await?;
        Ok(AssemblyVideoRecords {
            workflow_mode: workflow_mode.to_string(),
            clips: assembly_clip_records_from_rows(
                &rows,
                project_id,
                chapter_id,
                workflow_mode,
                None,
                adopted_video_ids.as_ref(),
                timeline_order.as_ref(),
            )?,
        })
    }

    async fn current_mixed_adopted_video_ids(
        &self,
        project_id: i64,
        chapter_id: i64,
        videos: &[generated_video::Model],
        snapshot: &a4n_db::video::VideoTimelineSnapshot,
    ) -> Result<HashSet<i64>, AppError> {
        let (plan_result, prompts_result) = futures::join!(
            self.current_shared_segment_plan(project_id, chapter_id),
            self.chapter_video_prompt_video_binding_records_for_chapter(project_id, chapter_id),
        );
        let plan = plan_result?;
        let segments = serde_json::from_value::<Vec<a4n_video_contract::VideoSegmentPlanV2>>(
            plan.metadata_json
                .as_ref()
                .and_then(|metadata| metadata.get("segments"))
                .cloned()
                .ok_or_else(|| {
                    AppError::InvalidArgument("当前片段规划缺少 segments".to_string())
                })?,
        )
        .map_err(|error| AppError::InvalidArgument(format!("当前片段规划无效：{error}")))?;
        let segments_by_number = segments.iter().fold(
            HashMap::with_capacity(segments.len()),
            |mut by_number, segment| {
                by_number.entry(segment.segment_number).or_insert(segment);
                by_number
            },
        );
        let videos_by_id =
            videos
                .iter()
                .fold(HashMap::with_capacity(videos.len()), |mut by_id, video| {
                    by_id.entry(video.id).or_insert(video);
                    by_id
                });
        let plan_style = plan
            .style
            .as_deref()
            .map(str::trim)
            .filter(|style| !style.is_empty())
            .ok_or_else(|| AppError::InvalidArgument("当前片段规划缺少 style".to_string()))?;
        let recipes = a4n_video_contract::video_segment_recipes_for_plan_from_metadata(
            plan.metadata_json.as_ref(),
            plan_style,
        )
        .map_err(AppError::InvalidArgument)?;
        let plan_identity = plan
            .metadata_json
            .as_ref()
            .and_then(|metadata| metadata.get("video_execution_identity"))
            .ok_or_else(|| AppError::InvalidArgument("当前片段规划缺少执行身份".to_string()))?;
        let mut adopted = HashSet::new();
        for recipe in &recipes {
            let Some(video_id) = recipe.adopted_video_id else {
                continue;
            };
            let segment_number = recipe.segment_number;
            let segment = segments_by_number
                .get(&segment_number)
                .copied()
                .ok_or_else(|| {
                    AppError::InvalidArgument(format!(
                        "片段 {segment_number} 的执行方案不属于当前规划"
                    ))
                })?;
            let expected_identity = super::prompt_generation::video_segment_execution_identity(
                plan_identity.clone(),
                Some(plan.id),
                Some(recipe),
                (&segment.source_shot_id, segment.segment_index_in_shot),
            )?;
            let Some(video) = videos_by_id.get(&video_id).copied() else {
                continue;
            };
            if video.project_id == project_id
                && video.chapter_id == Some(chapter_id)
                && video.status == a4n_domain::video_workflow::VIDEO_STATUS_COMPLETED
                && video
                    .file_path
                    .as_deref()
                    .map(str::trim)
                    .is_some_and(|path| !path.is_empty())
                && video.workflow_mode.as_deref() == Some(recipe.workflow_mode.as_str())
                && super::prompt_records::video_execution_segment_lineage_matches(
                    video.metadata_json.as_ref(),
                    &expected_identity,
                )
                && !adopted.insert(video_id)
            {
                return Err(AppError::InvalidArgument(format!(
                    "视频 #{video_id} 被多个片段重复采用"
                )));
            }
        }
        let prompts = prompts_result?;
        let prompt_count = prompts.len();
        let prompts_by_id = prompts.into_iter().fold(
            HashMap::with_capacity(prompt_count),
            |mut by_id, (prompt_id, video_id)| {
                by_id.entry(prompt_id).or_insert(video_id);
                by_id
            },
        );
        for item in snapshot.items.iter().filter(|item| {
            item.item_kind == a4n_db::video::TIMELINE_INSERTED_KIND
                && item.status == a4n_db::video::TIMELINE_ACTIVE_STATUS
        }) {
            let current_video_id = item
                .insert_plan_prompt_id
                .and_then(|prompt_id| prompts_by_id.get(&prompt_id).copied().flatten());
            let video_candidates = current_video_id
                .and_then(|video_id| videos_by_id.get(&video_id).copied())
                .map_or(&[][..], std::slice::from_ref);
            let Some(video_id) = super::timeline::current_timeline_item_video_id(
                item,
                current_video_id,
                video_candidates,
            )?
            else {
                continue;
            };
            if !adopted.insert(video_id) {
                return Err(AppError::InvalidArgument(format!(
                    "视频 #{video_id} 被多个时间线片段重复采用"
                )));
            }
        }
        Ok(adopted)
    }

    async fn assembly_selection_for_workflow(
        &self,
        project_id: i64,
        chapter_id: i64,
        workflow_mode: &str,
        videos: &[generated_video::Model],
    ) -> Result<
        (
            Option<HashSet<i64>>,
            Option<super::timeline::VideoTimelineAssemblyOrder>,
        ),
        AppError,
    > {
        if workflow_mode != WORKFLOW_MODE_MIXED {
            return Ok((None, None));
        }
        let snapshot = self
            .current_video_timeline_snapshot(project_id, chapter_id)
            .await?;
        let order = super::timeline::video_timeline_assembly_order_from_snapshot(&snapshot)?;
        let adopted = self
            .current_mixed_adopted_video_ids(project_id, chapter_id, videos, &snapshot)
            .await?;
        Ok((Some(adopted), Some(order)))
    }

    async fn prepare_assembly_insert_metadata_validated(
        &self,
        project_id: i64,
        chapter_id: i64,
        workflow_mode: &str,
        base_metadata: Option<Value>,
        insert_after_video_id: Option<i64>,
    ) -> Result<Value, AppError> {
        self.project_chapter_video_source(project_id, chapter_id, "导入衔接片段", true)
            .await?;
        let workflow_mode =
            validate_assembly_workflow_mode(workflow_mode).map_err(AppError::InvalidArgument)?;
        let rows = self
            .generated_videos_for_chapter(project_id, chapter_id)
            .await?;
        let (adopted_video_ids, timeline_order) = self
            .assembly_selection_for_workflow(project_id, chapter_id, workflow_mode, &rows)
            .await?;
        assembly_insert_position_metadata_from_rows(
            &rows,
            project_id,
            chapter_id,
            workflow_mode,
            None,
            adopted_video_ids.as_ref(),
            timeline_order.as_ref(),
            base_metadata,
            insert_after_video_id,
        )
    }

    pub async fn create_imported_assembly_insert_video_record_validated(
        &self,
        project_id: i64,
        chapter_id: i64,
        workflow_mode: &str,
        insert_after_video_id: Option<i64>,
        properties: ImportedVideoProperties,
        source_file_name: &str,
        source_extension: &str,
        imported_at: &str,
    ) -> Result<generated_video::Model, AppError> {
        let workflow_mode =
            validate_assembly_workflow_mode(workflow_mode).map_err(AppError::InvalidArgument)?;
        let metadata = self
            .prepare_assembly_insert_metadata_validated(
                project_id,
                chapter_id,
                workflow_mode,
                Some(serde_json::json!({
                    "assembly_insert": {
                        "source_file_name": source_file_name,
                        "source_extension": source_extension,
                        "imported_at": imported_at,
                    }
                })),
                insert_after_video_id,
            )
            .await?;
        let anchor_label = insert_after_video_id
            .map(|id| format!("视频 #{id} 之后"))
            .unwrap_or_else(|| "成片开头".to_string());
        self.create_video_record(a4n_db::video::CreateGeneratedVideoRecordDraftInput {
            project_id,
            chapter_id: Some(chapter_id),
            video_type: a4n_domain::video_workflow::VIDEO_TYPE_ASSEMBLY_INSERT.to_string(),
            workflow_mode: Some(workflow_mode.to_string()),
            prompt: format!("成片衔接片段，插入位置：{anchor_label}。"),
            negative_prompt: None,
            model: EXTERNAL_IMPORT_MODEL.to_string(),
            width: properties.width,
            height: properties.height,
            duration_secs: properties.duration_secs,
            fps: properties.fps,
            source_video_id: None,
            source_image_id: None,
            provider_job_id: None,
            metadata_json: Some(metadata),
        })
        .await
    }

    pub async fn prepare_assembly_postprocess_edit_validated(
        &self,
        push_video_id: i64,
        operation: AssemblyPostprocessOperation,
    ) -> Result<AssemblyPostprocessPlan, AppError> {
        let push = self.required_video_record(push_video_id).await?;
        if push.video_type != a4n_domain::video_workflow::VIDEO_TYPE_ASSEMBLY_PUSH {
            return Err(AppError::InvalidArgument(format!(
                "视频 #{} 不是后处理推送批次",
                push.id
            )));
        }
        if parse_assembly_push_metadata(push.id, push.metadata_json.as_ref())
            .map_err(AppError::InvalidArgument)?
            .status
            == ASSEMBLY_PUSH_STATUS_REJECTED
        {
            return Err(AppError::InvalidArgument(
                "已拒绝的推送批次不能继续编辑".to_string(),
            ));
        }
        let chapter_id = push.chapter_id.ok_or_else(|| {
            AppError::InvalidArgument(format!("推送批次 #{} 缺少章节绑定", push.id))
        })?;
        let workflow_mode = required_workflow_mode(&push, "推送批次")?;
        let rows = self
            .generated_videos_for_chapter(push.project_id, chapter_id)
            .await?;
        let pushes =
            assembly_push_records(&rows, workflow_mode).map_err(AppError::InvalidArgument)?;
        let latest_index = latest_non_rejected_push_index(
            pushes.iter().map(|candidate| candidate.status.as_str()),
        )
        .ok_or_else(|| AppError::InvalidArgument("当前章节没有可编辑的推送批次".to_string()))?;
        if pushes[latest_index].video.id != push.id {
            return Err(AppError::InvalidArgument(
                "只能编辑当前最新推送批次，不能跨推送操作".to_string(),
            ));
        }
        let base = self
            .latest_assembly_postprocess_version_for_push(&push)
            .await?;
        Ok(AssemblyPostprocessPlan {
            push: generated_video_summary(&push),
            base: generated_video_summary(&base),
            operation,
        })
    }

    pub async fn create_assembly_postprocess_edit_record_validated(
        &self,
        plan: AssemblyPostprocessPlan,
        input: AssemblyPostprocessEditRecordInput,
    ) -> Result<PreparedAssemblyPostprocessEditRecord, AppError> {
        if plan.base.width <= 0 || plan.base.height <= 0 {
            return Err(AppError::InvalidArgument(format!(
                "后处理基底视频尺寸无效：{}x{}",
                plan.base.width, plan.base.height
            )));
        }
        if plan.base.fps <= 0 {
            return Err(AppError::InvalidArgument(format!(
                "后处理基底视频帧率无效：{}",
                plan.base.fps
            )));
        }
        let (start_secs, end_secs, output_duration) = postprocess_edit_timing(
            plan.base.duration_secs,
            plan.operation,
            input.at_secs,
            input.start_secs,
            input.end_secs,
            input.source_duration_secs,
        )
        .map_err(AppError::InvalidArgument)?;
        let video = self
            .create_video_record(a4n_db::video::CreateGeneratedVideoRecordDraftInput {
                project_id: plan.push.project_id,
                chapter_id: plan.push.chapter_id,
                video_type: a4n_domain::video_workflow::VIDEO_TYPE_ASSEMBLY_POSTPROCESS_EDIT
                    .to_string(),
                workflow_mode: plan.push.workflow_mode.clone(),
                prompt: format!(
                    "后处理编辑：推送批次 #{} {}",
                    plan.push.id,
                    plan.operation.as_str()
                ),
                negative_prompt: None,
                model: "a4n-postprocess-edit".to_string(),
                width: plan.base.width,
                height: plan.base.height,
                duration_secs: output_duration,
                fps: plan.base.fps,
                source_video_id: Some(plan.base.id),
                source_image_id: None,
                provider_job_id: None,
                metadata_json: Some(serde_json::json!({
                    "postprocess_edit": {
                        "push_video_id": plan.push.id,
                        "base_video_id": plan.base.id,
                        "operation": plan.operation,
                        "at_secs": input.at_secs,
                        "start_secs": start_secs,
                        "end_secs": end_secs,
                        "source_video_id": input.source_video_id,
                        "source_file_name": input.source_file_name,
                        "edited_at": Utc::now().to_rfc3339(),
                    }
                })),
            })
            .await?;
        Ok(PreparedAssemblyPostprocessEditRecord {
            video: generated_video_summary(&video),
            base: plan.base,
            operation: plan.operation,
            start_secs,
            end_secs,
        })
    }

    pub async fn prepare_assembly_postprocess_source_validated(
        &self,
        plan: &AssemblyPostprocessPlan,
        source_video_id: Option<i64>,
        has_uploaded_source: bool,
    ) -> Result<AssemblyPostprocessSourcePlan, AppError> {
        if plan.operation == AssemblyPostprocessOperation::Delete {
            if source_video_id.is_some() || has_uploaded_source {
                return Err(AppError::InvalidArgument(
                    "删除后处理片段不接受来源视频".to_string(),
                ));
            }
            return Ok(AssemblyPostprocessSourcePlan::None);
        }
        if source_video_id.is_some() && has_uploaded_source {
            return Err(AppError::InvalidArgument(
                "插入或替换后处理片段只能选择一个来源视频".to_string(),
            ));
        }
        if let Some(video_id) = source_video_id {
            let video = self.required_video_record(video_id).await?;
            if video.project_id != plan.push.project_id {
                return Err(AppError::InvalidArgument(format!(
                    "替换/插入来源视频 #{} 不属于当前项目",
                    video.id
                )));
            }
            if video.status != a4n_domain::video_workflow::VIDEO_STATUS_COMPLETED {
                return Err(AppError::InvalidArgument(format!(
                    "替换/插入来源视频 #{} 尚未完成",
                    video.id
                )));
            }
            if video
                .file_path
                .as_deref()
                .map(str::trim)
                .is_none_or(|path| path.is_empty())
            {
                return Err(AppError::InvalidArgument(format!(
                    "替换/插入来源视频 #{} 没有本地文件",
                    video.id
                )));
            }
            return Ok(AssemblyPostprocessSourcePlan::Existing(
                generated_video_summary(&video),
            ));
        }
        if has_uploaded_source {
            return Ok(AssemblyPostprocessSourcePlan::Upload);
        }
        Err(AppError::InvalidArgument(
            "插入或替换后处理片段时必须选择上传视频或 API 生成视频".to_string(),
        ))
    }

    async fn latest_assembly_postprocess_version_for_push(
        &self,
        push: &generated_video::Model,
    ) -> Result<generated_video::Model, AppError> {
        let chapter_id = push.chapter_id.ok_or_else(|| {
            AppError::InvalidArgument(format!("推送批次 #{} 缺少章节绑定", push.id))
        })?;
        let mut versions = Vec::new();
        for row in self
            .generated_videos_for_chapter(push.project_id, chapter_id)
            .await?
        {
            if row.video_type != a4n_domain::video_workflow::VIDEO_TYPE_ASSEMBLY_POSTPROCESS_EDIT
                || row.status != a4n_domain::video_workflow::VIDEO_STATUS_COMPLETED
            {
                continue;
            }
            let metadata = parse_postprocess_edit_metadata(row.id, row.metadata_json.as_ref())
                .map_err(AppError::InvalidArgument)?;
            if metadata.push_video_id != push.id {
                continue;
            }
            if row.source_video_id != Some(metadata.base_video_id) {
                return Err(AppError::InvalidArgument(format!(
                    "后处理编辑视频 #{} 的 source_video_id 与 postprocess_edit.base_video_id 不一致",
                    row.id
                )));
            }
            if row.workflow_mode.as_deref() != push.workflow_mode.as_deref() {
                return Err(AppError::InvalidArgument(format!(
                    "后处理编辑视频 #{} 的 workflow_mode 与推送批次 #{} 不一致",
                    row.id, push.id
                )));
            }
            versions.push(row);
        }
        versions.sort_by(|left, right| {
            left.created_at
                .cmp(&right.created_at)
                .then_with(|| left.id.cmp(&right.id))
        });
        Ok(versions.pop().unwrap_or_else(|| push.clone()))
    }

    pub async fn create_assembly_push_record_validated(
        &self,
        project_id: i64,
        chapter_id: i64,
        workflow_mode: &str,
        clips: Vec<AssemblyPushClipInput>,
        end_video_id: Option<i64>,
    ) -> Result<PreparedAssemblyPushRecord, AppError> {
        let workflow_mode =
            validate_assembly_workflow_mode(workflow_mode).map_err(AppError::InvalidArgument)?;
        if clips.is_empty() {
            return Err(AppError::InvalidArgument(
                "当前视频生成流程还没有可推送的成片片段".to_string(),
            ));
        }
        let rows = self
            .generated_videos_for_chapter(project_id, chapter_id)
            .await?;
        let pushes =
            assembly_push_records(&rows, workflow_mode).map_err(AppError::InvalidArgument)?;
        if pushes
            .iter()
            .any(|push| push.status == ASSEMBLY_PUSH_STATUS_PENDING)
        {
            return Err(AppError::InvalidArgument(
                "当前最新推送尚未同意或拒绝，不能继续推送后续片段".to_string(),
            ));
        }
        let active_source_ids = active_assembly_push_source_ids(
            pushes
                .iter()
                .map(|push| (push.status.as_str(), push.source_video_ids.as_slice())),
        );
        let clip_ids = clips.iter().map(|clip| clip.video_id).collect::<Vec<_>>();
        super::ensure_active_pushes_match_clip_prefix(&clip_ids, &active_source_ids)
            .map_err(AppError::InvalidArgument)?;
        let pending = &clips[active_source_ids.len()..];
        let pending_ids = pending.iter().map(|clip| clip.video_id).collect::<Vec<_>>();
        let end_index = selected_assembly_push_end_index(&pending_ids, end_video_id)
            .map_err(AppError::InvalidArgument)?;
        let source_clips = &pending[..=end_index];
        let push_index = pushes
            .iter()
            .map(|push| push.push_index)
            .max()
            .unwrap_or(0)
            .checked_add(1)
            .ok_or_else(|| {
                AppError::InvalidArgument("成片推送批次序号超过 i32 上限".to_string())
            })?;
        let (width, height, fps, duration_secs) =
            assembly_media_properties(source_clips.iter().map(|clip| {
                (
                    clip.video_id,
                    clip.duration_secs,
                    clip.width,
                    clip.height,
                    clip.fps,
                )
            }))
            .map_err(AppError::InvalidArgument)?;
        let source_video_ids = source_clips
            .iter()
            .map(|clip| clip.video_id)
            .collect::<Vec<_>>();
        let source_sort_orders = source_clips
            .iter()
            .map(|clip| clip.sort_order)
            .collect::<Vec<_>>();
        let video = self
            .create_video_record(a4n_db::video::CreateGeneratedVideoRecordDraftInput {
                project_id,
                chapter_id: Some(chapter_id),
                video_type: a4n_domain::video_workflow::VIDEO_TYPE_ASSEMBLY_PUSH.to_string(),
                workflow_mode: Some(workflow_mode.to_string()),
                prompt: format!("成片后处理推送批次 {push_index}"),
                negative_prompt: None,
                model: "a4n-assembly-push".to_string(),
                width,
                height,
                duration_secs,
                fps,
                source_video_id: source_video_ids.first().copied(),
                source_image_id: None,
                provider_job_id: None,
                metadata_json: Some(serde_json::json!({
                    "assembly_push": {
                        "status": ASSEMBLY_PUSH_STATUS_PENDING,
                        "push_index": push_index,
                        "workflow_mode": workflow_mode,
                        "source_video_ids": source_video_ids,
                        "source_sort_orders": source_sort_orders,
                        "source_clip_count": source_clips.len(),
                        "pushed_at": Utc::now().to_rfc3339(),
                    }
                })),
            })
            .await?;
        Ok(PreparedAssemblyPushRecord {
            video: generated_video_summary(&video),
            source_video_ids,
            push_index,
        })
    }

    pub async fn create_imported_chapter_video_record_validated(
        &self,
        project_id: i64,
        chapter_id: i64,
        prompt_id: i64,
        properties: ImportedVideoProperties,
        source_file_name: &str,
        source_extension: &str,
        imported_at: &str,
    ) -> Result<PreparedImportedChapterVideoRecord, AppError> {
        let (source_result, prompt_result) = futures::join!(
            self.project_chapter_video_source(project_id, chapter_id, "导入视频", true),
            self.chapter_video_prompt_for_import(project_id, chapter_id, prompt_id),
        );
        let source = source_result?;
        let prompt = prompt_result?;
        if let Some(source_hash) = metadata_source_content_sha256(prompt.metadata_json.as_ref())
            && source_hash != source_text_hash(&source.source.content)
        {
            return Err(AppError::InvalidArgument(
                "当前提示词基于旧正文内容，请重新生成视频提示词".to_string(),
            ));
        }
        let video = self
            .create_video_record(a4n_db::video::CreateGeneratedVideoRecordDraftInput {
                project_id,
                chapter_id: Some(chapter_id),
                video_type: a4n_domain::video_workflow::VIDEO_TYPE_CHAPTER.to_string(),
                workflow_mode: Some(prompt.workflow_mode.clone()),
                prompt: imported_video_prompt_text(&prompt),
                negative_prompt: prompt.negative_prompt.clone(),
                model: EXTERNAL_IMPORT_MODEL.to_string(),
                width: properties.width,
                height: properties.height,
                duration_secs: properties.duration_secs,
                fps: properties.fps,
                source_video_id: None,
                source_image_id: None,
                provider_job_id: None,
                metadata_json: Some(imported_chapter_video_metadata(
                    prompt.id,
                    prompt.shot_number,
                    &prompt.prompt_type,
                    source_file_name,
                    source_extension,
                    imported_at,
                    prompt.metadata_json.as_ref(),
                )),
            })
            .await?;
        Ok(PreparedImportedChapterVideoRecord { video, prompt })
    }

    pub async fn create_imported_reference_media_video_record_validated(
        &self,
        project_id: i64,
        chapter_id: i64,
        properties: ImportedVideoProperties,
        source_file_name: &str,
        source_extension: &str,
        imported_at: &str,
        segment_plan_prompt_id: i64,
        target: (&str, i32),
    ) -> Result<generated_video::Model, AppError> {
        self.project_chapter_video_source(project_id, chapter_id, "导入参考媒体视频", true)
            .await?;
        let plan = self
            .require_current_shared_segment_plan(project_id, chapter_id, segment_plan_prompt_id)
            .await?;
        let shot_number = a4n_video_contract::resolve_video_segment_number(
            plan.metadata_json.as_ref(),
            target.0,
            target.1,
        )
        .map_err(AppError::InvalidArgument)?;
        let recipe = a4n_video_contract::video_segment_recipe_from_metadata(
            plan.metadata_json.as_ref(),
            shot_number,
        )
        .map_err(AppError::InvalidArgument)?
        .ok_or_else(|| {
            AppError::InvalidArgument(format!("当前规划缺少片段 {shot_number} 的执行方案"))
        })?;
        if recipe.workflow_mode != VideoSegmentWorkflowModeV1::ReferenceMedia {
            return Err(AppError::InvalidArgument(format!(
                "片段 {shot_number} 当前未选择参考媒体方案"
            )));
        }
        let reference_media = recipe.reference_media.as_ref().ok_or_else(|| {
            AppError::InvalidArgument(format!("片段 {shot_number} 尚未保存参考媒体提示词和媒体槽"))
        })?;
        let prompt =
            super::reference_media_rules::reference_media_prompt_from_recipe(reference_media);
        if prompt.trim().is_empty() {
            return Err(AppError::InvalidArgument(
                "当前片段参考媒体提示词不能为空".to_string(),
            ));
        }
        validate_reference_media_import_options(recipe.duration_secs, &recipe.aspect_ratio)
            .map_err(AppError::InvalidArgument)?;
        let plan_identity = plan
            .metadata_json
            .as_ref()
            .and_then(|metadata| metadata.get("video_execution_identity"))
            .cloned()
            .ok_or_else(|| AppError::InvalidArgument("当前片段规划缺少执行身份".to_string()))?;
        let video_execution_identity = super::prompt_generation::video_segment_execution_identity(
            plan_identity,
            Some(plan.id),
            Some(&recipe),
            target,
        )?;
        self.create_video_record(a4n_db::video::CreateGeneratedVideoRecordDraftInput {
            project_id,
            chapter_id: Some(chapter_id),
            video_type: a4n_domain::video_workflow::VIDEO_TYPE_REFERENCE_MEDIA.to_string(),
            workflow_mode: Some(
                VideoSegmentWorkflowModeV1::ReferenceMedia
                    .as_str()
                    .to_string(),
            ),
            prompt,
            negative_prompt: None,
            model: EXTERNAL_IMPORT_MODEL.to_string(),
            width: properties.width,
            height: properties.height,
            duration_secs: properties.duration_secs,
            fps: properties.fps,
            source_video_id: None,
            source_image_id: None,
            provider_job_id: None,
            metadata_json: Some(serde_json::json!({
                "segment_plan_prompt_id": segment_plan_prompt_id,
                "shot_number": shot_number,
                "video_execution_identity": video_execution_identity,
                "provider_backend": "external-import",
                "reference_media": {
                    "functionMode": "omni_reference",
                    "aspect_ratio": recipe.aspect_ratio,
                },
                "import": {
                    "external": true,
                    "source_file_name": source_file_name,
                    "source_extension": source_extension,
                    "imported_at": imported_at,
                }
            })),
        })
        .await
    }

    pub async fn create_imported_canvas_video_record_validated(
        &self,
        project_id: i64,
        properties: ImportedVideoProperties,
        source_file_name: &str,
        source_extension: &str,
        imported_at: &str,
    ) -> Result<generated_video::Model, AppError> {
        self.create_video_record(a4n_db::video::CreateGeneratedVideoRecordDraftInput {
            project_id,
            chapter_id: None,
            video_type: a4n_domain::video_workflow::VIDEO_TYPE_CANVAS.to_string(),
            workflow_mode: Some(WORKFLOW_MODE_CANVAS_VIDEO.to_string()),
            prompt: source_file_name.to_string(),
            negative_prompt: None,
            model: EXTERNAL_IMPORT_MODEL.to_string(),
            width: properties.width,
            height: properties.height,
            duration_secs: properties.duration_secs,
            fps: properties.fps,
            source_video_id: None,
            source_image_id: None,
            provider_job_id: None,
            metadata_json: Some(serde_json::json!({
                "canvas_import": {
                    "external": true,
                    "source_file_name": source_file_name,
                    "source_extension": source_extension,
                    "imported_at": imported_at,
                }
            })),
        })
        .await
    }

    pub async fn prepare_canvas_assembly_validated(
        &self,
        project_id: i64,
        video_ids: Vec<i64>,
        title: Option<&str>,
    ) -> Result<CanvasAssemblyPlan, AppError> {
        if video_ids.is_empty() {
            return Err(AppError::InvalidArgument(
                "AssembleNode 至少需要一个视频片段".to_string(),
            ));
        }
        let mut seen = HashSet::new();
        for &video_id in &video_ids {
            if !seen.insert(video_id) {
                return Err(AppError::InvalidArgument(format!(
                    "AssembleNode 视频片段重复: {video_id}"
                )));
            }
        }
        let videos_by_id = self
            .generated_videos_by_ids(&video_ids)
            .await?
            .into_iter()
            .map(|video| (video.id, video))
            .collect::<std::collections::HashMap<_, _>>();
        let mut sources = Vec::with_capacity(video_ids.len());
        for video_id in video_ids {
            let video = videos_by_id
                .get(&video_id)
                .cloned()
                .ok_or_else(|| AppError::NotFound("视频不存在".to_string()))?;
            if video.project_id != project_id {
                return Err(AppError::InvalidArgument(format!(
                    "视频 #{video_id} 不属于项目 {project_id}"
                )));
            }
            if video.chapter_id.is_some() {
                return Err(AppError::InvalidArgument(format!(
                    "AssembleNode 只支持项目级画布视频: {video_id}"
                )));
            }
            if video.status != a4n_domain::video_workflow::VIDEO_STATUS_COMPLETED {
                return Err(AppError::InvalidArgument(format!(
                    "视频 #{video_id} 尚未完成，不能成片"
                )));
            }
            if video
                .file_path
                .as_deref()
                .map(str::trim)
                .is_none_or(|path| path.is_empty())
            {
                return Err(AppError::InvalidArgument(format!(
                    "视频 #{video_id} 缺少本地文件"
                )));
            }
            sources.push(video);
        }
        let (width, height, fps, duration_secs) =
            assembly_media_properties(sources.iter().map(|video| {
                (
                    video.id,
                    video.duration_secs,
                    video.width,
                    video.height,
                    video.fps,
                )
            }))
            .map_err(AppError::InvalidArgument)?;
        Ok(CanvasAssemblyPlan {
            width,
            height,
            fps,
            duration_secs,
            sources: sources.iter().map(generated_video_summary).collect(),
            title: title
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .unwrap_or("画布成片")
                .to_string(),
        })
    }

    pub async fn prepare_canvas_frame_validated(
        &self,
        project_id: i64,
        video_id: i64,
        frame_kind: CanvasFrameKind,
        at_secs: Option<f32>,
    ) -> Result<CanvasFramePlan, AppError> {
        let video = self.required_video_record(video_id).await?;
        if video.project_id != project_id {
            return Err(AppError::InvalidArgument(format!(
                "视频 #{video_id} 不属于项目 {project_id}"
            )));
        }
        if video.chapter_id.is_some() {
            return Err(AppError::InvalidArgument(
                "ExtractFrameNode 只支持项目级画布视频".to_string(),
            ));
        }
        if video
            .file_path
            .as_deref()
            .map(str::trim)
            .is_none_or(|path| path.is_empty())
        {
            return Err(AppError::InvalidArgument(
                "源视频还没有可抽帧的本地文件".to_string(),
            ));
        }
        if video.width <= 0 || video.height <= 0 {
            return Err(AppError::InvalidArgument(format!(
                "源视频尺寸无效：{}x{}",
                video.width, video.height
            )));
        }
        let timestamp = canvas_frame_timestamp(video.duration_secs, frame_kind, at_secs)
            .map_err(AppError::InvalidArgument)?;
        Ok(CanvasFramePlan {
            video: generated_video_summary(&video),
            frame_kind,
            timestamp,
        })
    }

    pub async fn create_canvas_frame_record_validated(
        &self,
        plan: CanvasFramePlan,
        file_path: String,
        file_size: i64,
    ) -> Result<GeneratedImageSummary, AppError> {
        let image = self
            .create_generated_image_record(a4n_db::video::CreateGeneratedImageRecordDraftInput {
                project_id: plan.video.project_id,
                image_type: IMAGE_TYPE_CANVAS_FRAME.to_string(),
                file_path,
                prompt: format!("视频 #{} 画布抽帧 {}", plan.video.id, plan.frame_kind),
                model: "ffmpeg".to_string(),
                width: plan.video.width,
                height: plan.video.height,
                file_size: Some(file_size),
                chapter_id: None,
                panel_id: None,
                metadata_json: Some(serde_json::json!({
                    "canvas_frame": {
                        "source_video_id": plan.video.id,
                        "frame_kind": plan.frame_kind,
                        "at_secs": plan.timestamp,
                        "source_workflow_mode": plan.video.workflow_mode,
                        "extracted_at": Utc::now().to_rfc3339(),
                    },
                    "source": "video_canvas",
                })),
                created_at: None,
            })
            .await?;
        Ok(crate::media::generated_image_summary(&image))
    }

    /// 将 Runtime 已提取的真实尾帧登记为视频素材。
    /// 文件提取仍属于 Host；记录字段和归属元数据只在 Video application 生成。
    pub async fn create_video_tail_frame_record_validated(
        &self,
        video: &generated_video::Model,
        chapter_id: i64,
        file_path: String,
        file_size: i64,
    ) -> Result<generated_image::Model, AppError> {
        if video.chapter_id != Some(chapter_id) {
            return Err(AppError::InvalidArgument(format!(
                "视频 #{} 不属于章节 {}，无法登记真实尾帧",
                video.id, chapter_id
            )));
        }
        if file_path.trim().is_empty() {
            return Err(AppError::InvalidArgument(
                "真实尾帧文件路径不能为空".to_string(),
            ));
        }
        if file_size <= 0 {
            return Err(AppError::InvalidArgument(
                "真实尾帧文件大小必须大于 0".to_string(),
            ));
        }
        let shot_number = video_shot_number(video).map_err(AppError::InvalidArgument)?;
        let source = video
            .metadata_json
            .as_ref()
            .and_then(|metadata| metadata.get("source"))
            .cloned();
        self.create_generated_image_record(a4n_db::video::CreateGeneratedImageRecordDraftInput {
            project_id: video.project_id,
            image_type: IMAGE_TYPE_VIDEO_TAIL_FRAME.to_string(),
            file_path,
            prompt: format!("视频 #{} 第 {} 段真实尾帧采集", video.id, shot_number),
            model: "ffmpeg".to_string(),
            width: video.width.max(1),
            height: video.height.max(1),
            file_size: Some(file_size),
            chapter_id: Some(chapter_id),
            panel_id: None,
            metadata_json: Some(serde_json::json!({
                "generation_context": {
                    "source": IMAGE_GENERATION_SOURCE_VIDEO_TAIL_FRAME_CAPTURE,
                    "video_id": video.id,
                    "shot_number": shot_number,
                    "source_content_sha256": metadata_source_content_sha256(
                        video.metadata_json.as_ref()
                    ),
                },
                "source": source,
            })),
            created_at: None,
        })
        .await
    }

    pub async fn create_canvas_assembly_record_validated(
        &self,
        project_id: i64,
        plan: CanvasAssemblyPlan,
    ) -> Result<GeneratedVideoSummary, AppError> {
        let source_video_ids = plan
            .sources
            .iter()
            .map(|video| video.id)
            .collect::<Vec<_>>();
        let record = self
            .create_video_record(a4n_db::video::CreateGeneratedVideoRecordDraftInput {
                project_id,
                chapter_id: None,
                video_type: a4n_domain::video_workflow::VIDEO_TYPE_CANVAS_ASSEMBLY.to_string(),
                workflow_mode: Some(WORKFLOW_MODE_CANVAS_ASSEMBLY.to_string()),
                prompt: plan.title,
                negative_prompt: None,
                model: "ffmpeg".to_string(),
                width: plan.width,
                height: plan.height,
                duration_secs: plan.duration_secs,
                fps: plan.fps,
                source_video_id: None,
                source_image_id: None,
                provider_job_id: None,
                metadata_json: Some(serde_json::json!({
                    "canvas_assembly": {
                        "source_video_ids": source_video_ids,
                        "clip_count": plan.sources.len(),
                        "assembled_at": Utc::now().to_rfc3339(),
                    },
                    "source": "video_canvas",
                })),
            })
            .await?;
        Ok(generated_video_summary(&record))
    }

    pub fn prepare_video_trim_validated(
        &self,
        source: generated_video::Model,
        start_secs: f32,
        end_secs: f32,
    ) -> Result<VideoTrimPlan, AppError> {
        validate_video_trim_range(start_secs, end_secs).map_err(AppError::InvalidArgument)?;
        let source_workflow_mode = source.workflow_mode.clone().ok_or_else(|| {
            AppError::InvalidArgument(format!(
                "源视频 #{} 缺少 workflow_mode，无法按生成模式保存裁剪结果",
                source.id
            ))
        })?;
        if !is_generated_video_workflow_mode(&source_workflow_mode) {
            return Err(AppError::InvalidArgument(format!(
                "源视频 #{} workflow_mode 无效：{}",
                source.id, source_workflow_mode
            )));
        }
        if !source.duration_secs.is_finite() || source.duration_secs <= 0.0 {
            return Err(AppError::InvalidArgument(format!(
                "源视频 #{} 时长无效：{}",
                source.id, source.duration_secs
            )));
        }
        if end_secs > source.duration_secs {
            return Err(AppError::InvalidArgument(
                "裁剪结束时间超过视频时长".to_string(),
            ));
        }
        if source.width <= 0 || source.height <= 0 {
            return Err(AppError::InvalidArgument(format!(
                "源视频尺寸无效：{}x{}",
                source.width, source.height
            )));
        }
        if source.fps <= 0 {
            return Err(AppError::InvalidArgument(format!(
                "源视频帧率无效：{}",
                source.fps
            )));
        }
        let target_workflow_mode = match source.chapter_id {
            None if is_canvas_video_record(&source) => WORKFLOW_MODE_CANVAS_TRIM.to_string(),
            None => {
                return Err(AppError::InvalidArgument(format!(
                    "项目级源视频 workflow_mode 无效：{source_workflow_mode}"
                )));
            }
            Some(_) if !is_assembly_video_workflow_mode(&source_workflow_mode) => {
                return Err(AppError::InvalidArgument(format!(
                    "章节源视频不能使用 Canvas workflow_mode：{source_workflow_mode}"
                )));
            }
            Some(_) => source_workflow_mode,
        };
        Ok(VideoTrimPlan {
            source: generated_video_summary(&source),
            start_secs,
            end_secs,
            target_workflow_mode,
        })
    }

    pub async fn create_trimmed_video_record_validated(
        &self,
        plan: VideoTrimPlan,
    ) -> Result<GeneratedVideoSummary, AppError> {
        let record = self
            .create_video_record(a4n_db::video::CreateGeneratedVideoRecordDraftInput {
                project_id: plan.source.project_id,
                chapter_id: plan.source.chapter_id,
                video_type: a4n_domain::video_workflow::VIDEO_TYPE_TRIMMED_CLIP.to_string(),
                workflow_mode: Some(plan.target_workflow_mode),
                prompt: plan.source.prompt.clone(),
                negative_prompt: plan.source.negative_prompt.clone(),
                model: plan.source.model.clone(),
                width: plan.source.width,
                height: plan.source.height,
                duration_secs: plan.end_secs - plan.start_secs,
                fps: plan.source.fps,
                source_video_id: Some(plan.source.id),
                source_image_id: plan.source.source_image_id,
                provider_job_id: None,
                metadata_json: Some(trimmed_video_metadata(
                    plan.source.metadata_json.as_ref(),
                    plan.start_secs,
                    plan.end_secs,
                )),
            })
            .await?;
        Ok(generated_video_summary(&record))
    }

    pub async fn required_video_record(
        &self,
        video_id: i64,
    ) -> Result<generated_video::Model, AppError> {
        self.generated_video_by_id(video_id)
            .await?
            .ok_or_else(|| AppError::NotFound("视频不存在".to_string()))
    }

    async fn video_record_is_pushed_source(
        &self,
        video: &generated_video::Model,
    ) -> Result<bool, AppError> {
        if !is_assembly_clip_video_type(&video.video_type) {
            return Ok(false);
        }
        let (Some(chapter_id), Some(workflow_mode)) = (
            video.chapter_id,
            video.workflow_mode.as_deref().map(str::trim),
        ) else {
            return Ok(false);
        };
        Ok(self
            .active_pushed_source_ids(video.project_id, chapter_id, workflow_mode)
            .await?
            .contains(&video.id))
    }

    async fn active_pushed_source_ids(
        &self,
        project_id: i64,
        chapter_id: i64,
        workflow_mode: &str,
    ) -> Result<HashSet<i64>, AppError> {
        let rows = self
            .generated_videos_for_chapter(project_id, chapter_id)
            .await?;
        let pushes =
            assembly_push_records(&rows, workflow_mode).map_err(AppError::InvalidArgument)?;
        Ok(active_assembly_push_source_ids(
            pushes
                .iter()
                .map(|push| (push.status.as_str(), push.source_video_ids.as_slice())),
        )
        .into_iter()
        .collect())
    }
}

#[async_trait]
impl VideoProviderExecutionApplicationPort for crate::OrchestratorContext {
    async fn poll_target(
        &self,
        user_id: i64,
        video_id: i64,
    ) -> Result<VideoProviderPollTarget, AppError> {
        let video = self.required_video_record(video_id).await?;
        self.verify_project_permission(
            video.project_id,
            user_id,
            crate::collaboration::ProjectAccessRole::Editor,
        )
        .await?;
        if !crate::video::has_active_provider_job(video.provider_job_id.as_deref(), &video.status) {
            return Ok(VideoProviderPollTarget::Inactive);
        }
        Ok(VideoProviderPollTarget::Active {
            config_id: crate::video::provider_poll_config_id(&video)
                .map_err(AppError::InvalidArgument)?,
        })
    }

    async fn poll_provider_job(
        &self,
        user_id: i64,
        video_id: i64,
        config: Option<&VideoProviderConfig>,
        host_port: &dyn VideoProviderHostPort,
    ) -> Result<GeneratedVideoSummary, AppError> {
        let video = self.required_video_record(video_id).await?;
        self.verify_project_permission(
            video.project_id,
            user_id,
            crate::collaboration::ProjectAccessRole::Editor,
        )
        .await?;
        if !crate::video::has_active_provider_job(video.provider_job_id.as_deref(), &video.status) {
            return Ok(generated_video_summary(&video));
        }
        let config = config.ok_or_else(|| AppError::InvalidState {
            current: "video_provider_poll_config_missing".to_string(),
            operation: "恢复视频 provider 轮询".to_string(),
        })?;
        let prompt_id = a4n_domain::video::generated_video_prompt_id(video.metadata_json.as_ref());
        let updated = self
            .poll_video_provider_once_validated(
                video,
                config,
                prompt_id,
                CancellationToken::new(),
                host_port,
            )
            .await?;
        Ok(generated_video_summary(&updated))
    }
}

#[async_trait]
impl VideoRecordApplicationPort for crate::OrchestratorContext {
    async fn create_imported_chapter_video_record(
        &self,
        project_id: i64,
        chapter_id: i64,
        prompt_id: i64,
        properties: ImportedVideoProperties,
        source_file_name: &str,
        source_extension: &str,
        imported_at: &str,
    ) -> Result<VideoImportRecordSummary, AppError> {
        let prepared = self
            .create_imported_chapter_video_record_validated(
                project_id,
                chapter_id,
                prompt_id,
                properties,
                source_file_name,
                source_extension,
                imported_at,
            )
            .await?;
        Ok(VideoImportRecordSummary::from_record(
            &prepared.video,
            Some(prepared.prompt.id),
        ))
    }

    async fn create_imported_reference_media_video_record(
        &self,
        project_id: i64,
        chapter_id: i64,
        properties: ImportedVideoProperties,
        source_file_name: &str,
        source_extension: &str,
        imported_at: &str,
        segment_plan_prompt_id: i64,
        target_source_shot_id: &str,
        target_segment_index: i32,
    ) -> Result<VideoImportRecordSummary, AppError> {
        let video = self
            .create_imported_reference_media_video_record_validated(
                project_id,
                chapter_id,
                properties,
                source_file_name,
                source_extension,
                imported_at,
                segment_plan_prompt_id,
                (target_source_shot_id, target_segment_index),
            )
            .await?;
        Ok(VideoImportRecordSummary::from_record(&video, None))
    }

    async fn create_imported_assembly_insert_video_record(
        &self,
        project_id: i64,
        chapter_id: i64,
        workflow_mode: &str,
        insert_after_video_id: Option<i64>,
        properties: ImportedVideoProperties,
        source_file_name: &str,
        source_extension: &str,
        imported_at: &str,
    ) -> Result<VideoImportRecordSummary, AppError> {
        let video = self
            .create_imported_assembly_insert_video_record_validated(
                project_id,
                chapter_id,
                workflow_mode,
                insert_after_video_id,
                properties,
                source_file_name,
                source_extension,
                imported_at,
            )
            .await?;
        Ok(VideoImportRecordSummary::from_record(&video, None))
    }

    async fn create_imported_canvas_video_record(
        &self,
        project_id: i64,
        properties: ImportedVideoProperties,
        source_file_name: &str,
        source_extension: &str,
        imported_at: &str,
    ) -> Result<VideoImportRecordSummary, AppError> {
        let video = self
            .create_imported_canvas_video_record_validated(
                project_id,
                properties,
                source_file_name,
                source_extension,
                imported_at,
            )
            .await?;
        Ok(VideoImportRecordSummary::from_record(&video, None))
    }

    async fn assembly_records(
        &self,
        project_id: i64,
        chapter_id: i64,
        workflow_mode: &str,
        operation: &str,
    ) -> Result<AssemblyVideoRecords, AppError> {
        self.assembly_video_records_validated(project_id, chapter_id, workflow_mode, operation)
            .await
    }

    async fn prepare_assembly_postprocess_edit(
        &self,
        push_video_id: i64,
        operation: AssemblyPostprocessOperation,
    ) -> Result<AssemblyPostprocessPlan, AppError> {
        self.prepare_assembly_postprocess_edit_validated(push_video_id, operation)
            .await
    }

    async fn prepare_assembly_postprocess_source(
        &self,
        plan: &AssemblyPostprocessPlan,
        source_video_id: Option<i64>,
        has_uploaded_source: bool,
    ) -> Result<AssemblyPostprocessSourcePlan, AppError> {
        self.prepare_assembly_postprocess_source_validated(
            plan,
            source_video_id,
            has_uploaded_source,
        )
        .await
    }

    async fn create_assembly_postprocess_edit_record(
        &self,
        plan: AssemblyPostprocessPlan,
        input: AssemblyPostprocessEditRecordInput,
    ) -> Result<PreparedAssemblyPostprocessEditRecord, AppError> {
        self.create_assembly_postprocess_edit_record_validated(plan, input)
            .await
    }

    async fn create_assembly_push_record(
        &self,
        project_id: i64,
        chapter_id: i64,
        workflow_mode: &str,
        clips: Vec<AssemblyPushClipInput>,
        end_video_id: Option<i64>,
    ) -> Result<PreparedAssemblyPushRecord, AppError> {
        self.create_assembly_push_record_validated(
            project_id,
            chapter_id,
            workflow_mode,
            clips,
            end_video_id,
        )
        .await
    }

    async fn mark_error_status_and_prompt(
        &self,
        video_id: i64,
        prompt_id: Option<i64>,
        status: &str,
        message: &str,
    ) -> Result<GeneratedVideoSummary, AppError> {
        Ok(generated_video_summary(
            &self
                .mark_generated_video_error_status_and_prompt(video_id, prompt_id, status, message)
                .await?,
        ))
    }

    async fn playback_source(
        &self,
        user_id: i64,
        video_id: i64,
    ) -> Result<GeneratedVideoSummary, AppError> {
        let video = self.required_video_record(video_id).await?;
        self.verify_project_permission(
            video.project_id,
            user_id,
            crate::collaboration::ProjectAccessRole::Editor,
        )
        .await?;
        Ok(generated_video_summary(&video))
    }

    async fn list_by_chapter(
        &self,
        user_id: i64,
        project_id: i64,
        chapter_id: i64,
    ) -> Result<Vec<GeneratedVideoSummary>, AppError> {
        self.verify_project_permission(
            project_id,
            user_id,
            crate::collaboration::ProjectAccessRole::Viewer,
        )
        .await?;
        Ok(self
            .list_video_records_by_chapter_validated(project_id, chapter_id)
            .await?
            .iter()
            .map(generated_video_summary)
            .collect())
    }

    async fn validated_reference_images(
        &self,
        user_id: i64,
        project_id: i64,
        chapter_id: i64,
        reference_image_ids_json: Option<&Value>,
    ) -> Result<Vec<GeneratedImageSummary>, AppError> {
        let images = self
            .validated_video_reference_images(project_id, chapter_id, reference_image_ids_json)
            .await?;
        let project_ids = images
            .iter()
            .map(|image| image.project_id)
            .collect::<HashSet<_>>();
        self.verify_project_permissions(
            &project_ids.into_iter().collect::<Vec<_>>(),
            user_id,
            crate::collaboration::ProjectAccessRole::Editor,
        )
        .await?;
        Ok(images
            .iter()
            .map(crate::media::generated_image_summary)
            .collect())
    }

    async fn create_video_tail_frame(
        &self,
        input: VideoTailFrameInput,
    ) -> Result<VideoTailFrameSummary, AppError> {
        let video = self.required_video_record(input.video_id).await?;
        let chapter_id = video.chapter_id.ok_or_else(|| {
            AppError::InvalidArgument(format!(
                "视频 #{} 缺少 chapter_id，无法登记真实尾帧",
                video.id
            ))
        })?;
        let shot_number = video_shot_number(&video).map_err(AppError::InvalidArgument)?;
        let image = self
            .create_video_tail_frame_record_validated(
                &video,
                chapter_id,
                input.file_path,
                input.file_size,
            )
            .await?;
        Ok(VideoTailFrameSummary {
            image_id: image.id,
            file_path: image.file_path,
            shot_number,
        })
    }

    async fn delete_video_tail_frame_assets(&self, video_id: i64) -> Result<(), AppError> {
        let images = self
            .video_tail_frame_images()
            .await?
            .into_iter()
            .filter(|(_, image_type, metadata_json)| {
                tail_frame_metadata_belongs_to_video(image_type, metadata_json.as_ref(), video_id)
            })
            .collect::<Vec<_>>();
        let mut failures = Vec::new();
        for (image_id, _, _) in images {
            if let Err(error) = self
                .delete_generated_image_record_with_cleanup(image_id)
                .await
            {
                failures.push(format!("尾帧图片 #{image_id} 删除失败：{error}"));
            }
        }
        if failures.is_empty() {
            Ok(())
        } else {
            Err(AppError::Internal(anyhow::anyhow!(failures.join("；"))))
        }
    }

    async fn prepare_canvas_assembly(
        &self,
        project_id: i64,
        video_ids: Vec<i64>,
        title: Option<&str>,
    ) -> Result<CanvasAssemblyPlan, AppError> {
        self.prepare_canvas_assembly_validated(project_id, video_ids, title)
            .await
    }

    async fn create_canvas_assembly_record(
        &self,
        project_id: i64,
        plan: CanvasAssemblyPlan,
    ) -> Result<GeneratedVideoSummary, AppError> {
        self.create_canvas_assembly_record_validated(project_id, plan)
            .await
    }

    async fn prepare_canvas_frame(
        &self,
        project_id: i64,
        video_id: i64,
        frame_kind: CanvasFrameKind,
        at_secs: Option<f32>,
    ) -> Result<CanvasFramePlan, AppError> {
        self.prepare_canvas_frame_validated(project_id, video_id, frame_kind, at_secs)
            .await
    }

    async fn create_canvas_frame_record(
        &self,
        plan: CanvasFramePlan,
        file_path: String,
        file_size: i64,
    ) -> Result<GeneratedImageSummary, AppError> {
        self.create_canvas_frame_record_validated(plan, file_path, file_size)
            .await
    }

    async fn update(
        &self,
        user_id: i64,
        video_id: i64,
        input: UpdateGeneratedVideoInput,
    ) -> Result<GeneratedVideoSummary, AppError> {
        let existing = self.required_video_record(video_id).await?;
        self.verify_project_permission(
            existing.project_id,
            user_id,
            crate::collaboration::ProjectAccessRole::Editor,
        )
        .await?;
        Ok(generated_video_summary(
            &self.update_video_record_validated(video_id, input).await?,
        ))
    }

    async fn prepare_trim(
        &self,
        user_id: i64,
        video_id: i64,
        start_secs: f32,
        end_secs: f32,
    ) -> Result<VideoTrimPlan, AppError> {
        let source = self.required_video_record(video_id).await?;
        self.verify_project_permission(
            source.project_id,
            user_id,
            crate::collaboration::ProjectAccessRole::Editor,
        )
        .await?;
        self.prepare_video_trim_validated(source, start_secs, end_secs)
    }

    async fn create_trimmed_record(
        &self,
        plan: VideoTrimPlan,
    ) -> Result<GeneratedVideoSummary, AppError> {
        self.create_trimmed_video_record_validated(plan).await
    }

    async fn complete(
        &self,
        input: CompleteVideoRecordInput,
    ) -> Result<VideoRecordCompletion, VideoRecordCompletionError> {
        let CompleteVideoRecordInput {
            record_id,
            output_path,
            thumbnail_path,
            file_size,
            prompt_id,
            metadata_json,
            duration_secs,
        } = input;
        match self
            .mark_generated_video_completed_and_link_prompt(
                record_id,
                &output_path,
                thumbnail_path.as_deref(),
                file_size,
                metadata_json,
                prompt_id,
            )
            .await
        {
            Ok(_) => {}
            Err(error) => {
                let can_adopt = crate::video::can_adopt_authoritative_video_completion(&error);
                let authoritative = match self.generated_video_by_id(record_id).await {
                    Ok(authoritative) => authoritative,
                    Err(lookup_error) => {
                        return Err(VideoRecordCompletionError::Submission {
                            error,
                            authoritative: None,
                            lookup_error: Some(lookup_error),
                        });
                    }
                };
                if can_adopt
                    && let Some(authoritative) = authoritative.as_ref()
                    && authoritative.status == a4n_domain::video_workflow::VIDEO_STATUS_COMPLETED
                {
                    return Ok(VideoRecordCompletion {
                        summary: generated_video_summary(&authoritative),
                        adopted_authoritative: true,
                    });
                }
                return Err(VideoRecordCompletionError::Submission {
                    error,
                    authoritative: authoritative.map(|record| VideoRecordCompletion {
                        summary: generated_video_summary(&record),
                        adopted_authoritative: false,
                    }),
                    lookup_error: None,
                });
            }
        };
        let record = match self
            .update_generated_video_duration_record(record_id, duration_secs)
            .await
        {
            Ok(record) => record,
            Err(error) => {
                return Err(VideoRecordCompletionError::DurationWriteback { error });
            }
        };
        Ok(VideoRecordCompletion {
            summary: generated_video_summary(&record),
            adopted_authoritative: false,
        })
    }

    async fn select_segment(
        &self,
        user_id: i64,
        project_id: i64,
        chapter_id: i64,
        video_id: i64,
        input: SelectSegmentVideoInput,
    ) -> Result<GeneratedVideoSummary, AppError> {
        self.verify_project_permission(
            project_id,
            user_id,
            crate::collaboration::ProjectAccessRole::Editor,
        )
        .await?;
        Ok(generated_video_summary(
            &self
                .select_segment_video_validated(
                    project_id,
                    chapter_id,
                    input.segment_plan_prompt_id,
                    (
                        &input.target_segment.source_shot_id,
                        input.target_segment.segment_index_in_shot,
                    ),
                    video_id,
                )
                .await?,
        ))
    }

    async fn update_assembly_push_status(
        &self,
        user_id: i64,
        video_id: i64,
        input: UpdateAssemblyPushStatusInput,
    ) -> Result<GeneratedVideoSummary, AppError> {
        let existing = self.required_video_record(video_id).await?;
        self.verify_project_permission(
            existing.project_id,
            user_id,
            crate::collaboration::ProjectAccessRole::Editor,
        )
        .await?;
        Ok(generated_video_summary(
            &self
                .update_assembly_push_status_validated(existing.id, input)
                .await?,
        ))
    }

    async fn delete(&self, user_id: i64, video_id: i64) -> Result<(), AppError> {
        let project_ids = self.missing_video_reference_project_ids(video_id).await?;
        if project_ids.is_empty() {
            return Err(AppError::NotFound("视频不存在".to_string()));
        }
        self.verify_project_permissions(
            &project_ids,
            user_id,
            crate::collaboration::ProjectAccessRole::Editor,
        )
        .await?;
        self.delete_video_record_validated(video_id).await
    }

    async fn delete_record_with_cleanup(&self, video_id: i64) -> Result<(), AppError> {
        self.delete_generated_video_record_with_cleanup(video_id)
            .await
    }
}

fn assembly_insert_position_metadata_from_rows(
    rows: &[generated_video::Model],
    project_id: i64,
    chapter_id: i64,
    workflow_mode: &str,
    excluded_video_id: Option<i64>,
    adopted_video_ids: Option<&HashSet<i64>>,
    timeline_order: Option<&super::timeline::VideoTimelineAssemblyOrder>,
    base_metadata: Option<Value>,
    insert_after_video_id: Option<i64>,
) -> Result<Value, AppError> {
    let anchors = assembly_clip_records_from_rows(
        rows,
        project_id,
        chapter_id,
        workflow_mode,
        excluded_video_id,
        adopted_video_ids,
        timeline_order,
    )?
    .into_iter()
    .map(|clip| AssemblyClipOrderAnchor {
        video_id: clip.video_id,
        sort_order: clip.sort_order,
        shot_number: clip.shot_number,
    })
    .collect::<Vec<_>>();
    let clip_ids = anchors.iter().map(|clip| clip.video_id).collect::<Vec<_>>();
    let pushes = assembly_push_records(rows, workflow_mode).map_err(AppError::InvalidArgument)?;
    let active_source_ids = active_assembly_push_source_ids(
        pushes
            .iter()
            .map(|push| (push.status.as_str(), push.source_video_ids.as_slice())),
    );
    ensure_insert_position_not_inside_pushed_prefix(
        &clip_ids,
        &active_source_ids,
        insert_after_video_id,
    )
    .map_err(AppError::InvalidArgument)?;
    let (sort_order, insert_after_shot_number) =
        next_assembly_insert_sort_order(&anchors, insert_after_video_id)
            .map_err(AppError::InvalidArgument)?;
    assembly_insert_metadata_with_position(
        base_metadata,
        sort_order,
        insert_after_video_id,
        insert_after_shot_number,
    )
    .map_err(AppError::InvalidArgument)
}

fn required_workflow_mode<'a>(
    video: &'a generated_video::Model,
    label: &str,
) -> Result<&'a str, AppError> {
    video
        .workflow_mode
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| {
            AppError::InvalidArgument(format!("{label} #{} 缺少 workflow_mode", video.id))
        })
}

fn assembly_clip_records_from_rows(
    rows: &[generated_video::Model],
    project_id: i64,
    chapter_id: i64,
    workflow_mode: &str,
    excluded_video_id: Option<i64>,
    adopted_video_ids: Option<&HashSet<i64>>,
    timeline_order: Option<&super::timeline::VideoTimelineAssemblyOrder>,
) -> Result<Vec<AssemblyClipRecord>, AppError> {
    let mut clips = rows
        .iter()
        .filter_map(|video| {
            let file_path = video.file_path.as_deref()?.trim();
            (Some(video.id) != excluded_video_id
                && video.project_id == project_id
                && video.chapter_id == Some(chapter_id)
                && if workflow_mode == WORKFLOW_MODE_MIXED {
                    adopted_video_ids.is_some_and(|ids| ids.contains(&video.id))
                        || (video.video_type
                            == a4n_domain::video_workflow::VIDEO_TYPE_ASSEMBLY_INSERT
                            && video.workflow_mode.as_deref() == Some(WORKFLOW_MODE_MIXED))
                } else {
                    video.workflow_mode.as_deref() == Some(workflow_mode)
                }
                && is_assembly_clip_video_type(&video.video_type)
                && video.status == a4n_domain::video_workflow::VIDEO_STATUS_COMPLETED
                && !file_path.is_empty())
            .then_some((video, file_path))
        })
        .map(|(video, file_path)| {
            let timeline_display_index =
                if video.video_type == a4n_domain::video_workflow::VIDEO_TYPE_ASSEMBLY_INSERT {
                    None
                } else {
                    timeline_order.and_then(|order| order.display_index_for_video(video))
                };
            if timeline_order.is_some()
                && video.video_type != a4n_domain::video_workflow::VIDEO_TYPE_ASSEMBLY_INSERT
                && timeline_display_index.is_none()
            {
                return Err(AppError::InvalidArgument(format!(
                    "视频 #{} 不属于当前章节视频时间线",
                    video.id
                )));
            }
            let (shot_number, sort_order) = match timeline_display_index {
                Some(index) => (index, f64::from(index) * 1000.0),
                None => (
                    assembly_clip_display_shot_number(
                        video.id,
                        &video.video_type,
                        video.metadata_json.as_ref(),
                    )
                    .map_err(AppError::InvalidArgument)?,
                    assembly_clip_sort_order(
                        video.id,
                        &video.video_type,
                        video.metadata_json.as_ref(),
                    )
                    .map_err(AppError::InvalidArgument)?,
                ),
            };
            Ok(AssemblyClipRecord {
                video_id: video.id,
                video_type: video.video_type.clone(),
                shot_number,
                sort_order,
                insert_after_video_id: assembly_clip_insert_after_video_id(
                    video.id,
                    &video.video_type,
                    video.metadata_json.as_ref(),
                )
                .map_err(AppError::InvalidArgument)?,
                file_path: file_path.to_string(),
                duration_secs: video.duration_secs,
                width: video.width,
                height: video.height,
                fps: video.fps,
            })
        })
        .collect::<Result<Vec<_>, AppError>>()?;
    clips.sort_by(|left, right| {
        left.sort_order
            .total_cmp(&right.sort_order)
            .then_with(|| left.video_id.cmp(&right.video_id))
    });
    Ok(clips)
}

fn next_assembly_push_status(
    action: AssemblyPushAction,
    status: &str,
) -> Result<&'static str, AppError> {
    match (action, status) {
        (AssemblyPushAction::Accept, ASSEMBLY_PUSH_STATUS_PENDING) => {
            Ok(ASSEMBLY_PUSH_STATUS_ACCEPTED)
        }
        (AssemblyPushAction::Reject, ASSEMBLY_PUSH_STATUS_PENDING) => {
            Ok(ASSEMBLY_PUSH_STATUS_REJECTED)
        }
        (AssemblyPushAction::Undo, ASSEMBLY_PUSH_STATUS_ACCEPTED) => {
            Ok(ASSEMBLY_PUSH_STATUS_PENDING)
        }
        (AssemblyPushAction::Accept, _) => Err(AppError::InvalidArgument(
            "只有待处理推送可以同意".to_string(),
        )),
        (AssemblyPushAction::Reject, _) => Err(AppError::InvalidArgument(
            "只有待处理推送可以拒绝".to_string(),
        )),
        (AssemblyPushAction::Undo, _) => Err(AppError::InvalidArgument(
            "只有已同意的最新推送可以撤销".to_string(),
        )),
    }
}

fn imported_video_prompt_text(prompt: &chapter_video_prompt::Model) -> String {
    let mut sections = vec![prompt.visual_prompt.trim().to_string()];
    if let Some(motion) = prompt
        .motion_prompt
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        sections.push(format!("Motion Prompt:\n{motion}"));
    }
    sections.join("\n\n")
}

#[cfg(test)]
mod tests;
