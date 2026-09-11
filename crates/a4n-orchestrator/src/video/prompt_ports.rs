use a4n_api_contract::video::{
    ChapterVideoPromptSummary, CommitInsertedSegmentDraftInput, DeleteInsertedSegmentInput,
    GetVideoTimelineInput, LoadVideoSegmentPlanDraftInput, MoveInsertedSegmentInput,
    OptimizeChapterVideoPromptInput, UpdateChapterVideoPromptInput,
    UpdateVideoSegmentPlanDraftRecipeInput, VideoPromptDownstreamCleanupPort, VideoTimelineSummary,
};
use a4n_domain::DownstreamConflictStrategy;
use a4n_error::AppError;
use a4n_prompt::rendering::PromptRenderer;
use a4n_video_contract::VideoSegmentExecutionRecipeV1;
use async_trait::async_trait;

use super::prompt_application::VideoSegmentPlanDraftState;
use crate::OrchestratorContext;

/// 提示词编辑、删除和执行方案变更的应用边界。
///
/// 下游媒体清理和提示词优化渲染由 Runtime 以显式 Host port/能力注入，
/// 权限、依赖策略、数据库写入和 DTO 投影仍由视频 owner 负责。
#[async_trait]
pub trait VideoPromptMutationApplicationPort: Send + Sync {
    async fn update_chapter_video_segment_recipe(
        &self,
        user_id: i64,
        prompt_id: i64,
        recipe: VideoSegmentExecutionRecipeV1,
    ) -> Result<Vec<VideoSegmentExecutionRecipeV1>, AppError>;

    async fn update_chapter_video_prompt(
        &self,
        user_id: i64,
        prompt_id: i64,
        input: UpdateChapterVideoPromptInput,
        cleanup: &dyn VideoPromptDownstreamCleanupPort,
    ) -> Result<ChapterVideoPromptSummary, AppError>;

    async fn delete_chapter_video_prompt(
        &self,
        user_id: i64,
        prompt_id: i64,
        downstream_strategy: DownstreamConflictStrategy,
        cleanup: &dyn VideoPromptDownstreamCleanupPort,
    ) -> Result<(), AppError>;

    async fn optimize_chapter_video_prompt(
        &self,
        user_id: i64,
        prompt_id: i64,
        input: OptimizeChapterVideoPromptInput,
        prompt_renderer: &dyn PromptRenderer,
        cleanup: &dyn VideoPromptDownstreamCleanupPort,
    ) -> Result<ChapterVideoPromptSummary, AppError>;
}

/// 章节视频时间线的读写边界。
#[async_trait]
pub trait VideoPromptTimelineApplicationPort: Send + Sync {
    async fn get_video_timeline(
        &self,
        user_id: i64,
        input: GetVideoTimelineInput,
    ) -> Result<VideoTimelineSummary, AppError>;

    async fn commit_inserted_segment_draft(
        &self,
        user_id: i64,
        input: CommitInsertedSegmentDraftInput,
    ) -> Result<VideoTimelineSummary, AppError>;

    async fn move_video_timeline_item(
        &self,
        user_id: i64,
        input: MoveInsertedSegmentInput,
    ) -> Result<VideoTimelineSummary, AppError>;

    async fn delete_video_timeline_item(
        &self,
        user_id: i64,
        input: DeleteInsertedSegmentInput,
    ) -> Result<VideoTimelineSummary, AppError>;
}

/// 片段规划草稿的读取与 checkpoint 更新边界。
#[async_trait]
pub trait VideoPromptDraftApplicationPort: Send + Sync {
    async fn load_video_segment_plan_draft(
        &self,
        user_id: i64,
        input: LoadVideoSegmentPlanDraftInput,
    ) -> Result<VideoSegmentPlanDraftState, AppError>;

    async fn update_video_segment_plan_draft_recipe(
        &self,
        user_id: i64,
        input: UpdateVideoSegmentPlanDraftRecipeInput,
    ) -> Result<(), AppError>;
}

#[async_trait]
impl VideoPromptMutationApplicationPort for OrchestratorContext {
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
    ) -> Result<ChapterVideoPromptSummary, AppError> {
        self.update_chapter_video_prompt_validated(user_id, prompt_id, input, cleanup)
            .await
    }

    async fn delete_chapter_video_prompt(
        &self,
        user_id: i64,
        prompt_id: i64,
        downstream_strategy: DownstreamConflictStrategy,
        cleanup: &dyn VideoPromptDownstreamCleanupPort,
    ) -> Result<(), AppError> {
        self.delete_chapter_video_prompt_validated(user_id, prompt_id, downstream_strategy, cleanup)
            .await
    }

    async fn optimize_chapter_video_prompt(
        &self,
        user_id: i64,
        prompt_id: i64,
        input: OptimizeChapterVideoPromptInput,
        prompt_renderer: &dyn PromptRenderer,
        cleanup: &dyn VideoPromptDownstreamCleanupPort,
    ) -> Result<ChapterVideoPromptSummary, AppError> {
        self.optimize_chapter_video_prompt_validated(
            user_id,
            prompt_id,
            input,
            prompt_renderer,
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
    ) -> Result<VideoSegmentPlanDraftState, AppError> {
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
