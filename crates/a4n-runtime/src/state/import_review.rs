use a4n_core::{error::AppError, models::project::import_review_queue};
use a4n_orchestrator::{
    OrchestratorContext,
    chapter_content::ChapterDerivedCleanupHostPort,
    import_review::{AcceptedImportReview, ImportDedupResult, ModifyImportReviewInput},
};

pub(crate) struct ImportReviewRuntimeFacade<'a> {
    orchestrator: &'a OrchestratorContext,
}

impl<'a> ImportReviewRuntimeFacade<'a> {
    pub(super) fn new(orchestrator: &'a OrchestratorContext) -> Self {
        Self { orchestrator }
    }

    pub(crate) async fn import_reviews_by_project(
        &self,
        project_id: i64,
    ) -> Result<Vec<import_review_queue::Model>, AppError> {
        self.orchestrator
            .import_reviews_by_project(project_id)
            .await
    }

    pub(crate) async fn import_review_by_id(
        &self,
        review_id: i64,
    ) -> Result<Option<import_review_queue::Model>, AppError> {
        self.orchestrator.import_review_by_id(review_id).await
    }

    pub(crate) async fn update_import_review_status_validated(
        &self,
        review_id: i64,
        status: &str,
    ) -> Result<import_review_queue::Model, AppError> {
        self.orchestrator
            .update_import_review_status_validated(review_id, status)
            .await
    }

    pub(crate) async fn reject_import_review(
        &self,
        review_id: i64,
    ) -> Result<import_review_queue::Model, AppError> {
        self.orchestrator.reject_import_review(review_id).await
    }

    pub(crate) async fn delete_import_review(&self, review_id: i64) -> Result<(), AppError> {
        self.orchestrator.delete_import_review(review_id).await
    }

    pub(crate) async fn accept_import_review_for_user_validated(
        &self,
        user_id: i64,
        review_id: i64,
        host_port: &dyn ChapterDerivedCleanupHostPort,
    ) -> Result<AcceptedImportReview, AppError> {
        self.orchestrator
            .accept_import_review_for_user_validated(user_id, review_id, host_port)
            .await
    }

    pub(crate) async fn modify_import_review_for_user_validated(
        &self,
        user_id: i64,
        review_id: i64,
        input: ModifyImportReviewInput,
        host_port: &dyn ChapterDerivedCleanupHostPort,
    ) -> Result<AcceptedImportReview, AppError> {
        self.orchestrator
            .modify_import_review_for_user_validated(user_id, review_id, input, host_port)
            .await
    }

    pub(crate) async fn batch_accept_import_reviews_for_user_validated(
        &self,
        user_id: i64,
        review_ids: &[i64],
        host_port: &dyn ChapterDerivedCleanupHostPort,
    ) -> Result<Vec<AcceptedImportReview>, AppError> {
        self.orchestrator
            .batch_accept_import_reviews_for_user_validated(user_id, review_ids, host_port)
            .await
    }

    pub(crate) async fn check_import_dedup_for_user_validated(
        &self,
        user_id: i64,
        project_id: i64,
        content: &str,
    ) -> Result<ImportDedupResult, AppError> {
        self.orchestrator
            .check_import_dedup_for_user_validated(user_id, project_id, content)
            .await
    }
}
