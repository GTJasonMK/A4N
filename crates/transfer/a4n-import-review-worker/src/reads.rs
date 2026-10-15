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

    pub(super) async fn list_index(
        &self,
        request: WorkerEnvelope<ImportProjectReviewsInput>,
    ) -> Result<Vec<ImportReviewListItem>, WorkerFailure> {
        validate_request(&request, IMPORT_REVIEW_LIST_INDEX_OPERATION)?;
        let user_id = request.principal.require_user_id()?;
        let project_id = request.require_project_scope(request.payload.project_id)?;
        verify_editor(&self.database, project_id, user_id).await?;
        a4n_db::import_review::import_review_list_by_project(self.database.connection(), project_id)
            .await
            .map_err(a4n_worker_db_adapter::map_database_error)?
            .into_iter()
            .map(list_item_from_record)
            .collect::<Result<Vec<_>, _>>()
            .map_err(app_error)
    }

    pub(super) async fn detail(
        &self,
        request: WorkerEnvelope<ImportReviewIdInput>,
    ) -> Result<ImportReviewSummary, WorkerFailure> {
        validate_request(&request, IMPORT_REVIEW_DETAIL_OPERATION)?;
        let user_id = request.principal.require_user_id()?;
        let review = review_for_editor(&self.database, user_id, request.payload.review_id).await?;
        summary_from_record(review).map_err(app_error)
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

fn list_item_from_record(
    review: a4n_domain::import_review::ImportReviewListRecord,
) -> Result<ImportReviewListItem, a4n_error::AppError> {
    let review_status = a4n_domain::import_review::ImportReviewStatus::parse(&review.review_status)
        .ok_or_else(|| a4n_error::AppError::InvalidState {
            current: format!(
                "导入审核 #{} review_status={:?}",
                review.id, review.review_status
            ),
            operation: "读取导入审核列表状态".to_string(),
        })?;
    Ok(ImportReviewListItem {
        id: review.id,
        project_id: review.project_id,
        filename: review.filename,
        review_status,
        created_at: review.created_at.to_string(),
        updated_at: review.updated_at.to_string(),
    })
}
