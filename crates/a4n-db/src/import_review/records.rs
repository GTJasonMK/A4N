use super::contracts::{
    IMPORT_REVIEW_STATUS_PENDING, ImportReviewCreateInput, ImportReviewMutationError,
    normalize_import_review_status,
};
use a4n_core::models::{chapter::chapter, project::import_review_queue};
use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, DbErr, EntityTrait, QueryFilter, QueryOrder,
    Set,
};

pub async fn import_reviews_by_project<C>(
    db: &C,
    project_id: i64,
) -> Result<Vec<import_review_queue::Model>, DbErr>
where
    C: ConnectionTrait,
{
    import_review_queue::Entity::find()
        .filter(import_review_queue::Column::ProjectId.eq(project_id))
        .order_by_desc(import_review_queue::Column::CreatedAt)
        .all(db)
        .await
}

pub async fn import_review_by_id<C>(
    db: &C,
    review_id: i64,
) -> Result<Option<import_review_queue::Model>, DbErr>
where
    C: ConnectionTrait,
{
    import_review_queue::Entity::find_by_id(review_id)
        .one(db)
        .await
}

pub async fn create_import_reviews<C>(
    db: &C,
    inputs: Vec<ImportReviewCreateInput>,
) -> Result<Vec<import_review_queue::Model>, DbErr>
where
    C: ConnectionTrait,
{
    let now = Utc::now().naive_utc();
    let mut reviews = Vec::with_capacity(inputs.len());
    for input in inputs {
        reviews.push(
            import_review_queue::ActiveModel {
                project_id: Set(input.project_id),
                filename: Set(input.filename),
                file_type: Set(input.file_type),
                detected_encoding: Set(input.detected_encoding),
                parsed_chapters: Set(input.parsed_chapters),
                dedup_paragraphs: Set(input.dedup_paragraphs),
                dedup_rate: Set(input.dedup_rate),
                analysis_result: Set(Some(input.analysis_result)),
                review_status: Set(IMPORT_REVIEW_STATUS_PENDING.to_string()),
                created_at: Set(now),
                updated_at: Set(now),
                ..Default::default()
            }
            .insert(db)
            .await?,
        );
    }
    Ok(reviews)
}

pub async fn delete_pending_import_reviews_by_project<C>(
    db: &C,
    project_id: i64,
) -> Result<u64, DbErr>
where
    C: ConnectionTrait,
{
    let result = import_review_queue::Entity::delete_many()
        .filter(import_review_queue::Column::ProjectId.eq(project_id))
        .filter(import_review_queue::Column::ReviewStatus.eq(IMPORT_REVIEW_STATUS_PENDING))
        .exec(db)
        .await?;
    Ok(result.rows_affected)
}

pub async fn update_import_review_status<C>(
    db: &C,
    review_id: i64,
    status: &str,
) -> Result<import_review_queue::Model, ImportReviewMutationError>
where
    C: ConnectionTrait,
{
    let status = normalize_import_review_status(status)?;
    let review = import_review_queue::Entity::find_by_id(review_id)
        .one(db)
        .await?
        .ok_or_else(|| ImportReviewMutationError::NotFound("导入记录不存在".into()))?;
    let mut active: import_review_queue::ActiveModel = review.into();
    active.review_status = Set(status.to_string());
    active.updated_at = Set(Utc::now().naive_utc());
    active.update(db).await.map_err(Into::into)
}

pub async fn delete_import_review<C>(
    db: &C,
    review_id: i64,
) -> Result<(), ImportReviewMutationError>
where
    C: ConnectionTrait,
{
    import_review_queue::Entity::delete_by_id(review_id)
        .exec(db)
        .await?;
    Ok(())
}

pub async fn import_dedup_chapters<C>(db: &C, project_id: i64) -> Result<Vec<chapter::Model>, DbErr>
where
    C: ConnectionTrait,
{
    chapter::Entity::find()
        .filter(chapter::Column::ProjectId.eq(project_id))
        .filter(chapter::Column::Status.ne("deleted"))
        .all(db)
        .await
}
