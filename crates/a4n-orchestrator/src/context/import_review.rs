use a4n_core::{
    error::AppError,
    models::{chapter::chapter, project::import_review_queue},
};
use a4n_db::DbHandle;

use super::OrchestratorContext;
use a4n_db::import_review::ImportReviewMutationError;

use crate::{
    chapter_content::ChapterDerivedCleanupHostPort,
    collaboration::ProjectAccessRole,
    import_review::{
        AcceptedImportReview, ImportDedupResult, ImportReviewCreateInput, ModifyImportReviewInput,
        calculate_import_dedup, ensure_import_review_ready_for_accept,
        normalize_import_review_status, normalize_modify_import_review_input,
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

    async fn delete_pending_import_reviews_by_project(
        &self,
        project_id: i64,
    ) -> Result<u64, AppError> {
        a4n_db::import_review::delete_pending_import_reviews_by_project(
            self.database.connection(),
            project_id,
        )
        .await
        .map_err(AppError::Database)
    }

    async fn update_import_review_status(
        &self,
        review_id: i64,
        status: &str,
    ) -> Result<import_review_queue::Model, AppError> {
        a4n_db::import_review::update_import_review_status(
            self.database.connection(),
            review_id,
            status,
        )
        .await
        .map_err(import_review_mutation_error)
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
    ) -> Result<AcceptedImportReview, AppError> {
        ensure_import_review_ready_for_accept(&review)?;
        a4n_db::import_review::accept_import_review(self.database.connection(), review)
            .await
            .map_err(import_review_mutation_error)
    }

    /// 规范化修改输入后修改分析结果并接受。
    async fn modify_import_review_validated(
        &self,
        review: import_review_queue::Model,
        input: ModifyImportReviewInput,
    ) -> Result<AcceptedImportReview, AppError> {
        let input = normalize_modify_import_review_input(input);
        a4n_db::import_review::modify_import_review(self.database.connection(), review, input)
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
    pub async fn import_reviews_by_project(
        &self,
        project_id: i64,
    ) -> Result<Vec<import_review_queue::Model>, AppError> {
        self.import_review
            .import_reviews_by_project(project_id)
            .await
    }

    pub async fn import_review_by_id(
        &self,
        review_id: i64,
    ) -> Result<Option<import_review_queue::Model>, AppError> {
        self.import_review.import_review_by_id(review_id).await
    }

    pub(crate) async fn create_import_reviews(
        &self,
        inputs: Vec<ImportReviewCreateInput>,
    ) -> Result<Vec<import_review_queue::Model>, AppError> {
        self.import_review.create_import_reviews(inputs).await
    }

    pub(crate) async fn delete_pending_import_reviews_by_project(
        &self,
        project_id: i64,
    ) -> Result<u64, AppError> {
        self.import_review
            .delete_pending_import_reviews_by_project(project_id)
            .await
    }

    /// 规范化状态后更新导入审核状态。
    pub async fn update_import_review_status_validated(
        &self,
        review_id: i64,
        status: &str,
    ) -> Result<import_review_queue::Model, AppError> {
        let status =
            normalize_import_review_status(status).map_err(import_review_mutation_error)?;
        self.import_review
            .update_import_review_status(review_id, status)
            .await
    }

    /// 拒绝导入审核（经状态规范化写路径）。
    pub async fn reject_import_review(
        &self,
        review_id: i64,
    ) -> Result<import_review_queue::Model, AppError> {
        self.update_import_review_status_validated(review_id, "rejected")
            .await
    }

    pub async fn delete_import_review(&self, review_id: i64) -> Result<(), AppError> {
        self.import_review.delete_import_review(review_id).await
    }

    /// 接受导入审核并完成章节派生数据后处理。
    pub async fn accept_import_review_for_user_validated(
        &self,
        user_id: i64,
        review_id: i64,
        host_port: &dyn ChapterDerivedCleanupHostPort,
    ) -> Result<AcceptedImportReview, AppError> {
        let review = self.import_review_for_editor(user_id, review_id).await?;
        let accepted = self
            .import_review
            .accept_import_review_validated(review)
            .await?;
        self.finish_import_review_acceptance(&accepted, host_port)
            .await?;
        Ok(accepted)
    }

    /// 修改并接受导入审核，并完成章节派生数据后处理。
    pub async fn modify_import_review_for_user_validated(
        &self,
        user_id: i64,
        review_id: i64,
        input: ModifyImportReviewInput,
        host_port: &dyn ChapterDerivedCleanupHostPort,
    ) -> Result<AcceptedImportReview, AppError> {
        let review = self.import_review_for_editor(user_id, review_id).await?;
        let accepted = self
            .import_review
            .modify_import_review_validated(review, input)
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
                .accept_import_review_for_user_validated(user_id, *review_id, host_port)
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
        self.postprocess_chapter_after_content_change_validated(host_port, accepted.chapter_id)
            .await
            .map_err(|error| AppError::InvalidState {
                current: format!(
                    "review_id={} status={} chapter_id={}",
                    accepted.review.id, accepted.review.review_status, accepted.chapter_id
                ),
                operation: format!("导入审核已落库，但章节后处理失败：{error}"),
            })
    }
}
