#[cfg(feature = "embedded-prompt-git")]
use std::sync::Arc;

use a4n_core::config::app_config::AppConfig;
use a4n_prompt::tera_engine::TeraPromptEngine;
use sea_orm::DatabaseConnection;

use super::portable::ensure_portable_prompts_dir;

pub(in crate::state) async fn create_prompt_engine(
    db: &DatabaseConnection,
    config: &AppConfig,
) -> Result<TeraPromptEngine, a4n_core::error::AppError> {
    let prompts_dir = ensure_portable_prompts_dir(&config.data_dir)
        .map_err(a4n_core::error::AppError::ConfigError)?;
    let base = TeraPromptEngine::from_directory(&prompts_dir)?;
    #[cfg(feature = "embedded-prompt-git")]
    let engine = base.with_git_store(Arc::new(a4n_prompt_git::GitPromptStore::open_or_init(
        config.data_dir.join("prompts.git"),
    )?))?;
    #[cfg(not(feature = "embedded-prompt-git"))]
    let engine = base;
    hydrate_project_overrides_from_settings(db, &engine)
        .await
        .map_err(a4n_core::error::AppError::ConfigError)?;
    Ok(engine)
}

async fn hydrate_project_overrides_from_settings(
    db: &DatabaseConnection,
    engine: &TeraPromptEngine,
) -> Result<usize, String> {
    let plan = a4n_db::prompt_override::project_override_hydration_plan(db)
        .await
        .map_err(|error| error.to_string())?;
    let overrides = plan
        .into_iter()
        .map(|item| (item.identity.project_id, item.identity.name, item.content))
        .collect();
    engine
        .hydrate_project_overrides(overrides)
        .map_err(|e| e.to_string())
}
