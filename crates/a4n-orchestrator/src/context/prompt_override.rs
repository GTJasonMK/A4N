//! 提示词项目覆盖领域私有 owner。
//!
//! 持有 `DbHandle`，封装项目级 prompt override 的读写与清理。

use std::collections::HashSet;

use a4n_db::DbHandle;
use a4n_db::prompt_override::{
    ProjectOverrideIdentity, ProjectPromptOverrideSummary, PromptOverrideDbError,
};
use a4n_error::AppError;
use a4n_prompt::tera_engine::TeraPromptEngine;

use super::OrchestratorContext;

#[derive(Clone)]
pub(super) struct PromptOverrideApplication {
    database: DbHandle,
}

impl PromptOverrideApplication {
    pub(super) fn new(database: DbHandle) -> Self {
        Self { database }
    }

    fn connection(&self) -> &sea_orm::DatabaseConnection {
        self.database.connection()
    }

    async fn load_project_override_persisted_content(
        &self,
        identity: &ProjectOverrideIdentity,
    ) -> Result<Option<String>, AppError> {
        a4n_db::prompt_override::load_project_override_persisted_content(
            self.connection(),
            identity,
        )
        .await
        .map_err(prompt_override_db_error)
    }

    async fn save_project_override(
        &self,
        project_id: i64,
        name: &str,
        content: &str,
    ) -> Result<ProjectOverrideIdentity, AppError> {
        a4n_db::prompt_override::save_project_override(self.connection(), project_id, name, content)
            .await
            .map_err(prompt_override_db_error)
    }

    async fn delete_project_override(
        &self,
        identity: &ProjectOverrideIdentity,
    ) -> Result<(), AppError> {
        a4n_db::prompt_override::delete_project_override(self.connection(), identity)
            .await
            .map_err(prompt_override_db_error)
    }

    async fn delete_project_overrides_for_prompt_names(
        &self,
        prompt_names: &HashSet<String>,
    ) -> Result<u64, AppError> {
        a4n_db::prompt_override::delete_project_overrides_for_prompt_names(
            self.connection(),
            prompt_names,
        )
        .await
        .map_err(prompt_override_db_error)
    }

    async fn preflight_project_override_keys_for_retired_cleanup(&self) -> Result<(), AppError> {
        a4n_db::prompt_override::preflight_project_override_keys_for_retired_cleanup(
            self.connection(),
        )
        .await
        .map_err(prompt_override_db_error)
    }

    async fn project_override_summaries(
        &self,
        project_id: i64,
        active_prompt_names: &HashSet<String>,
    ) -> Result<Vec<ProjectPromptOverrideSummary>, AppError> {
        a4n_db::prompt_override::project_override_summaries(
            self.connection(),
            project_id,
            active_prompt_names,
        )
        .await
        .map_err(prompt_override_db_error)
    }

    async fn hydrate_prompt_engine(&self, engine: &TeraPromptEngine) -> Result<(), AppError> {
        let plan = a4n_db::prompt_override::project_override_hydration_plan(self.connection())
            .await
            .map_err(prompt_override_db_error)?;
        let overrides = plan
            .into_iter()
            .map(|item| (item.identity.project_id, item.identity.name, item.content))
            .collect();
        engine
            .hydrate_project_overrides(overrides)
            .map_err(|error| AppError::ConfigError(error.to_string()))
    }
}

fn prompt_override_db_error(error: PromptOverrideDbError) -> AppError {
    match error {
        PromptOverrideDbError::InvalidArgument(message) => AppError::InvalidArgument(message),
        PromptOverrideDbError::NotFound(message) => AppError::NotFound(message),
        PromptOverrideDbError::Project { source } => {
            crate::project::project_mutation_error_to_app_error(source)
        }
        PromptOverrideDbError::Database { source, .. } => AppError::Database(source),
    }
}

impl OrchestratorContext {
    pub(crate) async fn load_project_override_persisted_content(
        &self,
        identity: &ProjectOverrideIdentity,
    ) -> Result<Option<String>, AppError> {
        self.prompt_override
            .load_project_override_persisted_content(identity)
            .await
    }

    pub(crate) async fn save_project_override(
        &self,
        project_id: i64,
        name: &str,
        content: &str,
    ) -> Result<ProjectOverrideIdentity, AppError> {
        self.prompt_override
            .save_project_override(project_id, name, content)
            .await
    }

    pub(crate) async fn delete_project_override(
        &self,
        identity: &ProjectOverrideIdentity,
    ) -> Result<(), AppError> {
        self.prompt_override.delete_project_override(identity).await
    }

    pub(crate) async fn delete_project_overrides_for_prompt_names(
        &self,
        prompt_names: &HashSet<String>,
    ) -> Result<u64, AppError> {
        self.prompt_override
            .delete_project_overrides_for_prompt_names(prompt_names)
            .await
    }

    pub(crate) async fn preflight_project_override_keys_for_retired_cleanup(
        &self,
    ) -> Result<(), AppError> {
        self.prompt_override
            .preflight_project_override_keys_for_retired_cleanup()
            .await
    }

    pub(crate) async fn project_override_summaries(
        &self,
        project_id: i64,
        active_prompt_names: &HashSet<String>,
    ) -> Result<Vec<ProjectPromptOverrideSummary>, AppError> {
        self.prompt_override
            .project_override_summaries(project_id, active_prompt_names)
            .await
    }

    /// Load persisted project overrides into the prompt engine during bootstrap.
    ///
    /// Runtime creates the engine, while this owner performs the database read
    /// and applies the validated overrides.
    pub(crate) async fn hydrate_prompt_overrides(&self) -> Result<(), AppError> {
        self.prompt_override
            .hydrate_prompt_engine(&self.engine)
            .await
    }
}
