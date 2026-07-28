//! 视频领域 runtime 窄 facade。

use a4n_api_contract::{generation_tasks::GenerationTaskHandle, video::StartVideoGenerationInput};
use a4n_core::error::AppError;
use a4n_core::models::ai::{generated_image, generated_video};
use a4n_orchestrator::{
    GenerationTaskAccessError, GenerationTaskStart, OrchestratorContext,
    video::{
        ChapterVideoPromptSummary, DeleteChapterVideoPromptGroupResult,
        GenerateChapterVideoPromptInput, OptimizeChapterVideoPromptInput,
        ProjectChapterVideoSource, UpdateAssemblyPushStatusInput, UpdateGeneratedVideoInput,
        VideoPromptApplicationContext, VideoPromptChunk, VideoPromptDownstreamCleanupPort,
        VideoPromptProgress, VideoPromptRequestPreview, VideoPromptTaskScope,
        VideoPromptUpdateApplicationInput, VideoRecordLifecycleHostPort,
    },
};
use a4n_prompt::rendering::PromptRenderer;
use serde_json::Value;

pub(crate) struct VideoRuntimeFacade<'a> {
    orchestrator: &'a OrchestratorContext,
}

impl<'a> VideoRuntimeFacade<'a> {
    pub(super) fn new(orchestrator: &'a OrchestratorContext) -> Self {
        Self { orchestrator }
    }

    pub(crate) async fn start_video_generation_task(
        &self,
        user_id: i64,
        input: StartVideoGenerationInput,
    ) -> Result<GenerationTaskStart, GenerationTaskAccessError> {
        self.orchestrator
            .start_video_generation_task(user_id, input)
            .await
    }

    pub(crate) async fn run_video_generation_task<H>(
        &self,
        user_id: i64,
        input: StartVideoGenerationInput,
        handle: GenerationTaskHandle,
        host_port: &H,
    ) -> Result<(), String>
    where
        H: a4n_orchestrator::video::ChapterVideoGenerationHostPort
            + a4n_orchestrator::video::ReferenceMediaVideoGenerationHostPort,
    {
        self.orchestrator
            .run_video_generation_task(user_id, input, handle, host_port)
            .await
    }

    // --- records / images / videos ---

    pub(crate) async fn generated_video_by_id(
        &self,
        video_id: i64,
    ) -> Result<Option<generated_video::Model>, AppError> {
        self.orchestrator.generated_video_by_id(video_id).await
    }

    pub(crate) async fn resolve_video_config_for_provider_job(
        &self,
        project_id: Option<i64>,
        provider_kind: &str,
        provider_job_id: &str,
    ) -> Result<a4n_core::models::ai::video_gen_config::Model, AppError> {
        self.orchestrator
            .resolve_video_config_for_provider_job(project_id, provider_kind, provider_job_id)
            .await
    }

    pub(crate) async fn list_video_records_by_chapter_validated(
        &self,
        project_id: i64,
        chapter_id: i64,
    ) -> Result<Vec<generated_video::Model>, AppError> {
        self.orchestrator
            .list_video_records_by_chapter_validated(project_id, chapter_id)
            .await
    }

    pub(crate) async fn toggle_video_record_favorite_validated(
        &self,
        video_id: i64,
    ) -> Result<generated_video::Model, AppError> {
        self.orchestrator
            .toggle_video_record_favorite_validated(video_id)
            .await
    }

    pub(crate) async fn update_video_record_validated(
        &self,
        video_id: i64,
        input: UpdateGeneratedVideoInput,
    ) -> Result<generated_video::Model, AppError> {
        self.orchestrator
            .update_video_record_validated(video_id, input)
            .await
    }

    pub(crate) async fn select_segment_video_validated(
        &self,
        project_id: i64,
        chapter_id: i64,
        shot_number: i32,
        video_id: i64,
    ) -> Result<generated_video::Model, AppError> {
        self.orchestrator
            .select_segment_video_validated(project_id, chapter_id, shot_number, video_id)
            .await
    }

    pub(crate) async fn delete_video_record_validated(
        &self,
        host_port: &dyn VideoRecordLifecycleHostPort,
        video_id: i64,
    ) -> Result<(), AppError> {
        self.orchestrator
            .delete_video_record_validated(host_port, video_id)
            .await
    }

