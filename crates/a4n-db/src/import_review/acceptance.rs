use super::chapter::upsert_imported_chapter_from_review;
use super::contracts::{
    AcceptedImportReview, IMPORT_REVIEW_STATUS_ACCEPTED, IMPORT_REVIEW_STATUS_MODIFIED,
    ImportReviewMutationError, ModifyImportReviewInput,
};
use a4n_core::models::project::import_review_queue;
use chrono::Utc;
use sea_orm::{ActiveModelTrait, ConnectionTrait, Set, TransactionTrait};
use serde_json::json;

pub async fn accept_import_review<C>(
    db: &C,
    review: import_review_queue::Model,
) -> Result<AcceptedImportReview, ImportReviewMutationError>
where
    C: ConnectionTrait + TransactionTrait,
{
    let tx = db.begin().await?;
    let chapter = upsert_imported_chapter_from_review(&tx, &review).await?;
    let mut active: import_review_queue::ActiveModel = review.into();
    active.review_status = Set(IMPORT_REVIEW_STATUS_ACCEPTED.to_string());
    active.updated_at = Set(Utc::now().naive_utc());
    let updated = active.update(&tx).await?;
    tx.commit().await?;
    Ok(AcceptedImportReview {
        review: updated,
        chapter_id: chapter.id,
    })
}

pub async fn modify_import_review<C>(
    db: &C,
    review: import_review_queue::Model,
    input: ModifyImportReviewInput,
) -> Result<AcceptedImportReview, ImportReviewMutationError>
where
    C: ConnectionTrait + TransactionTrait,
{
    let review_id = review.id;
    let mut updated_result = input.analysis_result.clone().map_or_else(
        || {
            review.analysis_result.clone().ok_or_else(|| {
                ImportReviewMutationError::InvalidArgument(format!(
                    "导入审核缺少分析结果，无法修改并接受: review_id={review_id}"
                ))
            })
        },
        Ok,
    )?;
    let updated_object = updated_result.as_object_mut().ok_or_else(|| {
        ImportReviewMutationError::InvalidArgument(format!(
            "导入审核分析结果必须是 JSON object: review_id={review_id}"
        ))
    })?;
    if let Some(content) = &input.content {
        updated_object.insert("modified_content".to_string(), json!(content));
    }
    if let Some(summary) = &input.ai_summary {
        updated_object.insert("modified_summary".to_string(), json!(summary));
    }
    if let Some(chars) = &input.ai_characters {
        updated_object.insert("modified_characters".to_string(), chars.clone());
    }
    if let Some(world) = &input.ai_world_setting {
        updated_object.insert("modified_world_setting".to_string(), world.clone());
    }

    let tx = db.begin().await?;
    let mut active: import_review_queue::ActiveModel = review.clone().into();
    active.analysis_result = Set(Some(updated_result.clone()));
    active.review_status = Set(IMPORT_REVIEW_STATUS_MODIFIED.to_string());
    active.updated_at = Set(Utc::now().naive_utc());
    let updated = active.update(&tx).await?;
    let mut review_for_upsert = review;
    review_for_upsert.analysis_result = Some(updated_result);
    let chapter = upsert_imported_chapter_from_review(&tx, &review_for_upsert).await?;
    tx.commit().await?;
    Ok(AcceptedImportReview {
        review: updated,
        chapter_id: chapter.id,
    })
}
