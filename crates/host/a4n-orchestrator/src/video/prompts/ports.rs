use a4n_api_contract::video::{
    ChapterVideoPromptSummary, CommitInsertedSegmentDraftInput, DeleteInsertedSegmentInput,
    DownstreamConflictStrategy, GetVideoTimelineInput, LoadVideoSegmentPlanDraftInput,
    MoveInsertedSegmentInput, OptimizeChapterVideoPromptInput, UpdateChapterVideoPromptInput,
    UpdateChapterVideoSegmentPlanInput, UpdateVideoSegmentPlanDraftRecipeInput,
    VideoPromptDeleteResult, VideoPromptDownstreamCleanupPort, VideoPromptDraftApplicationPort,
    VideoPromptMutationApplicationPort, VideoPromptTimelineApplicationPort,
    VideoPromptUpdateResult, VideoSegmentPlanDraftSummary, VideoTimelineSummary,
};
use a4n_error::AppError;
use a4n_video_contract::VideoSegmentExecutionRecipeV1;
use async_trait::async_trait;

use crate::{OrchestratorContext, prompt_records::PromptRendererAdapter};

#[async_trait]
impl VideoPromptMutationApplicationPort for OrchestratorContext {
    async fn update_chapter_video_segment_plan(
        &self,
        user_id: i64,
        prompt_id: i64,
        input: UpdateChapterVideoSegmentPlanInput,
    ) -> Result<ChapterVideoPromptSummary, AppError> {
        self.update_chapter_video_segment_plan_validated(user_id, prompt_id, input)
            .await
    }

    async fn update_chapter_video_segment_recipe(
        &self,
        user_id: i64,
        prompt_id: i64,
        recipe: VideoSegmentExecutionRecipeV1,
    ) -> Result<Vec<VideoSegmentExecutionRecipeV1>, AppError> {
        self.update_chapter_video_segment_recipe_validated(user_id, prompt_id, recipe)
            .await
    }

    async fn update_chapter_video_prompt(
        &self,
        user_id: i64,
        prompt_id: i64,
        input: UpdateChapterVideoPromptInput,
        cleanup: &dyn VideoPromptDownstreamCleanupPort,
    ) -> Result<VideoPromptUpdateResult, AppError> {
        self.update_chapter_video_prompt_validated(user_id, prompt_id, input, cleanup)
            .await
    }

    async fn delete_chapter_video_prompt(
        &self,
        user_id: i64,
        prompt_id: i64,
        downstream_strategy: DownstreamConflictStrategy,
        cleanup: &dyn VideoPromptDownstreamCleanupPort,
    ) -> Result<VideoPromptDeleteResult, AppError> {
        self.delete_chapter_video_prompt_validated(user_id, prompt_id, downstream_strategy, cleanup)
            .await
    }

    async fn optimize_chapter_video_prompt(
        &self,
        user_id: i64,
        prompt_id: i64,
        input: OptimizeChapterVideoPromptInput,
        prompt_renderer: &dyn a4n_api_contract::prompt::PromptRenderPort,
        cleanup: &dyn VideoPromptDownstreamCleanupPort,
    ) -> Result<VideoPromptUpdateResult, AppError> {
        let prompt_renderer = PromptRendererAdapter(prompt_renderer);
        self.optimize_chapter_video_prompt_validated(
            user_id,
            prompt_id,
            input,
            &prompt_renderer,
            cleanup,
        )
        .await
    }
}

#[async_trait]
impl VideoPromptTimelineApplicationPort for OrchestratorContext {
    async fn get_video_timeline(
        &self,
        user_id: i64,
        input: GetVideoTimelineInput,
    ) -> Result<VideoTimelineSummary, AppError> {
        self.get_video_timeline(user_id, input).await
    }

    async fn commit_inserted_segment_draft(
        &self,
        user_id: i64,
        input: CommitInsertedSegmentDraftInput,
    ) -> Result<VideoTimelineSummary, AppError> {
        self.commit_inserted_segment_draft(user_id, input).await
    }

    async fn move_video_timeline_item(
        &self,
        user_id: i64,
        input: MoveInsertedSegmentInput,
    ) -> Result<VideoTimelineSummary, AppError> {
        self.move_video_timeline_item(user_id, input).await
    }

    async fn delete_video_timeline_item(
        &self,
        user_id: i64,
        input: DeleteInsertedSegmentInput,
    ) -> Result<VideoTimelineSummary, AppError> {
        self.delete_video_timeline_item(user_id, input).await
    }
}

#[async_trait]
impl VideoPromptDraftApplicationPort for OrchestratorContext {
    async fn load_video_segment_plan_draft(
        &self,
        user_id: i64,
        input: LoadVideoSegmentPlanDraftInput,
    ) -> Result<VideoSegmentPlanDraftSummary, AppError> {
        self.load_video_segment_plan_draft_validated(user_id, input)
            .await
    }

    async fn update_video_segment_plan_draft_recipe(
        &self,
        user_id: i64,
        input: UpdateVideoSegmentPlanDraftRecipeInput,
    ) -> Result<(), AppError> {
        self.update_video_segment_plan_draft_recipe_validated(user_id, input)
            .await
    }
}
