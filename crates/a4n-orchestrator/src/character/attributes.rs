use super::{CharacterAttributeEventHistoryFilter, character_db_error};
use crate::OrchestratorContext;
use a4n_core::error::AppError;

impl OrchestratorContext {
    pub async fn attribute_events_by_ids(
        &self,
        attribute_event_ids: &[i64],
    ) -> Result<Vec<a4n_core::models::character::attribute_event::Model>, AppError> {
        a4n_db::character::attribute_events_by_ids(self.db.connection(), attribute_event_ids)
            .await
            .map_err(AppError::Database)
    }

    pub async fn attribute_history(
        &self,
        profile_id: i64,
        filter: CharacterAttributeEventHistoryFilter,
    ) -> Result<Vec<a4n_core::models::character::attribute_event::Model>, AppError> {
        a4n_db::character::attribute_history(self.db.connection(), profile_id, filter)
            .await
            .map_err(character_db_error)
    }
}
