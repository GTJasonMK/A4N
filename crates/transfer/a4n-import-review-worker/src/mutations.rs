use super::*;

impl WorkerState {
    pub(super) async fn delete(
        &self,
        request: WorkerEnvelope<ImportReviewIdInput>,
    ) -> Result<(), WorkerFailure> {
        validate_request(&request, IMPORT_REVIEW_DELETE_OPERATION)?;
        let user_id = request.principal.parse_user_id()?;
        let review = review_for_editor(&self.database, user_id, request.payload.review_id).await?;
        a4n_db::import_review::delete_import_review(self.database.connection(), review.id)
            .await
            .map_err(mutation_error)
    }

    pub(super) async fn reject(
        &self,
        request: WorkerEnvelope<ImportReviewIdInput>,
    ) -> Result<ImportReviewSummary, WorkerFailure> {
        validate_request(&request, IMPORT_REVIEW_REJECT_OPERATION)?;
        let user_id = request.principal.parse_user_id()?;
        let review = review_for_editor(&self.database, user_id, request.payload.review_id).await?;
        let review =
            a4n_db::import_review::reject_import_review(self.database.connection(), review.id)
                .await
                .map_err(mutation_error)?;
        summary_from_record(review).map_err(app_error)
    }
}
