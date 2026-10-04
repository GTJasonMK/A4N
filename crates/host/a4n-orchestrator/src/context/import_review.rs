use std::collections::HashMap;

use a4n_api_contract::chapter::ChapterDerivedCleanupHostPort;
use a4n_api_contract::import_review::{
    AcceptImportReviewInput, BatchAcceptImportReviewsInput, ImportReviewIdInput,
    ImportReviewSummary,
};
use a4n_db::{
    DbHandle,
    import_review::{AcceptedImportReview, ImportReviewMutationError},
};
use a4n_domain::import_review::{ImportChapterDraft, ImportReviewCreateInput, ImportReviewRecord};
use a4n_error::AppError;
use async_trait::async_trait;
use serde_json::Value;

use super::OrchestratorContext;
use a4n_domain::collaboration::ProjectAccessRole;

#[derive(Clone)]
pub(super) struct ImportReviewApplication {
    database: DbHandle,
}

impl ImportReviewApplication {
    pub(super) fn new(database: DbHandle) -> Self {
        Self { database }
    }

    async fn import_review_by_id(
        &self,
        review_id: i64,
    ) -> Result<Option<ImportReviewRecord>, AppError> {
        a4n_db::import_review::import_review_by_id(self.database.connection(), review_id)
            .await
            .map_err(AppError::Database)
    }

    async fn import_review_project_ids_by_ids(
        &self,
        review_ids: &[i64],
    ) -> Result<Vec<(i64, i64)>, AppError> {
        a4n_db::import_review::import_review_project_ids_by_ids(
            self.database.connection(),
            review_ids,
        )
        .await
        .map_err(AppError::Database)
    }

    async fn create_import_reviews(
        &self,
        inputs: Vec<ImportReviewCreateInput>,
    ) -> Result<Vec<ImportReviewRecord>, AppError> {
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
        review_id: i64,
        analysis_result: Option<Value>,
    ) -> Result<AcceptedImportReview, AppError> {
        a4n_db::import_review::accept_import_review(
            self.database.connection(),
            review_id,
            analysis_result,
        )
        .await
        .map_err(import_review_mutation_error)
    }

