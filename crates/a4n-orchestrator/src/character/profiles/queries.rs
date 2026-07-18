use a4n_core::error::AppError;

use crate::OrchestratorContext;

impl OrchestratorContext {
    pub async fn profiles_by_project(
        &self,
        project_id: i64,
    ) -> Result<Vec<a4n_core::models::character::protagonist_profile::Model>, AppError> {
        a4n_db::character::profiles_by_project(self.db.connection(), project_id)
            .await
            .map_err(AppError::Database)
    }

    pub async fn profile_by_id(
        &self,
        profile_id: i64,
    ) -> Result<Option<a4n_core::models::character::protagonist_profile::Model>, AppError> {
        a4n_db::character::profile_by_id(self.db.connection(), profile_id)
            .await
            .map_err(AppError::Database)
    }

    pub async fn profile_by_project_name(
        &self,
        project_id: i64,
        name: &str,
    ) -> Result<Option<a4n_core::models::character::protagonist_profile::Model>, AppError> {
        a4n_db::character::profile_by_project_name(self.db.connection(), project_id, name)
            .await
            .map_err(AppError::Database)
    }

    pub async fn missing_portrait_profiles(
        &self,
        project_id: i64,
    ) -> Result<Vec<a4n_core::models::character::protagonist_profile::Model>, AppError> {
        a4n_db::character::missing_portrait_profiles(self.db.connection(), project_id)
            .await
            .map_err(AppError::Database)
    }
}
