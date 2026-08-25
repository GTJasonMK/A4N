use super::{CharacterPortraitCreateInput, CharacterPortraitRecordError};
use a4n_core::models::character::{character_portrait, protagonist_profile};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, EntityTrait, PaginatorTrait, QueryFilter,
    QuerySelect, Set, TransactionTrait,
};

pub async fn character_portrait_by_id<C>(
    db: &C,
    portrait_id: i64,
) -> Result<Option<character_portrait::Model>, sea_orm::DbErr>
where
    C: ConnectionTrait,
{
    character_portrait::Entity::find_by_id(portrait_id)
        .one(db)
        .await
}

pub async fn character_portrait_records_by_project<C>(
    db: &C,
    project_id: i64,
) -> Result<Vec<character_portrait::Model>, sea_orm::DbErr>
where
    C: ConnectionTrait,
{
    character_portrait::Entity::find()
        .filter(character_portrait::Column::ProjectId.eq(project_id))
        .all(db)
        .await
}

pub async fn primary_character_portrait_record<C>(
    db: &C,
    profile_id: i64,
) -> Result<Option<character_portrait::Model>, sea_orm::DbErr>
where
    C: ConnectionTrait,
{
    character_portrait::Entity::find()
        .filter(character_portrait::Column::ProfileId.eq(profile_id))
        .filter(character_portrait::Column::IsPrimary.eq(true))
        .one(db)
        .await
}

pub async fn create_character_portrait_for_profile<C>(
    db: &C,
    input: CharacterPortraitCreateInput,
) -> Result<character_portrait::Model, CharacterPortraitRecordError>
where
    C: ConnectionTrait,
{
    let profile_project_id = protagonist_profile::Entity::find_by_id(input.profile_id)
        .select_only()
        .column(protagonist_profile::Column::ProjectId)
        .into_tuple::<i64>()
        .one(db)
        .await?
        .ok_or_else(|| {
            CharacterPortraitRecordError::NotFound(format!("角色档案不存在: {}", input.profile_id))
        })?;
    if profile_project_id != input.project_id {
        return Err(CharacterPortraitRecordError::InvalidArgument(format!(
            "角色档案 {} 不属于项目 {}",
            input.profile_id, input.project_id
        )));
    }

    let existing_count = character_portrait::Entity::find()
        .filter(character_portrait::Column::ProfileId.eq(input.profile_id))
        .count(db)
        .await?;

    character_portrait::ActiveModel {
        profile_id: Set(input.profile_id),
        project_id: Set(input.project_id),
        portrait_type: Set(input.portrait_type),
        image_path: Set(input.image_path),
        prompt: Set(input.prompt),
        expression: Set(input.expression),
        pose: Set(input.pose),
        is_primary: Set(existing_count == 0),
        created_at: Set(chrono::Utc::now().naive_utc()),
        ..Default::default()
    }
    .insert(db)
    .await
    .map_err(CharacterPortraitRecordError::Database)
}

pub async fn set_primary_character_portrait_record<C>(
    db: &C,
    portrait_id: i64,
) -> Result<(), CharacterPortraitRecordError>
where
    C: ConnectionTrait + TransactionTrait,
{
    let txn = db.begin().await?;
    let profile_id = character_portrait::Entity::find_by_id(portrait_id)
        .select_only()
        .column(character_portrait::Column::ProfileId)
        .into_tuple::<i64>()
        .one(&txn)
        .await?
        .ok_or_else(|| CharacterPortraitRecordError::NotFound("立绘不存在".into()))?;

    character_portrait::Entity::update_many()
        .col_expr(
            character_portrait::Column::IsPrimary,
            sea_orm::sea_query::Expr::value(false),
        )
        .filter(character_portrait::Column::ProfileId.eq(profile_id))
        .filter(character_portrait::Column::IsPrimary.eq(true))
        .exec(&txn)
        .await?;

    let updated = character_portrait::Entity::update_many()
        .col_expr(
            character_portrait::Column::IsPrimary,
            sea_orm::sea_query::Expr::value(true),
        )
        .filter(character_portrait::Column::Id.eq(portrait_id))
        .exec(&txn)
        .await?;
    if updated.rows_affected != 1 {
        return Err(CharacterPortraitRecordError::NotFound("立绘不存在".into()));
    }
    txn.commit().await?;
    Ok(())
}
