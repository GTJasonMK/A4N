use super::*;

impl WorkerState {
    pub(super) async fn list(
        &self,
        request: WorkerEnvelope<ImportProjectReviewsInput>,
    ) -> Result<Vec<ImportReviewSummary>, WorkerFailure> {
        validate_request(&request, IMPORT_REVIEW_LIST_OPERATION)?;
        let user_id = request.principal.parse_user_id()?;
        let project_id = request.require_project_scope(request.payload.project_id)?;
        verify_editor(&self.database, project_id, user_id).await?;
        a4n_db::import_review::import_reviews_by_project(self.database.connection(), project_id)
            .await
            .map_err(a4n_worker_db_adapter::map_database_error)?
            .into_iter()
            .map(summary_from_record)
            .collect::<Result<Vec<_>, _>>()
            .map_err(app_error)
    }

    pub(super) async fn dedup(
        &self,
        request: WorkerEnvelope<CheckImportDedupInput>,
    ) -> Result<a4n_import_review_worker_contract::DedupResult, WorkerFailure> {
        validate_request(&request, IMPORT_REVIEW_DEDUP_OPERATION)?;
        let user_id = request.principal.parse_user_id()?;
        let project_id = request.require_project_scope(request.payload.project_id)?;
        let input = request.payload;
        verify_editor(&self.database, project_id, user_id).await?;
        let chapters =
            a4n_db::import_review::import_dedup_chapters(self.database.connection(), project_id)
                .await
                .map_err(a4n_worker_db_adapter::map_database_error)?;
        calculate_dedup(&input.content, &chapters).map_err(app_error)
    }
}
