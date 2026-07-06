use a4n_core::error::AppError;
use a4n_core::models::character::{
    attribute_event, character_state_index, protagonist_behavior_record, snapshot_reference,
};
use sea_orm::{ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, QuerySelect};

use super::{CharacterService, validate_chapter_number};

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CharacterTimelineDeletionSummary {
    pub attribute_event_ids: Vec<i64>,
    pub state_index_ids: Vec<i64>,
    pub deleted_attribute_events: u64,
    pub deleted_behaviors: u64,
    pub deleted_state_indexes: u64,
    pub deleted_snapshots: u64,
}

impl CharacterService {
    pub async fn delete_timeline_artifacts_from_chapter<C>(
        &self,
        db: &C,
        profile_id: i64,
        start_chapter: i32,
        end_chapter: Option<i32>,
    ) -> Result<CharacterTimelineDeletionSummary, AppError>
    where
        C: ConnectionTrait,
    {
        validate_chapter_number(start_chapter)?;
        if let Some(end_chapter) = end_chapter {
            validate_chapter_number(end_chapter)?;
            if start_chapter > end_chapter {
                return Err(AppError::InvalidArgument(
                    "时间线清理起始章节不能大于结束章节".to_string(),
                ));
            }
        }

        let mut attribute_event_ids_query = attribute_event::Entity::find()
            .select_only()
            .column(attribute_event::Column::Id)
            .filter(attribute_event::Column::ProfileId.eq(profile_id))
            .filter(attribute_event::Column::ChapterNumber.gte(start_chapter));
        if let Some(end_chapter) = end_chapter {
            attribute_event_ids_query = attribute_event_ids_query
                .filter(attribute_event::Column::ChapterNumber.lte(end_chapter));
        }
        let attribute_event_ids = attribute_event_ids_query
            .into_tuple::<i64>()
            .all(db)
            .await
            .map_err(AppError::Database)?;

        let mut state_index_ids_query = character_state_index::Entity::find()
            .select_only()
            .column(character_state_index::Column::Id)
            .filter(character_state_index::Column::ProfileId.eq(profile_id))
            .filter(character_state_index::Column::ChapterNumber.gte(start_chapter));
        if let Some(end_chapter) = end_chapter {
            state_index_ids_query = state_index_ids_query
                .filter(character_state_index::Column::ChapterNumber.lte(end_chapter));
        }
        let state_index_ids = state_index_ids_query
            .into_tuple::<i64>()
            .all(db)
            .await
            .map_err(AppError::Database)?;

        let mut attribute_event_delete = attribute_event::Entity::delete_many()
            .filter(attribute_event::Column::ProfileId.eq(profile_id))
            .filter(attribute_event::Column::ChapterNumber.gte(start_chapter));
        if let Some(end_chapter) = end_chapter {
            attribute_event_delete = attribute_event_delete
                .filter(attribute_event::Column::ChapterNumber.lte(end_chapter));
        }
        let deleted_attribute_events = attribute_event_delete
            .exec(db)
            .await
            .map_err(AppError::Database)?
            .rows_affected;

        let mut behavior_delete = protagonist_behavior_record::Entity::delete_many()
            .filter(protagonist_behavior_record::Column::ProfileId.eq(profile_id))
            .filter(protagonist_behavior_record::Column::ChapterNumber.gte(start_chapter));
        if let Some(end_chapter) = end_chapter {
            behavior_delete = behavior_delete
                .filter(protagonist_behavior_record::Column::ChapterNumber.lte(end_chapter));
        }
        let deleted_behaviors = behavior_delete
            .exec(db)
            .await
            .map_err(AppError::Database)?
            .rows_affected;

        let mut state_index_delete = character_state_index::Entity::delete_many()
            .filter(character_state_index::Column::ProfileId.eq(profile_id))
            .filter(character_state_index::Column::ChapterNumber.gte(start_chapter));
        if let Some(end_chapter) = end_chapter {
            state_index_delete = state_index_delete
                .filter(character_state_index::Column::ChapterNumber.lte(end_chapter));
        }
        let deleted_state_indexes = state_index_delete
            .exec(db)
            .await
            .map_err(AppError::Database)?
            .rows_affected;

        let mut snapshot_delete = snapshot_reference::Entity::delete_many()
            .filter(snapshot_reference::Column::ProfileId.eq(profile_id))
            .filter(snapshot_reference::Column::ChapterNumber.gte(start_chapter));
        if let Some(end_chapter) = end_chapter {
            snapshot_delete =
                snapshot_delete.filter(snapshot_reference::Column::ChapterNumber.lte(end_chapter));
        }
        let deleted_snapshots = snapshot_delete
            .exec(db)
            .await
            .map_err(AppError::Database)?
            .rows_affected;

        Ok(CharacterTimelineDeletionSummary {
            attribute_event_ids,
            state_index_ids,
            deleted_attribute_events,
            deleted_behaviors,
            deleted_state_indexes,
            deleted_snapshots,
        })
    }
}
