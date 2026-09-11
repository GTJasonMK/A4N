use super::validation::validate_behavior_record_input;
use crate::character::CharacterDbError;
use a4n_db_models::models::character::protagonist_behavior_record;
use a4n_domain::character::InsertBehaviorRecordInput;
use sea_orm::{ActiveModelTrait, ConnectionTrait, DbErr, Set};

async fn insert_behavior_record<C>(
    db: &C,
    input: InsertBehaviorRecordInput,
) -> Result<protagonist_behavior_record::Model, DbErr>
where
    C: ConnectionTrait,
{
    let now = chrono::Utc::now().naive_utc();
    protagonist_behavior_record::ActiveModel {
        profile_id: Set(input.profile_id),
        chapter_number: Set(input.chapter_number),
        source_kind: Set(input.source_kind),
        behavior_type: Set(input.behavior_type),
        behavior_description: Set(input.behavior_description),
        original_text: Set(input.original_text),
        behavior_tags: Set(input.behavior_tags),
        classification_results: Set(input.classification_results),
        related_characters: Set(input.related_characters),
        impact_analysis: Set(input.impact_analysis),
        created_at: Set(now),
        ..Default::default()
    }
    .insert(db)
    .await
}

pub(in crate::character) async fn record_behavior_in<C>(
    db: &C,
    input: InsertBehaviorRecordInput,
) -> Result<protagonist_behavior_record::Model, CharacterDbError>
where
    C: ConnectionTrait,
{
    let input = validate_behavior_record_input(input)?;
    insert_behavior_record(db, input)
        .await
        .map_err(CharacterDbError::Database)
}
