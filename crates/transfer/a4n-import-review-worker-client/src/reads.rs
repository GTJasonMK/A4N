use super::*;

#[async_trait::async_trait]
impl ImportReviewReadApplicationPort for ImportReviewWorkerClient {
    async fn list(
        &self,
        user_id: i64,
        input: ImportProjectReviewsInput,
    ) -> Result<Vec<ImportReviewSummary>, AppError> {
        self.command(
            user_id,
            Some(input.project_id),
            IMPORT_REVIEW_LIST_OPERATION,
            input,
        )
        .await
    }

    async fn list_index(
        &self,
        user_id: i64,
        input: ImportProjectReviewsInput,
    ) -> Result<Vec<ImportReviewListItem>, AppError> {
        self.command(
            user_id,
            Some(input.project_id),
            IMPORT_REVIEW_LIST_INDEX_OPERATION,
            input,
        )
        .await
    }

    async fn detail(
        &self,
        user_id: i64,
        input: ImportReviewIdInput,
    ) -> Result<ImportReviewSummary, AppError> {
        self.command(user_id, None, IMPORT_REVIEW_DETAIL_OPERATION, input)
            .await
    }

    async fn dedup(
        &self,
        user_id: i64,
        input: CheckImportDedupInput,
    ) -> Result<DedupResult, AppError> {
        self.command(
            user_id,
            Some(input.project_id),
            IMPORT_REVIEW_DEDUP_OPERATION,
            input,
        )
        .await
    }
}
