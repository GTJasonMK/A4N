use a4n_core::error::AppError;
use a4n_core::models::character::character_state_index;
use chrono::Utc;
use sea_orm::{ActiveModelTrait, ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, Set};

use super::CharacterService;

#[derive(Debug)]
pub struct StateIndexUpsertInput<'a> {
    pub profile_id: i64,
    pub chapter_number: i32,
    pub state_summary: &'a str,
    pub key_attributes: serde_json::Value,
    pub emotional_state: Option<&'a str>,
    pub current_location: Option<&'a str>,
    pub current_goal: Option<&'a str>,
}

impl CharacterService {
    pub async fn upsert_state_index_in<C>(
        &self,
        db: &C,
        input: StateIndexUpsertInput<'_>,
    ) -> Result<character_state_index::Model, AppError>
    where
        C: ConnectionTrait,
    {
        let StateIndexUpsertInput {
            profile_id,
            chapter_number,
            state_summary,
            key_attributes,
            emotional_state,
            current_location,
            current_goal,
        } = input;
        let now = Utc::now().naive_utc();
        let existing = character_state_index::Entity::find()
            .filter(character_state_index::Column::ProfileId.eq(profile_id))
            .filter(character_state_index::Column::ChapterNumber.eq(chapter_number))
            .one(db)
            .await
            .map_err(AppError::Database)?;

        if let Some(idx) = existing {
            let mut active: character_state_index::ActiveModel = idx.into();
            active.state_summary = Set(state_summary.to_string());
            active.key_attributes = Set(key_attributes);
            active.emotional_state = Set(emotional_state.map(|s| s.to_string()));
            active.current_location = Set(current_location.map(|s| s.to_string()));
            active.current_goal = Set(current_goal.map(|s| s.to_string()));
            active.update(db).await.map_err(AppError::Database)
        } else {
            character_state_index::ActiveModel {
                profile_id: Set(profile_id),
                chapter_number: Set(chapter_number),
                state_summary: Set(state_summary.to_string()),
                key_attributes: Set(key_attributes),
                emotional_state: Set(emotional_state.map(|s| s.to_string())),
                current_location: Set(current_location.map(|s| s.to_string())),
                current_goal: Set(current_goal.map(|s| s.to_string())),
                created_at: Set(now),
                ..Default::default()
            }
            .insert(db)
            .await
            .map_err(AppError::Database)
        }
    }
}
