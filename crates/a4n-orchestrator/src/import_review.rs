use crate::OrchestratorContext;
use a4n_core::{
    error::AppError,
    models::{chapter::chapter, project::import_review_queue},
};
pub use a4n_db::import_review::{
    AcceptedImportReview, ImportReviewCreateInput, ImportReviewMutationError,
    ModifyImportReviewInput, normalize_import_review_status,
};

impl OrchestratorContext {
    pub async fn import_reviews_by_project(
        &self,
        project_id: i64,
    ) -> Result<Vec<import_review_queue::Model>, AppError> {
        a4n_db::import_review::import_reviews_by_project(self.db.connection(), project_id)
            .await
            .map_err(AppError::Database)
    }

    pub async fn import_review_by_id(
        &self,
        review_id: i64,
    ) -> Result<Option<import_review_queue::Model>, AppError> {
        a4n_db::import_review::import_review_by_id(self.db.connection(), review_id)
            .await
            .map_err(AppError::Database)
    }

    pub async fn create_import_reviews(
        &self,
        inputs: Vec<ImportReviewCreateInput>,
    ) -> Result<Vec<import_review_queue::Model>, AppError> {
        a4n_db::import_review::create_import_reviews(self.db.connection(), inputs)
            .await
            .map_err(AppError::Database)
    }

    pub async fn delete_pending_import_reviews_by_project(
        &self,
        project_id: i64,
    ) -> Result<u64, AppError> {
        a4n_db::import_review::delete_pending_import_reviews_by_project(
            self.db.connection(),
            project_id,
        )
        .await
        .map_err(AppError::Database)
    }

    pub async fn update_import_review_status(
        &self,
        review_id: i64,
        status: &str,
    ) -> Result<import_review_queue::Model, AppError> {
        a4n_db::import_review::update_import_review_status(self.db.connection(), review_id, status)
            .await
            .map_err(import_review_mutation_error)
    }

    pub async fn delete_import_review(&self, review_id: i64) -> Result<(), AppError> {
        a4n_db::import_review::delete_import_review(self.db.connection(), review_id)
            .await
            .map_err(import_review_mutation_error)
    }

    pub async fn accept_import_review(
        &self,
        review: import_review_queue::Model,
    ) -> Result<AcceptedImportReview, AppError> {
        a4n_db::import_review::accept_import_review(self.db.connection(), review)
            .await
            .map_err(import_review_mutation_error)
    }

    pub async fn modify_import_review(
        &self,
        review: import_review_queue::Model,
        input: ModifyImportReviewInput,
    ) -> Result<AcceptedImportReview, AppError> {
        a4n_db::import_review::modify_import_review(self.db.connection(), review, input)
            .await
            .map_err(import_review_mutation_error)
    }

    pub async fn import_dedup_chapters(
        &self,
        project_id: i64,
    ) -> Result<Vec<chapter::Model>, AppError> {
        a4n_db::import_review::import_dedup_chapters(self.db.connection(), project_id)
            .await
            .map_err(AppError::Database)
    }
}

fn import_review_mutation_error(error: ImportReviewMutationError) -> AppError {
    match error {
        ImportReviewMutationError::Database(error) => AppError::Database(error),
        ImportReviewMutationError::NotFound(message) => AppError::NotFound(message),
        ImportReviewMutationError::InvalidArgument(message) => AppError::InvalidArgument(message),
        ImportReviewMutationError::InvalidState { current, operation } => {
            AppError::InvalidState { current, operation }
        }
    }
}
