use super::contracts::CharacterStateIndexUpsertInput;
use a4n_core::models::character::{character_state_index, protagonist_behavior_record};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, DbErr, EntityTrait, QueryFilter, QueryOrder,
    Set,
};

pub(in crate::character) async fn upsert_state_index<C>(
    db: &C,
    input: CharacterStateIndexUpsertInput,
) -> Result<character_state_index::Model, DbErr>
where
    C: ConnectionTrait,
{
    let now = chrono::Utc::now().naive_utc();
    let existing = character_state_index::Entity::find()
        .filter(character_state_index::Column::ProfileId.eq(input.profile_id))
        .filter(character_state_index::Column::ChapterNumber.eq(input.chapter_number))
        .one(db)
        .await?;

    if let Some(idx) = existing {
        let mut active: character_state_index::ActiveModel = idx.into();
        active.state_summary = Set(input.state_summary);
        active.key_attributes = Set(input.key_attributes);
        active.emotional_state = Set(input.emotional_state);
        active.current_location = Set(input.current_location);
        active.current_goal = Set(input.current_goal);
        return active.update(db).await;
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
