use a4n_core::error::AppError;
use a4n_core::models::project::{blueprint_character, blueprint_relationship, novel_blueprint};

use crate::blueprint::OrchestratorContext;

impl OrchestratorContext {
    pub(in crate::blueprint) async fn load_blueprint(
        &self,
        project_id: i64,
    ) -> Result<Option<novel_blueprint::Model>, AppError> {
        a4n_db::project::novel_blueprint_for_project(self.db.connection(), project_id)
            .await
            .map_err(AppError::Database)
    }

    pub(in crate::blueprint) async fn load_blueprint_characters(
        &self,
        project_id: i64,
    ) -> Result<Vec<blueprint_character::Model>, AppError> {
        a4n_db::project::blueprint_characters_for_project(self.db.connection(), project_id)
            .await
            .map_err(AppError::Database)
    }

    pub(in crate::blueprint) async fn load_blueprint_relationships(
        &self,
        project_id: i64,
    ) -> Result<Vec<blueprint_relationship::Model>, AppError> {
        a4n_db::project::blueprint_relationships_for_project(self.db.connection(), project_id)
            .await
            .map_err(AppError::Database)
    }
}
