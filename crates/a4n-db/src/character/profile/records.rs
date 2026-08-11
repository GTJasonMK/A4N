use a4n_core::models::character::protagonist_profile;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, DbBackend, DbErr, EntityTrait, QueryFilter,
    QueryOrder, QuerySelect, Set,
};

use crate::character::CharacterDbError;

use super::types::CharacterProfileCurrentStateUpdate;

pub(in crate::character) async fn require_profile<C>(
    db: &C,
    profile_id: i64,
) -> Result<protagonist_profile::Model, CharacterDbError>
where
    C: ConnectionTrait,
{
    profile_by_id(db, profile_id)
        .await?
        .ok_or_else(|| CharacterDbError::NotFound(format!("角色档案不存在: {profile_id}")))
}

pub(in crate::character) async fn require_profile_for_update<C>(
    db: &C,
    profile_id: i64,
) -> Result<protagonist_profile::Model, CharacterDbError>
where
    C: ConnectionTrait,
{
    let mut query = protagonist_profile::Entity::find_by_id(profile_id);
    if db.get_database_backend() != DbBackend::Sqlite {
        query = query.lock_exclusive();
    }
    query
        .one(db)
        .await?
        .ok_or_else(|| CharacterDbError::NotFound(format!("角色档案不存在: {profile_id}")))
}

pub async fn profiles_by_project<C>(
    db: &C,
    project_id: i64,
) -> Result<Vec<protagonist_profile::Model>, DbErr>
where
    C: ConnectionTrait,
{
    protagonist_profile::Entity::find()
        .filter(protagonist_profile::Column::ProjectId.eq(project_id))
        .order_by_asc(protagonist_profile::Column::Id)
        .all(db)
        .await
}

pub async fn profile_by_id<C>(
    db: &C,
    profile_id: i64,
) -> Result<Option<protagonist_profile::Model>, DbErr>
where
    C: ConnectionTrait,
{
    protagonist_profile::Entity::find_by_id(profile_id)
        .one(db)
        .await
}

pub async fn profile_by_project_name<C>(
    db: &C,
    project_id: i64,
    name: &str,
) -> Result<Option<protagonist_profile::Model>, DbErr>
where
    C: ConnectionTrait,
{
    protagonist_profile::Entity::find()
        .filter(protagonist_profile::Column::ProjectId.eq(project_id))
        .filter(protagonist_profile::Column::Name.eq(name))
        .one(db)
        .await
}

pub async fn update_profile_current_state<C>(
    db: &C,
    profile: protagonist_profile::Model,
    input: CharacterProfileCurrentStateUpdate,
) -> Result<protagonist_profile::Model, CharacterDbError>
where
    C: ConnectionTrait,
{
    let mut active: protagonist_profile::ActiveModel = profile.into();
    active.gender = Set(input.gender);
    active.age = Set(input.age);
    active.appearance = Set(input.appearance);
    active.background = Set(input.background);
    active.personality = Set(input.personality);
    active.current_state = Set(input.current_state);
    if let Some(last_synced_chapter) = input.last_synced_chapter {
        active.last_synced_chapter = Set(last_synced_chapter);
    }
    active.updated_at = Set(chrono::Utc::now().naive_utc());
    active.update(db).await.map_err(CharacterDbError::Database)
}
