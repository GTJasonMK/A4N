use super::*;

/// Host-provided cleanup for media that becomes unreachable after a prompt
/// mutation. The video owner decides which IDs are safe to remove.
#[async_trait::async_trait]
pub trait VideoPromptDownstreamCleanupPort: Send + Sync {
    async fn delete_downstream_media(
        &self,
        image_ids: &[i64],
        video_ids: &[i64],
    ) -> Result<(), a4n_error::AppError>;
}

/// Host-facing read boundary for persisted chapter video prompts.
///
/// Chapter ownership, authorization and ORM projection stay in the video
/// owner; Runtime only routes stable summaries.
#[async_trait::async_trait]
pub trait VideoPromptReadApplicationPort: Send + Sync {
    async fn list_chapter(
        &self,
        user_id: i64,
        project_id: i64,
        chapter_id: i64,
    ) -> Result<Vec<ChapterVideoPromptSummary>, a4n_error::AppError>;

    async fn validate_project_chapter_source(
        &self,
        project_id: i64,
        chapter_id: i64,
        operation: &str,
        writable: bool,
    ) -> Result<(), a4n_error::AppError>;
}

/// Host-facing read/write boundary for the chapter video timeline.
///
/// Timeline revision and ordering rules stay in the video owner; Runtime only
/// routes the stable timeline contract.
#[async_trait::async_trait]
pub trait VideoPromptTimelineApplicationPort: Send + Sync {
    async fn get_video_timeline(
        &self,
        user_id: i64,
        input: GetVideoTimelineInput,
    ) -> Result<VideoTimelineSummary, a4n_error::AppError>;

    async fn commit_inserted_segment_draft(
        &self,
        user_id: i64,
        input: CommitInsertedSegmentDraftInput,
    ) -> Result<VideoTimelineSummary, a4n_error::AppError>;

    async fn move_video_timeline_item(
        &self,
        user_id: i64,
        input: MoveInsertedSegmentInput,
    ) -> Result<VideoTimelineSummary, a4n_error::AppError>;

    async fn delete_video_timeline_item(
        &self,
        user_id: i64,
        input: DeleteInsertedSegmentInput,
    ) -> Result<VideoTimelineSummary, a4n_error::AppError>;
}

/// Host-facing boundary for loading and updating a segment-plan draft.
///
/// Workflow/checkpoint validation and the summary projection stay in the video
/// owner; Runtime only routes the contract.
#[async_trait::async_trait]
pub trait VideoPromptDraftApplicationPort: Send + Sync {
    async fn load_video_segment_plan_draft(
        &self,
        user_id: i64,
        input: LoadVideoSegmentPlanDraftInput,
    ) -> Result<VideoSegmentPlanDraftSummary, a4n_error::AppError>;

    async fn update_video_segment_plan_draft_recipe(
        &self,
        user_id: i64,
        input: UpdateVideoSegmentPlanDraftRecipeInput,
    ) -> Result<(), a4n_error::AppError>;
}

/// Host-facing mutation boundary for chapter video prompts and segment recipes.
///
/// Video ownership keeps authorization, dependency planning, persistence and
/// projection. Runtime supplies only cleanup and prompt rendering capabilities.
#[async_trait::async_trait]
pub trait VideoPromptMutationApplicationPort: Send + Sync {
    async fn update_chapter_video_segment_plan(
        &self,
        user_id: i64,
        prompt_id: i64,
        input: UpdateChapterVideoSegmentPlanInput,
    ) -> Result<ChapterVideoPromptSummary, a4n_error::AppError>;

    async fn update_chapter_video_segment_recipe(
        &self,
        user_id: i64,
        prompt_id: i64,
        recipe: VideoSegmentExecutionRecipeV1,
    ) -> Result<Vec<VideoSegmentExecutionRecipeV1>, a4n_error::AppError>;

    async fn update_chapter_video_prompt(
        &self,
        user_id: i64,
        prompt_id: i64,
        input: UpdateChapterVideoPromptInput,
        cleanup: &dyn VideoPromptDownstreamCleanupPort,
    ) -> Result<ChapterVideoPromptSummary, a4n_error::AppError>;

    async fn delete_chapter_video_prompt(
        &self,
        user_id: i64,
        prompt_id: i64,
        downstream_strategy: DownstreamConflictStrategy,
        cleanup: &dyn VideoPromptDownstreamCleanupPort,
    ) -> Result<(), a4n_error::AppError>;

    async fn optimize_chapter_video_prompt(
        &self,
        user_id: i64,
        prompt_id: i64,
        input: OptimizeChapterVideoPromptInput,
        prompt_renderer: &dyn crate::prompt::PromptRenderPort,
        cleanup: &dyn VideoPromptDownstreamCleanupPort,
    ) -> Result<ChapterVideoPromptSummary, a4n_error::AppError>;
}
