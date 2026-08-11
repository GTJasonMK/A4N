use a4n_core::{
    error::AppError,
    models::{chapter::chapter, project::import_review_queue},
};
use a4n_db::DbHandle;
use serde_json::Value;

use super::OrchestratorContext;
use a4n_db::import_review::ImportReviewMutationError;

use crate::{
    chapter_content::ChapterDerivedCleanupHostPort,
    collaboration::ProjectAccessRole,
    import_review::{
        AcceptedImportReview, ImportDedupResult, ImportReviewCreateInput, calculate_import_dedup,
    },
};

#[derive(Clone)]
pub(super) struct ImportReviewApplication {
    database: DbHandle,
}

impl ImportReviewApplication {
    pub(super) fn new(database: DbHandle) -> Self {
        Self { database }
    }

    async fn import_reviews_by_project(
        &self,
        project_id: i64,
    ) -> Result<Vec<import_review_queue::Model>, AppError> {
        a4n_db::import_review::import_reviews_by_project(self.database.connection(), project_id)
            .await
            .map_err(AppError::Database)
    }

    async fn import_review_by_id(
        &self,
        review_id: i64,
    ) -> Result<Option<import_review_queue::Model>, AppError> {
        a4n_db::import_review::import_review_by_id(self.database.connection(), review_id)
            .await
            .map_err(AppError::Database)
    }

    async fn create_import_reviews(
        &self,
        inputs: Vec<ImportReviewCreateInput>,
    ) -> Result<Vec<import_review_queue::Model>, AppError> {
        a4n_db::import_review::create_import_reviews(self.database.connection(), inputs)
            .await
            .map_err(AppError::Database)
    }

    async fn delete_import_review(&self, review_id: i64) -> Result<(), AppError> {
        a4n_db::import_review::delete_import_review(self.database.connection(), review_id)
            .await
            .map_err(import_review_mutation_error)
    }

    /// 校验分析结果后接受导入审核并落库章节。
    async fn accept_import_review_validated(
        &self,
        review: import_review_queue::Model,
        analysis_result: Option<Value>,
    ) -> Result<AcceptedImportReview, AppError> {
        a4n_db::import_review::accept_import_review(
            self.database.connection(),
            review.id,
            analysis_result,
        )
        .await
        .map_err(import_review_mutation_error)
    }

    async fn reject_import_review(
        &self,
        review: import_review_queue::Model,
    ) -> Result<import_review_queue::Model, AppError> {
        a4n_db::import_review::reject_import_review(self.database.connection(), review.id)
            .await
            .map_err(import_review_mutation_error)
    }

    async fn import_dedup_chapters(
        &self,
        project_id: i64,
    ) -> Result<Vec<chapter::Model>, AppError> {
        a4n_db::import_review::import_dedup_chapters(self.database.connection(), project_id)
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

impl OrchestratorContext {
    pub async fn import_reviews_for_user_validated(
        &self,
        user_id: i64,
        project_id: i64,
    ) -> Result<Vec<import_review_queue::Model>, AppError> {
        self.verify_project_permission(project_id, user_id, ProjectAccessRole::Editor)
            .await?;
        self.import_review
            .import_reviews_by_project(project_id)
            .await
    }

    pub(crate) async fn create_import_reviews(
        &self,
        inputs: Vec<ImportReviewCreateInput>,
    ) -> Result<Vec<import_review_queue::Model>, AppError> {
        self.import_review.create_import_reviews(inputs).await
    }

    pub async fn reject_import_review_for_user_validated(
        &self,
        user_id: i64,
        review_id: i64,
    ) -> Result<import_review_queue::Model, AppError> {
        let review = self.import_review_for_editor(user_id, review_id).await?;
        self.import_review.reject_import_review(review).await
    }

    pub async fn delete_import_review_for_user_validated(
        &self,
        user_id: i64,
        review_id: i64,
    ) -> Result<(), AppError> {
        self.import_review_for_editor(user_id, review_id).await?;
        self.import_review.delete_import_review(review_id).await
    }

    /// 接受导入审核并完成章节派生数据后处理。
    pub async fn accept_import_review_for_user_validated(
        &self,
        user_id: i64,
        review_id: i64,
        analysis_result: Option<Value>,
        host_port: &dyn ChapterDerivedCleanupHostPort,
    ) -> Result<AcceptedImportReview, AppError> {
        let review = self.import_review_for_editor(user_id, review_id).await?;
        let accepted = self
            .import_review
            .accept_import_review_validated(review, analysis_result)
            .await?;
        self.finish_import_review_acceptance(&accepted, host_port)
            .await?;
        Ok(accepted)
    }

    pub async fn batch_accept_import_reviews_for_user_validated(
        &self,
        user_id: i64,
        review_ids: &[i64],
        host_port: &dyn ChapterDerivedCleanupHostPort,
    ) -> Result<Vec<AcceptedImportReview>, AppError> {
        let mut accepted = Vec::with_capacity(review_ids.len());
        for review_id in review_ids {
            match self
                .accept_import_review_for_user_validated(user_id, *review_id, None, host_port)
                .await
            {
                Ok(result) => accepted.push(result),
                Err(error) => {
                    return Err(AppError::InvalidState {
                        current: format!(
                            "accepted_review_ids={:?}",
                            accepted
                                .iter()
                                .map(|result| result.review.id)
                                .collect::<Vec<_>>()
                        ),
                        operation: format!(
                            "批量接受导入审核在 review_id={review_id} 失败：{error}"
                        ),
                    });
                }
            }
        }
        Ok(accepted)
    }

    pub async fn check_import_dedup_for_user_validated(
        &self,
        user_id: i64,
        project_id: i64,
        content: &str,
    ) -> Result<ImportDedupResult, AppError> {
        self.verify_project_permission(project_id, user_id, ProjectAccessRole::Editor)
            .await?;
        let chapters = self
            .import_review
            .import_dedup_chapters(project_id)
            .await?
            .into_iter()
            .map(|chapter| (chapter.id, chapter.content))
            .collect::<Vec<_>>();
        calculate_import_dedup(content, &chapters)
    }

    async fn import_review_for_editor(
        &self,
        user_id: i64,
        review_id: i64,
    ) -> Result<import_review_queue::Model, AppError> {
        let review = self
            .import_review
            .import_review_by_id(review_id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("导入审核不存在: review_id={review_id}")))?;
        self.verify_project_permission(review.project_id, user_id, ProjectAccessRole::Editor)
            .await?;
        Ok(review)
    }

    async fn finish_import_review_acceptance(
        &self,
        accepted: &AcceptedImportReview,
        host_port: &dyn ChapterDerivedCleanupHostPort,
    ) -> Result<(), AppError> {
        if let Err(error) = self
            .postprocess_chapter_derived_data_validated(host_port, accepted.chapter_id, true)
            .await
        {
            tracing::warn!(
                review_id = accepted.review.id,
                chapter_id = accepted.chapter_id,
                error = %error,
                "导入审核已接受，章节派生资料同步待恢复"
            );
        }
        Ok(())
    }
}
