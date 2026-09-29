use a4n_api_contract::chapter_visual_plan::{
    ChapterVisualPlanReadResult, ChapterVisualPlanRecordInput,
    ChapterVisualPlanRecordPersistenceApplicationPort, ChapterVisualPlanRecordReadApplicationPort,
    ChapterVisualPlanSetView, ChapterVisualPlanSourceInput, ChapterVisualPlanValidationResult,
    ChapterVisualPlanView, ConfirmedChapterVisualPlanView, ReadChapterVisualPlanInput,
    SaveChapterVisualPlanDraftInput, ValidateChapterVisualPlanInput,
};
use a4n_error::AppError;
use a4n_novel_planning_worker_contract::{
    CONFIRM_VISUAL_PLAN_OPERATION, ConfirmVisualPlanPayload, DISCARD_VISUAL_PLAN_OPERATION,
    DiscardVisualPlanPayload, GET_VISUAL_PLAN_OPERATION, GetVisualPlanPayload,
    READ_VISUAL_PLAN_OPERATION, ReadVisualPlanPayload, SAVE_VISUAL_PLAN_DRAFT_OPERATION,
    SaveVisualPlanDraftPayload, VALIDATE_VISUAL_PLAN_OPERATION, ValidateVisualPlanPayload,
};

use super::NovelPlanningWorkerClient;

#[async_trait::async_trait]
impl ChapterVisualPlanRecordPersistenceApplicationPort for NovelPlanningWorkerClient {
    async fn save_draft(
        &self,
        user_id: i64,
        input: SaveChapterVisualPlanDraftInput,
    ) -> Result<ChapterVisualPlanView, AppError> {
        self.command::<SaveVisualPlanDraftPayload, ChapterVisualPlanView>(
            user_id,
            input.project_id,
            input.expected_revision,
            SAVE_VISUAL_PLAN_DRAFT_OPERATION,
            input,
        )
        .await
    }

    async fn confirm(
        &self,
        user_id: i64,
        input: ChapterVisualPlanRecordInput,
    ) -> Result<ConfirmedChapterVisualPlanView, AppError> {
        self.command::<ConfirmVisualPlanPayload, ConfirmedChapterVisualPlanView>(
            user_id,
            input.project_id,
            Some(input.expected_revision),
            CONFIRM_VISUAL_PLAN_OPERATION,
            input,
        )
        .await
    }

    async fn discard(
        &self,
        user_id: i64,
        input: ChapterVisualPlanRecordInput,
    ) -> Result<(), AppError> {
        self.command::<DiscardVisualPlanPayload, ()>(
            user_id,
            input.project_id,
            Some(input.expected_revision),
            DISCARD_VISUAL_PLAN_OPERATION,
            input,
        )
        .await
    }
}

#[async_trait::async_trait]
impl ChapterVisualPlanRecordReadApplicationPort for NovelPlanningWorkerClient {
    async fn get(
        &self,
        user_id: i64,
        input: ChapterVisualPlanSourceInput,
    ) -> Result<ChapterVisualPlanSetView, AppError> {
        self.command::<GetVisualPlanPayload, ChapterVisualPlanSetView>(
            user_id,
            input.project_id,
            None,
            GET_VISUAL_PLAN_OPERATION,
            input,
        )
        .await
    }

    async fn read(
        &self,
        user_id: i64,
        input: ReadChapterVisualPlanInput,
    ) -> Result<ChapterVisualPlanReadResult, AppError> {
        self.command::<ReadVisualPlanPayload, ChapterVisualPlanReadResult>(
            user_id,
            input.project_id,
            input.expected_revision,
            READ_VISUAL_PLAN_OPERATION,
            input,
        )
        .await
    }

    async fn validate(
        &self,
        user_id: i64,
        input: ValidateChapterVisualPlanInput,
    ) -> Result<ChapterVisualPlanValidationResult, AppError> {
        self.command::<ValidateVisualPlanPayload, ChapterVisualPlanValidationResult>(
            user_id,
            input.project_id,
            Some(input.expected_revision),
            VALIDATE_VISUAL_PLAN_OPERATION,
            input,
        )
        .await
    }
}
