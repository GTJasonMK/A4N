use super::contracts::CharacterConflictContextRows;
use crate::character::CharacterDbError;
use a4n_core::models::{
    chapter::chapter,
    character::{attribute_event, character_state_index},
};
use sea_orm::{ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, QueryOrder};

pub async fn character_conflict_context_rows<C>(
    db: &C,
    project_id: i64,
    profile_id: i64,
    start_chapter: Option<i32>,
    end_chapter: Option<i32>,
) -> Result<CharacterConflictContextRows, CharacterDbError>
where
    C: ConnectionTrait,
{
    let mut chapters_query = chapter::Entity::find()
        .filter(chapter::Column::ProjectId.eq(project_id))
        .filter(chapter::Column::Status.ne("deleted"))
        .order_by_asc(chapter::Column::ChapterNumber);
    if let Some(start_chapter) = start_chapter {
        chapters_query = chapters_query.filter(chapter::Column::ChapterNumber.gte(start_chapter));
    }
    if let Some(end_chapter) = end_chapter {
        chapters_query = chapters_query.filter(chapter::Column::ChapterNumber.lte(end_chapter));
    }
    let chapters = chapters_query
        .all(db)
        .await
        .map_err(CharacterDbError::Database)?;

    let mut state_timeline_query = character_state_index::Entity::find()
        .filter(character_state_index::Column::ProfileId.eq(profile_id))
        .order_by_asc(character_state_index::Column::ChapterNumber);
    if let Some(start_chapter) = start_chapter {
        state_timeline_query = state_timeline_query
            .filter(character_state_index::Column::ChapterNumber.gte(start_chapter));
    }
    if let Some(end_chapter) = end_chapter {
        state_timeline_query = state_timeline_query
            .filter(character_state_index::Column::ChapterNumber.lte(end_chapter));
    }
    let state_timeline = state_timeline_query
        .all(db)
        .await
        .map_err(CharacterDbError::Database)?;

    let mut attribute_events_query = attribute_event::Entity::find()
        .filter(attribute_event::Column::ProfileId.eq(profile_id))
        .order_by_asc(attribute_event::Column::ChapterNumber)
        .order_by_asc(attribute_event::Column::Timestamp);
    if let Some(start_chapter) = start_chapter {
        attribute_events_query = attribute_events_query
            .filter(attribute_event::Column::ChapterNumber.gte(start_chapter));
    }
    if let Some(end_chapter) = end_chapter {
        attribute_events_query =
            attribute_events_query.filter(attribute_event::Column::ChapterNumber.lte(end_chapter));
    }
    let attribute_events = attribute_events_query
        .all(db)
        .await
        .map_err(CharacterDbError::Database)?;

    Ok(CharacterConflictContextRows {
        chapters,
        state_timeline,
        attribute_events,
    })
}
