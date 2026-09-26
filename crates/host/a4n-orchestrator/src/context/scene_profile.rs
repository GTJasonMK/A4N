use a4n_db::DbHandle;
use a4n_error::AppError;

use super::OrchestratorContext;
use crate::scene_profile::scene_profile_db_error;

#[derive(Clone)]
pub(super) struct SceneProfileApplication {
    database: DbHandle,
}

impl SceneProfileApplication {
    pub(super) fn new(database: DbHandle) -> Self {
        Self { database }
    }

    async fn scene_profiles_by_project(
        &self,
        project_id: i64,
    ) -> Result<Vec<a4n_db_models::models::scene::scene_profile::Model>, AppError> {
        a4n_db::scene_profile::scene_profiles_by_project(&self.database, project_id)
            .await
            .map_err(scene_profile_db_error)
    }

    async fn scene_profile_by_id(
        &self,
        scene_profile_id: i64,
    ) -> Result<a4n_db_models::models::scene::scene_profile::Model, AppError> {
        a4n_db::scene_profile::scene_profile_by_id(&self.database, scene_profile_id)
            .await
            .map_err(scene_profile_db_error)
    }
}

impl OrchestratorContext {
    pub(crate) async fn scene_profiles_by_project(
        &self,
        project_id: i64,
    ) -> Result<Vec<a4n_db_models::models::scene::scene_profile::Model>, AppError> {
        self.scene_profile
            .scene_profiles_by_project(project_id)
            .await
    }

    pub(crate) async fn scene_profile_by_id(
        &self,
        scene_profile_id: i64,
    ) -> Result<a4n_db_models::models::scene::scene_profile::Model, AppError> {
        self.scene_profile
            .scene_profile_by_id(scene_profile_id)
            .await
    }
}
