use std::collections::HashMap;

use a4n_core::error::AppError;
use a4n_prompt::blueprint_generation::BlueprintGenerationPreviewStep;
use a4n_prompt::rendering::PromptRenderer;
use serde_json::Value;
use tokio_util::sync::CancellationToken;

use crate::OrchestratorContext;
use generation::preview_generate_requests;

mod generation;
mod generation_contract;
mod json;
mod persistence;
mod refine;

pub(crate) use generation::BLUEPRINT_GENERATION_STAGES;

pub type BlueprintProgressCallback<'a> =
    dyn Fn(u32, u32, &str) -> Result<(), AppError> + Send + Sync + 'a;
pub type BlueprintProgressReporter<'a> = Option<&'a BlueprintProgressCallback<'a>>;

pub struct BlueprintGenerationInput<'a> {
    pub project_id: i64,
    pub novel_title: &'a str,
    pub genre: &'a str,
    pub target_length: i32,
    pub prompt_renderer: &'a dyn PromptRenderer,
    pub variables: &'a HashMap<String, Value>,
    pub cancel_token: Option<&'a CancellationToken>,
    pub progress: BlueprintProgressReporter<'a>,
    pub workflow_run_id: Option<i64>,
}

impl OrchestratorContext {
    pub async fn preview_blueprint_generation_requests(
        &self,
        project_id: i64,
        novel_title: &str,
        genre: &str,
        target_length: i32,
        prompt_renderer: &dyn PromptRenderer,
        variables: &HashMap<String, Value>,
    ) -> Result<Vec<BlueprintGenerationPreviewStep>, AppError> {
        preview_generate_requests(
            project_id,
            novel_title,
            genre,
            target_length,
            prompt_renderer,
            variables,
        )
        .await
    }
}

fn blueprint_db_error(error: a4n_db::blueprint::BlueprintDbError) -> AppError {
    match error {
        a4n_db::blueprint::BlueprintDbError::InvalidArgument(message) => {
            AppError::InvalidArgument(message)
        }
        a4n_db::blueprint::BlueprintDbError::NotFound(message) => AppError::NotFound(message),
        a4n_db::blueprint::BlueprintDbError::Database(error) => AppError::Database(error),
    }
}

fn blueprint_sync_error(error: a4n_db::project::BlueprintSyncError) -> AppError {
    match error {
        a4n_db::project::BlueprintSyncError::InvalidArgument(message) => {
            AppError::InvalidArgument(message)
        }
        a4n_db::project::BlueprintSyncError::Database(error) => AppError::Database(error),
    }
}

fn character_profile_seed_error(error: a4n_db::character::CharacterProfileSeedError) -> AppError {
    match error {
        a4n_db::character::CharacterProfileSeedError::Database(error) => AppError::Database(error),
        a4n_db::character::CharacterProfileSeedError::Internal(message) => {
            AppError::Internal(anyhow::anyhow!(message))
        }
    }
}

fn project_mutation_error(error: a4n_db::project::ProjectMutationError) -> AppError {
    match error {
        a4n_db::project::ProjectMutationError::InvalidArgument(message) => {
            AppError::InvalidArgument(message)
        }
        a4n_db::project::ProjectMutationError::NotFound(message) => AppError::NotFound(message),
        a4n_db::project::ProjectMutationError::Forbidden(message) => AppError::Forbidden(message),
        a4n_db::project::ProjectMutationError::Internal(message) => {
            AppError::Internal(anyhow::anyhow!(message))
        }
        a4n_db::project::ProjectMutationError::Database(error) => AppError::Database(error),
    }
}
