use a4n_core::{
    error::AppError,
    models::scene::{scene_occurrence, scene_profile, scene_transition},
};

use crate::OrchestratorContext;

impl OrchestratorContext {
    pub async fn scene_profiles_for_rag_ingest(
        &self,
        project_id: i64,
    ) -> Result<Vec<scene_profile::Model>, AppError> {
        a4n_db::rag::scene_profiles_for_rag_ingest(self.db.connection(), project_id)
            .await
            .map_err(AppError::Database)
    }

    pub async fn scene_occurrences_for_rag_ingest(
        &self,
        project_id: i64,
    ) -> Result<Vec<scene_occurrence::Model>, AppError> {
        a4n_db::rag::scene_occurrences_for_rag_ingest(self.db.connection(), project_id)
            .await
            .map_err(AppError::Database)
    }

    pub async fn scene_transitions_for_rag_ingest(
        &self,
        project_id: i64,
    ) -> Result<Vec<scene_transition::Model>, AppError> {
        a4n_db::rag::scene_transitions_for_rag_ingest(self.db.connection(), project_id)
            .await
            .map_err(AppError::Database)
    }
}