    pub(crate) async fn delete_chapter_video_records_validated(
        &self,
        host_port: &dyn VideoRecordLifecycleHostPort,
        project_id: i64,
        chapter_id: i64,
    ) -> Result<u64, AppError> {
        self.orchestrator
            .delete_chapter_video_records_validated(host_port, project_id, chapter_id)
            .await
    }

    pub(crate) async fn update_assembly_push_status_validated(
        &self,
        video_id: i64,
        input: UpdateAssemblyPushStatusInput,
    ) -> Result<generated_video::Model, AppError> {
        self.orchestrator
            .update_assembly_push_status_validated(video_id, input)
            .await
    }

    pub(crate) async fn assembly_video_records_validated(
        &self,
        project_id: i64,
        chapter_id: i64,
        workflow_mode: &str,
        operation: &str,
    ) -> Result<a4n_orchestrator::video::AssemblyVideoRecords, AppError> {
        self.orchestrator
            .assembly_video_records_validated(project_id, chapter_id, workflow_mode, operation)
            .await
    }

    pub(crate) async fn create_imported_assembly_insert_video_record_validated(
        &self,
        project_id: i64,
        chapter_id: i64,
        workflow_mode: &str,
        insert_after_video_id: Option<i64>,
        properties: a4n_orchestrator::video::ImportedVideoProperties,
        source_file_name: &str,
        source_extension: &str,
        imported_at: &str,
    ) -> Result<generated_video::Model, AppError> {
        self.orchestrator
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
            .await
    }

    pub(crate) async fn prepare_assembly_postprocess_edit_validated(
        &self,
        push_video_id: i64,
        operation: &str,
    ) -> Result<a4n_orchestrator::video::AssemblyPostprocessPlan, AppError> {
        self.orchestrator
            .prepare_assembly_postprocess_edit_validated(push_video_id, operation)
            .await
    }

    pub(crate) async fn create_assembly_postprocess_edit_record_validated(
        &self,
        plan: a4n_orchestrator::video::AssemblyPostprocessPlan,
        input: a4n_orchestrator::video::AssemblyPostprocessEditRecordInput,
    ) -> Result<a4n_orchestrator::video::PreparedAssemblyPostprocessEditRecord, AppError> {
        self.orchestrator
            .create_assembly_postprocess_edit_record_validated(plan, input)
            .await
    }

    pub(crate) async fn prepare_assembly_postprocess_source_validated(
        &self,
        plan: &a4n_orchestrator::video::AssemblyPostprocessPlan,
        source_video_id: Option<i64>,
        has_uploaded_source: bool,
    ) -> Result<a4n_orchestrator::video::AssemblyPostprocessSourcePlan, AppError> {
        self.orchestrator
            .prepare_assembly_postprocess_source_validated(
                plan,
                source_video_id,
                has_uploaded_source,
            )
            .await
    }

    pub(crate) async fn create_assembly_push_record_validated(
        &self,
        project_id: i64,
        chapter_id: i64,
        workflow_mode: &str,
        clips: Vec<a4n_orchestrator::video::AssemblyPushClipInput>,
        end_video_id: Option<i64>,
    ) -> Result<a4n_orchestrator::video::PreparedAssemblyPushRecord, AppError> {
        self.orchestrator
            .create_assembly_push_record_validated(
                project_id,
                chapter_id,
                workflow_mode,
                clips,
                end_video_id,
            )
            .await
    }

    pub(crate) async fn create_imported_chapter_video_record_validated(
        &self,
        project_id: i64,
        chapter_id: i64,
        prompt_id: i64,
        properties: a4n_orchestrator::video::ImportedVideoProperties,
        source_file_name: &str,
        source_extension: &str,
        imported_at: &str,
    ) -> Result<a4n_orchestrator::video::PreparedImportedChapterVideoRecord, AppError> {
        self.orchestrator
            .create_imported_chapter_video_record_validated(
                project_id,
                chapter_id,
                prompt_id,
                properties,
                source_file_name,
                source_extension,
                imported_at,
            )
            .await
    }

    pub(crate) async fn create_imported_reference_media_video_record_validated(
        &self,
        project_id: i64,
        chapter_id: i64,
        prompt: &str,
        declared_duration_secs: i32,
        aspect_ratio: &str,
        properties: a4n_orchestrator::video::ImportedVideoProperties,
        source_file_name: &str,
        source_extension: &str,
        imported_at: &str,
        segment_prompt_id: i64,
        shot_number: i32,
    ) -> Result<generated_video::Model, AppError> {
        self.orchestrator
            .create_imported_reference_media_video_record_validated(
                project_id,
                chapter_id,
                prompt,
                declared_duration_secs,
                aspect_ratio,
                properties,
                source_file_name,
                source_extension,
                imported_at,
                segment_prompt_id,
                shot_number,
            )
            .await
    }

