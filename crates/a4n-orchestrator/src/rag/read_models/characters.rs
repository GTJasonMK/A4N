use a4n_core::{
    error::AppError,
    models::{
        character::{attribute_event, character_state_index, protagonist_profile},
        project::{blueprint_character, blueprint_relationship},
    },
};

use crate::OrchestratorContext;

impl OrchestratorContext {
    pub async fn blueprint_characters_for_rag_search(
        &self,
        project_id: i64,
    ) -> Result<Vec<blueprint_character::Model>, AppError> {
        a4n_db::rag::blueprint_characters_for_rag_search(self.db.connection(), project_id)
            .await
            .map_err(AppError::Database)
    }

    pub async fn blueprint_characters_for_rag_ingest(
        &self,
        project_id: i64,
    ) -> Result<Vec<blueprint_character::Model>, AppError> {
        a4n_db::rag::blueprint_characters_for_rag_ingest(self.db.connection(), project_id)
            .await
            .map_err(AppError::Database)
    }

    pub async fn blueprint_relationships_for_rag_ingest(
        &self,
        project_id: i64,
    ) -> Result<Vec<blueprint_relationship::Model>, AppError> {
        a4n_db::rag::blueprint_relationships_for_rag_ingest(self.db.connection(), project_id)
            .await
            .map_err(AppError::Database)
    }

    pub async fn protagonist_profiles_for_rag_ingest(
        &self,
        project_id: i64,
    ) -> Result<Vec<protagonist_profile::Model>, AppError> {
        a4n_db::rag::protagonist_profiles_for_rag_ingest(self.db.connection(), project_id)
            .await
            .map_err(AppError::Database)
    }

    pub async fn character_states_for_rag_ingest(
        &self,
        profile_ids: &[i64],
    ) -> Result<Vec<character_state_index::Model>, AppError> {
        a4n_db::rag::character_states_for_rag_ingest(self.db.connection(), profile_ids)
            .await
            .map_err(AppError::Database)
    }

    pub async fn attribute_events_for_rag_ingest(
        &self,
        profile_ids: &[i64],
    ) -> Result<Vec<attribute_event::Model>, AppError> {
        a4n_db::rag::attribute_events_for_rag_ingest(self.db.connection(), profile_ids)
            .await
            .map_err(AppError::Database)
    }
}
