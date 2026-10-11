use super::*;

/// Host-facing read boundary for persisted video records and validated media.
#[async_trait::async_trait]
pub trait VideoRecordReadApplicationPort: Send + Sync {
    async fn playback_source(
        &self,
        user_id: i64,
        video_id: i64,
    ) -> Result<GeneratedVideoSummary, a4n_error::AppError>;

    async fn list_by_chapter(
        &self,
        user_id: i64,
        project_id: i64,
        chapter_id: i64,
    ) -> Result<Vec<GeneratedVideoSummary>, a4n_error::AppError>;

    async fn validated_reference_images(
        &self,
        user_id: i64,
        project_id: i64,
        chapter_id: Option<i64>,
        reference_image_ids_json: Option<&Value>,
    ) -> Result<Vec<crate::image::GeneratedImageSummary>, a4n_error::AppError>;
}

/// Host-facing mutation boundary for ordinary video-record commands.
#[async_trait::async_trait]
pub trait VideoRecordMutationApplicationPort: Send + Sync {
    async fn update(
        &self,
        user_id: i64,
        video_id: i64,
        input: UpdateGeneratedVideoInput,
    ) -> Result<GeneratedVideoSummary, a4n_error::AppError>;

    async fn select_segment(
        &self,
        user_id: i64,
        project_id: i64,
        chapter_id: i64,
        video_id: i64,
        input: SelectSegmentVideoInput,
    ) -> Result<GeneratedVideoSummary, a4n_error::AppError>;

    async fn delete(&self, user_id: i64, video_id: i64) -> Result<(), a4n_error::AppError>;
}

/// Host-facing chapter-video import boundary.
///
/// Runtime supplies inspected file facts; the video owner creates the record
/// and enforces project, chapter, and workflow invariants.
#[async_trait::async_trait]
pub trait ChapterVideoImportApplicationPort: Send + Sync {
    async fn create_imported_chapter_video_record(
        &self,
        project_id: i64,
        chapter_id: i64,
        prompt_id: i64,
        facts: a4n_video_host_contract::ImportedVideoRecordFacts<'_>,
    ) -> Result<VideoImportRecordSummary, a4n_error::AppError>;
}

/// Host-facing reference-media video import boundary.
#[async_trait::async_trait]
pub trait ReferenceMediaVideoImportApplicationPort: Send + Sync {
    async fn create_imported_reference_media_video_record(
        &self,
        project_id: i64,
        chapter_id: i64,
        facts: a4n_video_host_contract::ImportedVideoRecordFacts<'_>,
        segment_plan_prompt_id: i64,
        target_source_shot_id: &str,
        target_segment_index: i32,
    ) -> Result<VideoImportRecordSummary, a4n_error::AppError>;
}

/// Host-facing Assembly insert-video import boundary.
#[async_trait::async_trait]
pub trait AssemblyVideoImportApplicationPort: Send + Sync {
    async fn create_imported_assembly_insert_video_record(
        &self,
        project_id: i64,
        chapter_id: i64,
        workflow_mode: &str,
        insert_after_video_id: Option<i64>,
        facts: a4n_video_host_contract::ImportedVideoRecordFacts<'_>,
    ) -> Result<VideoImportRecordSummary, a4n_error::AppError>;
}

/// Host-facing Canvas video import-record boundary.
#[async_trait::async_trait]
pub trait CanvasVideoImportApplicationPort: Send + Sync {
    async fn create_imported_canvas_video_record(
        &self,
        project_id: i64,
        facts: a4n_video_host_contract::ImportedVideoRecordFacts<'_>,
    ) -> Result<VideoImportRecordSummary, a4n_error::AppError>;
}

/// Host-facing video trim boundary.
#[async_trait::async_trait]
pub trait VideoTrimApplicationPort: Send + Sync {
    async fn prepare_trim(
        &self,
        user_id: i64,
        video_id: i64,
        start_secs: f32,
        end_secs: f32,
    ) -> Result<VideoTrimPlan, a4n_error::AppError>;

    async fn create_trimmed_record(
        &self,
        plan: VideoTrimPlan,
    ) -> Result<GeneratedVideoSummary, a4n_error::AppError>;
}