    pub(crate) async fn create_imported_canvas_video_record_validated(
        &self,
        project_id: i64,
        prompt: Option<&str>,
        properties: a4n_orchestrator::video::ImportedVideoProperties,
        source_file_name: &str,
        source_extension: &str,
        imported_at: &str,
    ) -> Result<generated_video::Model, AppError> {
        self.orchestrator
            .create_imported_canvas_video_record_validated(
                project_id,
                prompt,
                properties,
                source_file_name,
                source_extension,
                imported_at,
            )
            .await
    }

    pub(crate) async fn prepare_canvas_assembly_validated(
        &self,
        project_id: i64,
        video_ids: Vec<i64>,
        title: Option<&str>,
    ) -> Result<a4n_orchestrator::video::CanvasAssemblyPlan, AppError> {
        self.orchestrator
            .prepare_canvas_assembly_validated(project_id, video_ids, title)
            .await
    }

    pub(crate) async fn prepare_canvas_frame_validated(
        &self,
        project_id: i64,
        video_id: i64,
        frame_kind: &str,
        at_secs: Option<f32>,
    ) -> Result<a4n_orchestrator::video::CanvasFramePlan, AppError> {
        self.orchestrator
            .prepare_canvas_frame_validated(project_id, video_id, frame_kind, at_secs)
            .await
    }

    pub(crate) async fn create_canvas_frame_record_validated(
        &self,
        plan: a4n_orchestrator::video::CanvasFramePlan,
        file_path: String,
        file_size: i64,
    ) -> Result<generated_image::Model, AppError> {
        self.orchestrator
            .create_canvas_frame_record_validated(plan, file_path, file_size)
            .await
    }

    pub(crate) async fn create_video_tail_frame_record_validated(
        &self,
        video: &generated_video::Model,
        chapter_id: i64,
        prompt_id: Option<i64>,
        file_path: String,
        file_size: i64,
    ) -> Result<generated_image::Model, AppError> {
        self.orchestrator
            .create_video_tail_frame_record_validated(
                video, chapter_id, prompt_id, file_path, file_size,
            )
            .await
    }

    pub(crate) async fn create_canvas_assembly_record_validated(
        &self,
        project_id: i64,
        plan: a4n_orchestrator::video::CanvasAssemblyPlan,
    ) -> Result<generated_video::Model, AppError> {
        self.orchestrator
            .create_canvas_assembly_record_validated(project_id, plan)
            .await
    }

    pub(crate) async fn prepare_video_trim_validated(
        &self,
        video_id: i64,
        start_secs: f32,
        end_secs: f32,
    ) -> Result<a4n_orchestrator::video::VideoTrimPlan, AppError> {
        self.orchestrator
            .prepare_video_trim_validated(video_id, start_secs, end_secs)
            .await
    }

    pub(crate) async fn create_trimmed_video_record_validated(
        &self,
        plan: a4n_orchestrator::video::VideoTrimPlan,
    ) -> Result<generated_video::Model, AppError> {
        self.orchestrator
            .create_trimmed_video_record_validated(plan)
            .await
    }

    pub(crate) async fn preview_reference_media_video_request_validated(
        &self,
        host_port: &dyn a4n_orchestrator::video::ReferenceMediaVideoGenerationHostPort,
        input: a4n_orchestrator::video::GenerateReferenceMediaVideoInput,
    ) -> Result<a4n_orchestrator::video::ReferenceMediaVideoRequestPreview, AppError> {
        self.orchestrator
            .preview_reference_media_video_request_validated(host_port, input)
            .await
    }

    pub(crate) async fn preview_chapter_video_request_validated(
        &self,
        host_port: &dyn a4n_orchestrator::video::ChapterVideoGenerationHostPort,
        input: a4n_orchestrator::video::GenerateChapterVideoInput,
        prompt_override: Option<a4n_orchestrator::video::PreviewChapterVideoPromptOverrideInput>,
    ) -> Result<a4n_orchestrator::video::ChapterVideoRequestPreview, AppError> {
        self.orchestrator
            .preview_chapter_video_request_validated(host_port, input, prompt_override)
            .await
    }

