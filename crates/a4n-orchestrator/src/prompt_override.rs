pub use a4n_db::prompt_override::{
    ProjectOverrideHydration, ProjectOverrideIdentity, ProjectPromptOverrideSummary,
    PromptOverrideDbError, normalize_project_override_identity, parse_project_override_key,
    project_override_key, validate_project_override_content, validate_project_override_project_id,
};

use std::collections::HashSet;

use crate::OrchestratorContext;
use a4n_core::error::AppError;

impl OrchestratorContext {
    pub async fn load_project_override_persisted_content(
        &self,
        identity: &ProjectOverrideIdentity,
    ) -> Result<Option<String>, AppError> {
        a4n_db::prompt_override::load_project_override_persisted_content(
            self.db.connection(),
            identity,
        )
        .await
        .map_err(prompt_override_db_error)
    }

    pub async fn save_project_override(
        &self,
        project_id: i64,
        name: &str,
        content: &str,
    ) -> Result<ProjectOverrideIdentity, AppError> {
        a4n_db::prompt_override::save_project_override(
            self.db.connection(),
            project_id,
            name,
            content,
        )
        .await
        .map_err(prompt_override_db_error)
    }

    pub async fn delete_project_override(
        &self,
        identity: &ProjectOverrideIdentity,
    ) -> Result<(), AppError> {
        a4n_db::prompt_override::delete_project_override(self.db.connection(), identity)
            .await
            .map_err(prompt_override_db_error)
    }

    pub async fn delete_project_overrides_for_prompt_names(
        &self,
        prompt_names: &HashSet<String>,
    ) -> Result<u64, AppError> {
        a4n_db::prompt_override::delete_project_overrides_for_prompt_names(
            self.db.connection(),
            prompt_names,
        )
        .await
        .map_err(prompt_override_db_error)
    }

    pub async fn preflight_project_override_keys_for_retired_cleanup(
        &self,
    ) -> Result<(), AppError> {
        a4n_db::prompt_override::preflight_project_override_keys_for_retired_cleanup(
            self.db.connection(),
        )
        .await
        .map_err(prompt_override_db_error)
    }

    pub async fn project_override_summaries(
        &self,
        project_id: i64,
        active_prompt_names: &HashSet<String>,
    ) -> Result<Vec<ProjectPromptOverrideSummary>, AppError> {
        a4n_db::prompt_override::project_override_summaries(
            self.db.connection(),
            project_id,
            active_prompt_names,
        )
        .await
        .map_err(prompt_override_db_error)
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
