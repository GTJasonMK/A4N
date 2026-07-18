use super::contracts::ProjectChapterContextError;
use a4n_core::models::{
    asset::object_profile,
    character::{character_state_index, protagonist_profile, snapshot_reference},
    scene::scene_profile,
};
use sea_orm::{
    ColumnTrait, ConnectionTrait, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder,
    QuerySelect,
};

pub async fn object_profiles_before_chapter(
    db: &DatabaseConnection,
    project_id: i64,
    chapter_number: i32,
) -> Result<Vec<object_profile::Model>, ProjectChapterContextError> {
    object_profile::Entity::find()
        .filter(object_profile::Column::ProjectId.eq(project_id))
        .filter(object_profile::Column::FirstSeenChapter.lt(chapter_number))
        .order_by_desc(object_profile::Column::LastSeenChapter)
        .order_by_asc(object_profile::Column::Id)
        .all(db)
        .await
        .map_err(ProjectChapterContextError::Database)
}

pub async fn scene_profiles_before_chapter(
    db: &DatabaseConnection,
    project_id: i64,
    chapter_number: i32,
) -> Result<Vec<scene_profile::Model>, ProjectChapterContextError> {
    scene_profile::Entity::find()
        .filter(scene_profile::Column::ProjectId.eq(project_id))
        .filter(scene_profile::Column::FirstSeenChapter.lt(chapter_number))
        .order_by_desc(scene_profile::Column::LastSeenChapter)
        .order_by_asc(scene_profile::Column::Id)
        .all(db)
        .await
        .map_err(ProjectChapterContextError::Database)
}

pub async fn protagonist_profiles_for_project(
    db: &DatabaseConnection,
    project_id: i64,
) -> Result<Vec<protagonist_profile::Model>, ProjectChapterContextError> {
    protagonist_profiles_for_project_in(db, project_id).await
}

pub async fn protagonist_profiles_for_project_in<C>(
    db: &C,
    project_id: i64,
) -> Result<Vec<protagonist_profile::Model>, ProjectChapterContextError>
where
    C: ConnectionTrait,
{
    protagonist_profile::Entity::find()
        .filter(protagonist_profile::Column::ProjectId.eq(project_id))
        .order_by_asc(protagonist_profile::Column::Id)
        .all(db)
        .await
        .map_err(ProjectChapterContextError::Database)
}

pub async fn character_states_for_profiles_before_chapter(
    db: &DatabaseConnection,
    profile_ids: &[i64],
    chapter_number: i32,
    limit: u64,
) -> Result<Vec<character_state_index::Model>, ProjectChapterContextError> {
    if profile_ids.is_empty() {
        return Ok(Vec::new());
    }
    character_state_index::Entity::find()
        .filter(character_state_index::Column::ProfileId.is_in(profile_ids.iter().copied()))
        .filter(character_state_index::Column::ChapterNumber.lt(chapter_number))
        .order_by_desc(character_state_index::Column::ChapterNumber)
        .limit(limit)
        .all(db)
        .await
        .map_err(ProjectChapterContextError::Database)
}

pub async fn snapshot_profile_ids_for_chapter(
    db: &DatabaseConnection,
    profile_ids: &[i64],
    chapter_number: i32,
) -> Result<Vec<i64>, ProjectChapterContextError> {
    if profile_ids.is_empty() {
        return Ok(Vec::new());
    }
    snapshot_reference::Entity::find()
        .select_only()
        .column(snapshot_reference::Column::ProfileId)
        .filter(snapshot_reference::Column::ProfileId.is_in(profile_ids.iter().copied()))
        .filter(snapshot_reference::Column::ChapterNumber.eq(chapter_number))
        .into_tuple::<i64>()
        .all(db)
        .await
        .map_err(ProjectChapterContextError::Database)
}

pub async fn snapshot_reference_for_profile_chapter<C>(
    db: &C,
    profile_id: i64,
    chapter_number: i32,
) -> Result<Option<snapshot_reference::Model>, ProjectChapterContextError>
where
    C: ConnectionTrait,
{
    snapshot_reference::Entity::find()
        .filter(snapshot_reference::Column::ProfileId.eq(profile_id))
        .filter(snapshot_reference::Column::ChapterNumber.eq(chapter_number))
        .one(db)
        .await
        .map_err(ProjectChapterContextError::Database)
}