    pub(crate) async fn poll_video_provider_once_validated(
        &self,
        video: generated_video::Model,
        config: &a4n_core::models::ai::video_gen_config::Model,
        prompt_id: Option<i64>,
        cancel_token: tokio_util::sync::CancellationToken,
        host_port: &dyn a4n_orchestrator::video::VideoProviderHostPort,
    ) -> Result<generated_video::Model, AppError> {
        self.orchestrator
            .poll_video_provider_once_validated(video, config, prompt_id, cancel_token, host_port)
            .await
    }

    pub(crate) async fn delete_generated_video_record(
        &self,
        video_id: i64,
    ) -> Result<(), AppError> {
        self.orchestrator
            .delete_generated_video_record_with_cleanup(video_id)
            .await
    }

    pub(crate) async fn missing_video_reference_project_ids(
        &self,
        video_id: i64,
    ) -> Result<Vec<i64>, AppError> {
        self.orchestrator
            .missing_video_reference_project_ids(video_id)
            .await
    }

    /// 通用 error 状态（导入失败等无 generation_run 合同场景）。
    pub(crate) async fn mark_generated_video_error_status_and_prompt(
        &self,
        video_id: i64,
        prompt_id: Option<i64>,
        status: &str,
        message: &str,
    ) -> Result<generated_video::Model, AppError> {
        self.orchestrator
            .mark_generated_video_error_status_and_prompt(video_id, prompt_id, status, message)
            .await
    }

    pub(crate) async fn mark_generated_video_completed_and_link_prompt(
        &self,
        video_id: i64,
        file_path: &str,
        thumbnail_path: Option<&str>,
        file_size: i64,
        metadata_json: Option<Value>,
        prompt_id: Option<i64>,
    ) -> Result<generated_video::Model, AppError> {
        self.orchestrator
            .mark_generated_video_completed_and_link_prompt(
                video_id,
                file_path,
                thumbnail_path,
                file_size,
                metadata_json,
                prompt_id,
            )
            .await
    }

    pub(crate) async fn generated_images_by_ids(
        &self,
        image_ids: &[i64],
    ) -> Result<Vec<generated_image::Model>, AppError> {
        self.orchestrator.generated_images_by_ids(image_ids).await
    }

    pub(crate) async fn generated_image_by_id(
        &self,
        image_id: i64,
    ) -> Result<Option<generated_image::Model>, AppError> {
        self.orchestrator.generated_image_by_id(image_id).await
    }

    pub(crate) async fn video_tail_frame_images(
        &self,
    ) -> Result<Vec<generated_image::Model>, AppError> {
        self.orchestrator.video_tail_frame_images().await
    }

    pub(crate) async fn video_asset_project_ids(
        &self,
        project_id: i64,
    ) -> Result<Vec<i64>, AppError> {
        self.orchestrator.video_asset_project_ids(project_id).await
    }

    pub(crate) async fn validated_video_reference_images(
        &self,
        project_id: i64,
        chapter_id: i64,
        reference_image_ids_json: Option<&Value>,
    ) -> Result<Vec<generated_image::Model>, AppError> {
        self.orchestrator
            .validated_video_reference_images(project_id, chapter_id, reference_image_ids_json)
            .await
    }

    // --- prompts ---

    pub(crate) async fn video_prompt_task_scope_validated(
        &self,
        input: &GenerateChapterVideoPromptInput,
    ) -> Result<VideoPromptTaskScope, AppError> {
        self.orchestrator
            .video_prompt_task_scope_validated(input)
            .await
    }

    pub(crate) async fn list_chapter_video_prompts_validated(
        &self,
        user_id: i64,
        project_id: i64,
        chapter_id: i64,
    ) -> Result<Vec<ChapterVideoPromptSummary>, AppError> {
        self.orchestrator
            .list_chapter_video_prompts_validated(user_id, project_id, chapter_id)
            .await
    }

    pub(crate) async fn update_chapter_video_prompt_validated(
        &self,
        user_id: i64,
        prompt_id: i64,
        input: VideoPromptUpdateApplicationInput,
        cleanup: &dyn VideoPromptDownstreamCleanupPort,
    ) -> Result<ChapterVideoPromptSummary, AppError> {
        self.orchestrator
            .update_chapter_video_prompt_validated(user_id, prompt_id, input, cleanup)
            .await
    }

