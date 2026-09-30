use super::*;

#[async_trait::async_trait]
impl ImportReviewMutationApplicationPort for ImportReviewWorkerClient {
    async fn delete(&self, user_id: i64, input: ImportReviewIdInput) -> Result<(), AppError> {
        self.command(user_id, None, IMPORT_REVIEW_DELETE_OPERATION, input)
            .await
    }

    async fn reject(
        &self,
        user_id: i64,
        input: ImportReviewIdInput,
    ) -> Result<ImportReviewSummary, AppError> {
        self.command(user_id, None, IMPORT_REVIEW_REJECT_OPERATION, input)
            .await
    }
}
