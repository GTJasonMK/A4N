use super::contracts::CharacterStateIndexUpsertInput;
use a4n_db_models::models::character::{character_state_index, protagonist_behavior_record};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, DbErr, EntityTrait, QueryFilter, QueryOrder,
    QuerySelect, Set,
};

pub(in crate::character) async fn upsert_state_index<C>(
    db: &C,
    input: CharacterStateIndexUpsertInput,
) -> Result<character_state_index::Model, DbErr>
where
    C: ConnectionTrait,
{
    let now = chrono::Utc::now().naive_utc();
    let existing_id = character_state_index::Entity::find()
        .select_only()
        .column(character_state_index::Column::Id)
        .filter(character_state_index::Column::ProfileId.eq(input.profile_id))
        .filter(character_state_index::Column::ChapterNumber.eq(input.chapter_number))
        .into_tuple::<i64>()
        .one(db)
        .await?;

    if let Some(id) = existing_id {
        return character_state_index::ActiveModel {
            id: Set(id),
            state_summary: Set(input.state_summary),
            key_attributes: Set(input.key_attributes),
            emotional_state: Set(input.emotional_state),
            current_location: Set(input.current_location),
            current_goal: Set(input.current_goal),
            ..Default::default()
        }
        .update(db)
        .await;
    }

    character_state_index::ActiveModel {
        profile_id: Set(input.profile_id),
        chapter_number: Set(input.chapter_number),
        state_summary: Set(input.state_summary),
        key_attributes: Set(input.key_attributes),
        emotional_state: Set(input.emotional_state),
        current_location: Set(input.current_location),
        current_goal: Set(input.current_goal),
        created_at: Set(now),
        ..Default::default()
    }
    .insert(db)
    .await
}

pub async fn behavior_records_before_chapter<C>(
    db: &C,
    profile_id: i64,
    start_chapter: i32,
) -> Result<Vec<protagonist_behavior_record::Model>, DbErr>
where
    C: ConnectionTrait,
{
    protagonist_behavior_record::Entity::find()
        .filter(protagonist_behavior_record::Column::ProfileId.eq(profile_id))
        .filter(protagonist_behavior_record::Column::ChapterNumber.lt(start_chapter))
        .order_by_desc(protagonist_behavior_record::Column::ChapterNumber)
        .order_by_desc(protagonist_behavior_record::Column::Id)
        .all(db)
        .await
}
