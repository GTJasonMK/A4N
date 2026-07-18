use super::super::types::CharacterAttributeEventHistoryFilter;
use super::validation::validate_attribute_history_filter;
use crate::character::CharacterDbError;
use a4n_core::models::character::attribute_event;
use sea_orm::{
    ColumnTrait, ConnectionTrait, DbErr, EntityTrait, QueryFilter, QueryOrder, QuerySelect,
};

pub async fn attribute_events_by_profile_chapter_range<C>(
    db: &C,
    profile_id: i64,
    start_chapter: i32,
    end_chapter: i32,
) -> Result<Vec<attribute_event::Model>, DbErr>
where
    C: ConnectionTrait,
{
    attribute_event::Entity::find()
        .filter(attribute_event::Column::ProfileId.eq(profile_id))
        .filter(attribute_event::Column::ChapterNumber.gte(start_chapter))
        .filter(attribute_event::Column::ChapterNumber.lte(end_chapter))
        .order_by_asc(attribute_event::Column::ChapterNumber)
        .order_by_asc(attribute_event::Column::Id)
        .all(db)
        .await
}

pub async fn attribute_events_by_ids<C>(
    db: &C,
    attribute_event_ids: &[i64],
) -> Result<Vec<attribute_event::Model>, DbErr>
where
    C: ConnectionTrait,
{
    if attribute_event_ids.is_empty() {
        return Ok(Vec::new());
    }

    attribute_event::Entity::find()
        .filter(attribute_event::Column::Id.is_in(attribute_event_ids.to_vec()))
        .order_by_asc(attribute_event::Column::Id)
        .all(db)
        .await
}

pub async fn attribute_events_by_profile<C>(
    db: &C,
    profile_id: i64,
    filter: CharacterAttributeEventHistoryFilter,
) -> Result<Vec<attribute_event::Model>, DbErr>
where
    C: ConnectionTrait,
{
    let mut query =
        attribute_event::Entity::find().filter(attribute_event::Column::ProfileId.eq(profile_id));

    if let Some(chapter_from) = filter.chapter_from {
        query = query.filter(attribute_event::Column::ChapterNumber.gte(chapter_from));
    }
    if let Some(chapter_to) = filter.chapter_to {
        query = query.filter(attribute_event::Column::ChapterNumber.lte(chapter_to));
    }
    if let Some(category) = filter.category {
        query = query.filter(attribute_event::Column::Category.eq(category));
    }
    if let Some(key) = filter.key {
        query = query.filter(attribute_event::Column::Key.eq(key));
    }
    if let Some(operation) = filter.operation {
        query = query.filter(attribute_event::Column::Operation.eq(operation));
    }
    if let Some(limit) = filter.limit {
        query = query.limit(limit);
    }

    query
        .order_by_asc(attribute_event::Column::ChapterNumber)
        .order_by_asc(attribute_event::Column::Id)
        .all(db)
        .await
}

pub async fn attribute_history<C>(
    db: &C,
    profile_id: i64,
    mut filter: CharacterAttributeEventHistoryFilter,
) -> Result<Vec<attribute_event::Model>, CharacterDbError>
where
    C: ConnectionTrait,
{
    validate_attribute_history_filter(&filter)?;
    if let Some(key) = filter.key.take() {
        filter.key = Some(key.trim().to_string());
    }
    attribute_events_by_profile(db, profile_id, filter)
        .await
        .map_err(CharacterDbError::Database)
}