    pub(crate) async fn delete_chapter_video_prompt_validated(
        &self,
        user_id: i64,
        prompt_id: i64,
        strategy: a4n_core::dependency_conflict::DownstreamConflictStrategy,
        cleanup: &dyn VideoPromptDownstreamCleanupPort,
    ) -> Result<(), AppError> {
        self.orchestrator
            .delete_chapter_video_prompt_validated(user_id, prompt_id, strategy, cleanup)
            .await
    }

    pub(crate) async fn delete_chapter_video_prompt_group_validated(
        &self,
        user_id: i64,
        project_id: i64,
        chapter_id: i64,
        group: &str,
        strategy: a4n_core::dependency_conflict::DownstreamConflictStrategy,
        cleanup: &dyn VideoPromptDownstreamCleanupPort,
    ) -> Result<DeleteChapterVideoPromptGroupResult, AppError> {
        self.orchestrator
            .delete_chapter_video_prompt_group_validated(
                user_id, project_id, chapter_id, group, strategy, cleanup,
            )
            .await
    }

    pub(crate) async fn optimize_chapter_video_prompt_validated(
        &self,
        user_id: i64,
        prompt_id: i64,
        input: OptimizeChapterVideoPromptInput,
        prompt_renderer: &dyn PromptRenderer,
        cleanup: &dyn VideoPromptDownstreamCleanupPort,
    ) -> Result<ChapterVideoPromptSummary, AppError> {
        self.orchestrator
            .optimize_chapter_video_prompt_validated(
                user_id,
                prompt_id,
                input,
                prompt_renderer,
                cleanup,
            )
            .await
    }

    pub(crate) async fn project_chapter_video_source(
        &self,
        project_id: i64,
        chapter_id: i64,
        operation: &str,
        writable: bool,
    ) -> Result<ProjectChapterVideoSource, AppError> {
        self.orchestrator
            .project_chapter_video_source(project_id, chapter_id, operation, writable)
            .await
    }

    // --- generation / preview ---

    pub(crate) async fn generate_chapter_video_frame_prompts_validated(
        &self,
        context: VideoPromptApplicationContext<'_>,
        input: GenerateChapterVideoPromptInput,
        progress: Option<VideoPromptProgress<'_>>,
        chunks: Option<VideoPromptChunk<'_>>,
        cancel_token: tokio_util::sync::CancellationToken,
    ) -> Result<Vec<ChapterVideoPromptSummary>, AppError> {
        self.orchestrator
            .generate_chapter_video_frame_prompts_validated(
                context,
                input,
                progress,
                chunks,
                cancel_token,
            )
            .await
    }

    pub(crate) async fn generate_chapter_video_segment_plan_validated(
        &self,
        context: VideoPromptApplicationContext<'_>,
        input: GenerateChapterVideoPromptInput,
        progress: Option<VideoPromptProgress<'_>>,
        chunks: Option<VideoPromptChunk<'_>>,
        cancel_token: tokio_util::sync::CancellationToken,
    ) -> Result<Vec<ChapterVideoPromptSummary>, AppError> {
        self.orchestrator
            .generate_chapter_video_segment_plan_validated(
                context,
                input,
                progress,
                chunks,
                cancel_token,
            )
            .await
    }

    pub(crate) async fn generate_chapter_video_prompts_validated(
        &self,
        context: VideoPromptApplicationContext<'_>,
        input: GenerateChapterVideoPromptInput,
        progress: Option<VideoPromptProgress<'_>>,
        chunks: Option<VideoPromptChunk<'_>>,
        cancel_token: tokio_util::sync::CancellationToken,
    ) -> Result<Vec<ChapterVideoPromptSummary>, AppError> {
        self.orchestrator
            .generate_chapter_video_prompts_validated(
                context,
                input,
                progress,
                chunks,
                cancel_token,
            )
            .await
    }

    pub(crate) async fn preview_chapter_video_frame_prompt_requests_validated(
        &self,
        context: VideoPromptApplicationContext<'_>,
        input: GenerateChapterVideoPromptInput,
    ) -> Result<VideoPromptRequestPreview, AppError> {
        self.orchestrator
            .preview_chapter_video_frame_prompt_requests_validated(context, input)
            .await
    }

    pub(crate) async fn preview_chapter_video_prompt_requests_validated(
        &self,
        context: VideoPromptApplicationContext<'_>,
        input: GenerateChapterVideoPromptInput,
    ) -> Result<VideoPromptRequestPreview, AppError> {
        self.orchestrator
            .preview_chapter_video_prompt_requests_validated(context, input)
            .await
    }
}
