use super::chapter::upsert_imported_chapter_from_review;
use super::contracts::{
    AcceptedImportReview, ImportReviewMutationError, ensure_pending_import_review,
};
use a4n_db_models::models::project::import_review_queue;
use a4n_domain::import_review::ImportReviewStatus;
use chrono::Utc;
use sea_orm::{ActiveModelTrait, ConnectionTrait, EntityTrait, QuerySelect, Set, TransactionTrait};
use serde_json::Value;

pub async fn accept_import_review<C>(
    db: &C,
    review_id: i64,
    analysis_result: Option<Value>,
) -> Result<AcceptedImportReview, ImportReviewMutationError>
where
    C: ConnectionTrait + TransactionTrait,
{
    let tx = db.begin().await?;
    let mut review = import_review_queue::Entity::find_by_id(review_id)
        .lock_exclusive()
        .one(&tx)
        .await?
        .ok_or_else(|| ImportReviewMutationError::NotFound("导入审核不存在".to_string()))?;
    ensure_pending_import_review(&review, "接受导入审核")?;
    let status = if let Some(result) = analysis_result {
        review.analysis_result = result;
        ImportReviewStatus::Modified
    } else {
        ImportReviewStatus::Accepted
    };
    let updated_analysis_result = review.analysis_result.clone();
    let chapter = upsert_imported_chapter_from_review(&tx, &review).await?;
    let mut active: import_review_queue::ActiveModel = review.into();
    active.analysis_result = Set(updated_analysis_result);
    active.review_status = Set(status.as_str().to_string());
    active.updated_at = Set(Utc::now().naive_utc());
    let updated = active.update(&tx).await?;
    tx.commit().await?;
    Ok(AcceptedImportReview {
        review: updated,
        chapter_id: chapter.id,
    })
}
