use super::*;

impl VideoRecordWorkerClient {
    pub async fn load_video_segment_plan_draft(
        &self,
        user_id: i64,
        input: LoadVideoSegmentPlanDraftInput,
    ) -> Result<a4n_api_contract::video::VideoSegmentPlanDraftSummary, AppError> {
        self.command(
            user_id,
            Some(input.project_id),
            VIDEO_PROMPT_DRAFT_LOAD_OPERATION,
            input,
        )
        .await
    }

    pub async fn update_video_segment_plan_draft_recipe(
        &self,
        user_id: i64,
        input: UpdateVideoSegmentPlanDraftRecipeInput,
    ) -> Result<(), AppError> {
        self.command(
            user_id,
            Some(input.project_id),
            VIDEO_PROMPT_DRAFT_UPDATE_OPERATION,
            input,
        )
        .await
    }
}

#[async_trait::async_trait]
impl VideoPromptReadApplicationPort for VideoRecordWorkerClient {
    async fn list_chapter(
        &self,
        user_id: i64,
        project_id: i64,
        chapter_id: i64,
    ) -> Result<Vec<ChapterVideoPromptSummary>, AppError> {
        self.command(
            user_id,
            Some(project_id),
            VIDEO_PROMPT_LIST_OPERATION,
            ChapterVideosInput {
                project_id,
                chapter_id,
            },
        )
        .await
    }

    async fn validate_project_chapter_source(
        &self,
        project_id: i64,
        chapter_id: i64,
        operation: &str,
        writable: bool,
    ) -> Result<(), AppError> {
        self.command_with_principal(
            Self::service_principal(),
            Some(project_id),
            VIDEO_PROMPT_VALIDATE_SOURCE_OPERATION,
            VideoSourceInput {
                project_id,
                chapter_id,
                operation: operation.to_string(),
                writable,
            },
        )
        .await
    }
}
