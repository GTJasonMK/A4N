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
