use a4n_core::{
    error::AppError,
    models::project::{blueprint_version, novel_blueprint},
};

use crate::OrchestratorContext;

use super::super::{
    BlueprintRestoreResult, blueprint_restore_error_to_app, project_mutation_error_to_app_error,
};

impl OrchestratorContext {
    pub async fn novel_blueprint_for_project(
        &self,
        project_id: i64,
    ) -> Result<Option<novel_blueprint::Model>, AppError> {
        a4n_db::project::novel_blueprint_for_project(self.db.connection(), project_id)
            .await
            .map_err(AppError::Database)
    }

    pub async fn blueprint_versions_by_project(
        &self,
        project_id: i64,
    ) -> Result<Vec<blueprint_version::Model>, AppError> {
        a4n_db::project::blueprint_versions_by_project(self.db.connection(), project_id)
            .await
            .map_err(project_mutation_error_to_app_error)
    }

    pub async fn blueprint_version_by_project_version(
        &self,
        project_id: i64,
        version: &str,
    ) -> Result<Option<blueprint_version::Model>, AppError> {
        a4n_db::project::blueprint_version_by_project_version(
            self.db.connection(),
            project_id,
            version,
        )
        .await
        .map_err(project_mutation_error_to_app_error)
    }

    pub async fn restore_blueprint_version_record(
        &self,
        project_id: i64,
        version: &str,
    ) -> Result<BlueprintRestoreResult, AppError> {
        a4n_db::project::restore_blueprint_version_record(self.db.connection(), project_id, version)
            .await
            .map_err(blueprint_restore_error_to_app)
    }

    pub async fn update_blueprint_world_setting(
        &self,
        project_id: i64,
        world_setting: serde_json::Value,
    ) -> Result<Option<novel_blueprint::Model>, AppError> {
        a4n_db::project::update_blueprint_world_setting(
            self.db.connection(),
            project_id,
            world_setting,
        )
        .await
        .map_err(project_mutation_error_to_app_error)
    }
}
