use super::contracts::CharacterTimelineDeletionSummary;
use crate::character::{CharacterDbError, validate_chapter_number};
use a4n_core::models::character::{
    attribute_event::{self, SOURCE_KIND_AI_SYNC},
    character_state_index, protagonist_behavior_record, snapshot_reference,
};
use sea_orm::{ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, QuerySelect};

async fn delete_timeline_artifacts_from_chapter<C>(
    db: &C,
    profile_id: i64,
    start_chapter: i32,
    end_chapter: Option<i32>,
) -> Result<CharacterTimelineDeletionSummary, CharacterDbError>
where
    C: ConnectionTrait,
{
    delete_timeline_artifacts_from_chapter_with_source(
        db,
        profile_id,
        start_chapter,
        end_chapter,
        None,
    )
    .await
}

pub(in crate::character) async fn delete_ai_timeline_artifacts_from_chapter_checked<C>(
    db: &C,
    profile_id: i64,
    start_chapter: i32,
    end_chapter: Option<i32>,
) -> Result<CharacterTimelineDeletionSummary, CharacterDbError>
where
    C: ConnectionTrait,
{
    validate_timeline_cleanup_range(start_chapter, end_chapter)?;
    delete_timeline_artifacts_from_chapter_with_source(
        db,
        profile_id,
        start_chapter,
        end_chapter,
        Some(SOURCE_KIND_AI_SYNC),
    )
    .await
}

async fn delete_timeline_artifacts_from_chapter_with_source<C>(
    db: &C,
    profile_id: i64,
    start_chapter: i32,
    end_chapter: Option<i32>,
    source_kind: Option<&str>,
) -> Result<CharacterTimelineDeletionSummary, CharacterDbError>
where
    C: ConnectionTrait,
{
    let mut attribute_event_ids_query = attribute_event::Entity::find()
        .select_only()
        .column(attribute_event::Column::Id)
        .filter(attribute_event::Column::ProfileId.eq(profile_id))
        .filter(attribute_event::Column::ChapterNumber.gte(start_chapter));
    if let Some(source_kind) = source_kind {
        attribute_event_ids_query =
            attribute_event_ids_query.filter(attribute_event::Column::SourceKind.eq(source_kind));
    }
    if let Some(end_chapter) = end_chapter {
        attribute_event_ids_query = attribute_event_ids_query
            .filter(attribute_event::Column::ChapterNumber.lte(end_chapter));
    }
    let attribute_event_ids = attribute_event_ids_query
        .into_tuple::<i64>()
        .all(db)
        .await?;

    let mut state_index_ids_query = character_state_index::Entity::find()
        .select_only()
        .column(character_state_index::Column::Id)
        .filter(character_state_index::Column::ProfileId.eq(profile_id))
        .filter(character_state_index::Column::ChapterNumber.gte(start_chapter));
    if let Some(end_chapter) = end_chapter {
        state_index_ids_query = state_index_ids_query
            .filter(character_state_index::Column::ChapterNumber.lte(end_chapter));
    }
    let state_index_ids = state_index_ids_query.into_tuple::<i64>().all(db).await?;

    let mut attribute_event_delete = attribute_event::Entity::delete_many()
        .filter(attribute_event::Column::ProfileId.eq(profile_id))
        .filter(attribute_event::Column::ChapterNumber.gte(start_chapter));
    if let Some(source_kind) = source_kind {
        attribute_event_delete =
            attribute_event_delete.filter(attribute_event::Column::SourceKind.eq(source_kind));
    }
    if let Some(end_chapter) = end_chapter {
        attribute_event_delete =
            attribute_event_delete.filter(attribute_event::Column::ChapterNumber.lte(end_chapter));
    }
    let deleted_attribute_events = attribute_event_delete.exec(db).await?.rows_affected;

    let mut behavior_delete = protagonist_behavior_record::Entity::delete_many()
        .filter(protagonist_behavior_record::Column::ProfileId.eq(profile_id))
        .filter(protagonist_behavior_record::Column::ChapterNumber.gte(start_chapter));
    if let Some(source_kind) = source_kind {
        behavior_delete =
            behavior_delete.filter(protagonist_behavior_record::Column::SourceKind.eq(source_kind));
    }
    if let Some(end_chapter) = end_chapter {
        behavior_delete = behavior_delete
            .filter(protagonist_behavior_record::Column::ChapterNumber.lte(end_chapter));
    }
    let deleted_behaviors = behavior_delete.exec(db).await?.rows_affected;

    let mut state_index_delete = character_state_index::Entity::delete_many()
        .filter(character_state_index::Column::ProfileId.eq(profile_id))
        .filter(character_state_index::Column::ChapterNumber.gte(start_chapter));
    if let Some(end_chapter) = end_chapter {
        state_index_delete = state_index_delete
            .filter(character_state_index::Column::ChapterNumber.lte(end_chapter));
    }
    let deleted_state_indexes = state_index_delete.exec(db).await?.rows_affected;

    let mut snapshot_delete = snapshot_reference::Entity::delete_many()
        .filter(snapshot_reference::Column::ProfileId.eq(profile_id))
        .filter(snapshot_reference::Column::ChapterNumber.gte(start_chapter));
    if let Some(end_chapter) = end_chapter {
        snapshot_delete =
            snapshot_delete.filter(snapshot_reference::Column::ChapterNumber.lte(end_chapter));
    }
    let deleted_snapshots = snapshot_delete.exec(db).await?.rows_affected;

    Ok(CharacterTimelineDeletionSummary {
        attribute_event_ids,
        state_index_ids,
        deleted_attribute_events,
        deleted_behaviors,
        deleted_state_indexes,
        deleted_snapshots,
    })
}

pub async fn delete_timeline_artifacts_from_chapter_checked<C>(
    db: &C,
    profile_id: i64,
    start_chapter: i32,
    end_chapter: Option<i32>,
) -> Result<CharacterTimelineDeletionSummary, CharacterDbError>
where
    C: ConnectionTrait,
{
    validate_timeline_cleanup_range(start_chapter, end_chapter)?;
    delete_timeline_artifacts_from_chapter(db, profile_id, start_chapter, end_chapter).await
}

fn validate_timeline_cleanup_range(
    start_chapter: i32,
    end_chapter: Option<i32>,
) -> Result<(), CharacterDbError> {
    validate_chapter_number(start_chapter)?;
    if let Some(end_chapter) = end_chapter {
        validate_chapter_number(end_chapter)?;
        if start_chapter > end_chapter {
            return Err(CharacterDbError::InvalidArgument(
                "时间线清理起始章节不能大于结束章节".to_string(),
            ));
        }
    }
    Ok(())
}