/// Host-facing Assembly postprocess boundary.
#[async_trait::async_trait]
pub trait VideoAssemblyPostprocessApplicationPort: Send + Sync {
    async fn prepare_assembly_postprocess_edit(
        &self,
        push_video_id: i64,
        operation: AssemblyPostprocessOperation,
    ) -> Result<AssemblyPostprocessPlan, a4n_error::AppError>;

    async fn prepare_assembly_postprocess_source(
        &self,
        plan: &AssemblyPostprocessPlan,
        source_video_id: Option<i64>,
        has_uploaded_source: bool,
    ) -> Result<AssemblyPostprocessSourcePlan, a4n_error::AppError>;

    async fn create_assembly_postprocess_edit_record(
        &self,
        plan: AssemblyPostprocessPlan,
        input: AssemblyPostprocessEditRecordInput,
    ) -> Result<PreparedAssemblyPostprocessEditRecord, a4n_error::AppError>;
}

/// Host-facing Assembly clip planning boundary.
#[async_trait::async_trait]
pub trait VideoAssemblyPlanningApplicationPort: Send + Sync {
    async fn assembly_records(
        &self,
        project_id: i64,
        chapter_id: i64,
        workflow_mode: &str,
        operation: &str,
    ) -> Result<AssemblyVideoRecords, a4n_error::AppError>;
}

/// Host-facing Assembly push boundary.
#[async_trait::async_trait]
pub trait VideoAssemblyPushApplicationPort: Send + Sync {
    async fn create_assembly_push_record(
        &self,
        project_id: i64,
        chapter_id: i64,
        workflow_mode: &str,
        clips: Vec<AssemblyPushClipInput>,
        end_video_id: Option<i64>,
    ) -> Result<PreparedAssemblyPushRecord, a4n_error::AppError>;

    async fn update_assembly_push_status(
        &self,
        user_id: i64,
        video_id: i64,
        input: UpdateAssemblyPushStatusInput,
    ) -> Result<GeneratedVideoSummary, a4n_error::AppError>;
}

/// Host-facing video completion boundary.
#[async_trait::async_trait]
pub trait VideoRecordCompletionApplicationPort: Send + Sync {
    async fn complete(
        &self,
        input: CompleteVideoRecordInput,
    ) -> Result<VideoRecordCompletion, VideoRecordCompletionError>;
}

/// Host-facing video tail-frame boundary.
#[async_trait::async_trait]
pub trait VideoTailFrameApplicationPort: Send + Sync {
    async fn create_video_tail_frame(
        &self,
        input: VideoTailFrameInput,
    ) -> Result<VideoTailFrameSummary, a4n_error::AppError>;

    async fn delete_video_tail_frame_assets(
        &self,
        video_id: i64,
    ) -> Result<(), a4n_error::AppError>;
}

/// Host-facing video failure-state boundary.
#[async_trait::async_trait]
pub trait VideoRecordFailureApplicationPort: Send + Sync {
    async fn mark_error_status_and_prompt(
        &self,
        video_id: i64,
        prompt_id: Option<i64>,
        status: &str,
        message: &str,
    ) -> Result<GeneratedVideoSummary, a4n_error::AppError>;
}

/// Host-facing media-cleanup lifecycle boundary.
#[async_trait::async_trait]
pub trait VideoRecordLifecycleApplicationPort: Send + Sync {
    async fn delete_record_with_cleanup(&self, video_id: i64) -> Result<(), a4n_error::AppError>;
}

/// Host-facing canvas-media preparation and persistence boundary.
#[async_trait::async_trait]
pub trait VideoCanvasMediaApplicationPort: Send + Sync {
    async fn prepare_canvas_assembly(
        &self,
        project_id: i64,
        video_ids: Vec<i64>,
        title: Option<&str>,
    ) -> Result<CanvasAssemblyPlan, a4n_error::AppError>;

    async fn create_canvas_assembly_record(
        &self,
        project_id: i64,
        plan: CanvasAssemblyPlan,
    ) -> Result<GeneratedVideoSummary, a4n_error::AppError>;

    async fn prepare_canvas_frame(
        &self,
        project_id: i64,
        video_id: i64,
        frame_kind: CanvasFrameKind,
        at_secs: Option<f32>,
    ) -> Result<CanvasFramePlan, a4n_error::AppError>;

    async fn create_canvas_frame_record(
        &self,
        plan: CanvasFramePlan,
        file_path: String,
        file_size: i64,
    ) -> Result<crate::image::GeneratedImageSummary, a4n_error::AppError>;
}