    async fn reject_import_review(
        &self,
        review: ImportReviewRecord,
    ) -> Result<ImportReviewRecord, AppError> {
        a4n_db::import_review::reject_import_review(self.database.connection(), review.id)
            .await
            .map_err(import_review_mutation_error)
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
    pub(crate) async fn create_import_reviews(
        &self,
        inputs: Vec<ImportReviewCreateInput>,
    ) -> Result<Vec<ImportReviewRecord>, AppError> {
        self.import_review.create_import_reviews(inputs).await
    }

    pub(crate) async fn reject_import_review_for_user_validated(
        &self,
        user_id: i64,
        review_id: i64,
    ) -> Result<ImportReviewRecord, AppError> {
        let review = self.import_review_for_editor(user_id, review_id).await?;
        self.import_review.reject_import_review(review).await
    }

    pub(crate) async fn delete_import_review_for_user_validated(
        &self,
        user_id: i64,
        review_id: i64,
    ) -> Result<(), AppError> {
        self.import_review_for_editor(user_id, review_id).await?;
        self.import_review.delete_import_review(review_id).await
    }

    /// 接受导入审核并完成章节派生数据后处理。
    pub(crate) async fn accept_import_review_for_user_validated(
        &self,
        user_id: i64,
        review_id: i64,
        analysis_result: Option<Value>,
        host_port: &dyn ChapterDerivedCleanupHostPort,
    ) -> Result<AcceptedImportReview, AppError> {
        let review = self.import_review_for_editor(user_id, review_id).await?;
        let accepted = self
            .import_review
            .accept_import_review_validated(review.id, analysis_result)
            .await?;
        if let Err(error) = self
            .postprocess_chapter_derived_data_validated(host_port, accepted.chapter_id)
            .await
        {
            tracing::warn!(
                review_id = accepted.review.id,
                chapter_id = accepted.chapter_id,
                error = %error,
                "导入审核已接受，章节派生资料同步待恢复"
            );
        }
        Ok(accepted)
    }

    pub(crate) async fn batch_accept_import_reviews_for_user_validated(
        &self,
        user_id: i64,
        review_ids: &[i64],
        host_port: &dyn ChapterDerivedCleanupHostPort,
    ) -> Result<Vec<AcceptedImportReview>, AppError> {
        if review_ids.is_empty() {
            return Ok(Vec::new());
        }
        let project_ids_by_review_id = self
            .import_review
            .import_review_project_ids_by_ids(review_ids)
            .await?
            .into_iter()
            .collect::<HashMap<_, _>>();
        let mut accepted = Vec::with_capacity(review_ids.len());
        for review_id in review_ids {
            let result = async {
                let project_id = project_ids_by_review_id
                    .get(review_id)
                    .copied()
                    .ok_or_else(|| {
                        AppError::NotFound(format!("导入审核不存在: review_id={review_id}"))
                    })?;
                self.verify_project_permission(project_id, user_id, ProjectAccessRole::Editor)
                    .await?;
                let accepted = self
                    .import_review
                    .accept_import_review_validated(*review_id, None)
                    .await?;
                if let Err(error) = self
                    .postprocess_chapter_derived_data_validated(host_port, accepted.chapter_id)
                    .await
                {
                    tracing::warn!(
                        review_id = accepted.review.id,
                        chapter_id = accepted.chapter_id,
                        error = %error,
                        "导入审核已接受，章节派生资料同步待恢复"
                    );
                }
                Ok::<_, AppError>(accepted)
            }
            .await;
            match result {
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

    async fn import_review_for_editor(
        &self,
        user_id: i64,
        review_id: i64,
    ) -> Result<ImportReviewRecord, AppError> {
        let review = self
            .import_review
            .import_review_by_id(review_id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("导入审核不存在: review_id={review_id}")))?;
        self.verify_project_permission(review.project_id, user_id, ProjectAccessRole::Editor)
            .await?;
        Ok(review)
    }
}

fn import_review_summary(review: &ImportReviewRecord) -> Result<ImportReviewSummary, AppError> {
    let review_status = a4n_domain::import_review::ImportReviewStatus::parse(&review.review_status)
        .ok_or_else(|| AppError::InvalidState {
            current: format!(
                "导入审核 #{} review_status={:?}",
                review.id, review.review_status
            ),
            operation: "读取导入审核状态".to_string(),
        })?;
    Ok(ImportReviewSummary {
        id: review.id,
        project_id: review.project_id,
        filename: review.filename.clone(),
        analysis_result: review.analysis_result.clone(),
        review_status,
        created_at: review.created_at.to_string(),
        updated_at: review.updated_at.to_string(),
    })
}

#[async_trait]
impl a4n_novel_host_contract::ImportReviewIngestionApplicationPort for OrchestratorContext {
    async fn create_from_parsed(
        &self,
        user_id: i64,
        project_id: i64,
        filename: &str,
        chapters: &[ImportChapterDraft],
    ) -> Result<Vec<ImportReviewSummary>, AppError> {
        let reviews = crate::import_analysis::create_import_analysis_reviews_for_user_validated(
            self, user_id, project_id, filename, chapters,
        )
        .await?;
        reviews.iter().map(import_review_summary).collect()
    }
}

#[async_trait]
impl a4n_api_contract::import_review::ImportReviewAcceptanceApplicationPort
    for OrchestratorContext
{
    async fn accept(
        &self,
        user_id: i64,
        input: AcceptImportReviewInput,
        host_port: &dyn ChapterDerivedCleanupHostPort,
    ) -> Result<ImportReviewSummary, AppError> {
        let accepted = self
            .accept_import_review_for_user_validated(
                user_id,
                input.review_id,
                input.analysis_result,
                host_port,
            )
            .await?;
        import_review_summary(&accepted.review)
    }

    async fn batch_accept(
        &self,
        user_id: i64,
        input: BatchAcceptImportReviewsInput,
        host_port: &dyn ChapterDerivedCleanupHostPort,
    ) -> Result<Vec<ImportReviewSummary>, AppError> {
        let accepted = self
            .batch_accept_import_reviews_for_user_validated(user_id, &input.review_ids, host_port)
            .await?;
        accepted
            .iter()
            .map(|accepted| import_review_summary(&accepted.review))
            .collect()
    }
}

#[async_trait]
impl a4n_api_contract::import_review::ImportReviewMutationApplicationPort for OrchestratorContext {
    async fn delete(&self, user_id: i64, input: ImportReviewIdInput) -> Result<(), AppError> {
        self.delete_import_review_for_user_validated(user_id, input.review_id)
            .await
    }

    async fn reject(
        &self,
        user_id: i64,
        input: ImportReviewIdInput,
    ) -> Result<ImportReviewSummary, AppError> {
        let updated = self
            .reject_import_review_for_user_validated(user_id, input.review_id)
            .await?;
        import_review_summary(&updated)
    }
}
