use super::contracts::{ImportReviewMutationError, ensure_pending_import_review};
use a4n_db_models::models::{chapter::chapter, project::import_review_queue};
use a4n_domain::import_review::{ImportReviewCreateInput, ImportReviewStatus};
use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, DbErr, EntityTrait, QueryFilter, QueryOrder,
    QuerySelect, Set, TransactionTrait,
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

pub async fn import_review_project_ids_by_ids<C>(
    db: &C,
    review_ids: &[i64],
) -> Result<Vec<(i64, i64)>, DbErr>
where
    C: ConnectionTrait,
{
    if review_ids.is_empty() {
        return Ok(Vec::new());
    }
    import_review_queue::Entity::find()
        .select_only()
        .column(import_review_queue::Column::Id)
        .column(import_review_queue::Column::ProjectId)
        .filter(import_review_queue::Column::Id.is_in(review_ids.iter().copied()))
        .into_tuple::<(i64, i64)>()
        .all(db)
        .await
}

pub async fn create_import_reviews<C>(
    db: &C,
    inputs: Vec<ImportReviewCreateInput>,
) -> Result<Vec<import_review_queue::Model>, DbErr>
where
    C: ConnectionTrait + TransactionTrait,
{
    if inputs.is_empty() {
        return Ok(Vec::new());
    }
    let tx = db.begin().await?;
    let now = Utc::now().naive_utc();
    let rows = inputs
        .into_iter()
        .map(|input| import_review_queue::ActiveModel {
            project_id: Set(input.project_id),
            filename: Set(input.filename),
            analysis_result: Set(input.analysis_result),
            review_status: Set(ImportReviewStatus::Pending.as_str().to_string()),
            created_at: Set(now),
            updated_at: Set(now),
            ..Default::default()
        })
        .collect::<Vec<_>>();
    let reviews = import_review_queue::Entity::insert_many(rows)
        .exec_with_returning_many(&tx)
        .await?;
    tx.commit().await?;
    Ok(reviews)
}

pub async fn reject_import_review<C>(
    db: &C,
    review_id: i64,
) -> Result<import_review_queue::Model, ImportReviewMutationError>
where
    C: ConnectionTrait + TransactionTrait,
{
    let tx = db.begin().await?;
    let review = import_review_queue::Entity::find_by_id(review_id)
        .lock_exclusive()
        .one(&tx)
        .await?
        .ok_or_else(|| ImportReviewMutationError::NotFound("导入审核不存在".to_string()))?;
    ensure_pending_import_review(&review, "拒绝导入审核")?;
    let mut active: import_review_queue::ActiveModel = review.into();
    active.review_status = Set(ImportReviewStatus::Rejected.as_str().to_string());
    active.updated_at = Set(Utc::now().naive_utc());
    let updated = active.update(&tx).await?;
    tx.commit().await?;
    Ok(updated)
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

pub async fn import_dedup_chapters<C>(db: &C, project_id: i64) -> Result<Vec<(i64, String)>, DbErr>
where
    C: ConnectionTrait,
{
    chapter::Entity::find()
        .select_only()
        .column(chapter::Column::Id)
        .column(chapter::Column::Content)
        .filter(chapter::Column::ProjectId.eq(project_id))
        .filter(chapter::Column::Status.ne("deleted"))
        .into_tuple::<(i64, String)>()
        .all(db)
        .await
}
