#[cfg(feature = "embedded-prompt-git")]
use std::sync::Arc;

use a4n_config::app_config::AppConfig;
use a4n_prompt::tera_engine::TeraPromptEngine;

pub(super) fn create_prompt_engine(
    config: &AppConfig,
) -> Result<TeraPromptEngine, a4n_error::AppError> {
    let prompts_dir = crate::prompt_resources::bundled_prompts_dir(&config.data_dir)
        .map_err(a4n_error::AppError::ConfigError)?;
    let base = TeraPromptEngine::from_directory(&prompts_dir)?;
    #[cfg(feature = "embedded-prompt-git")]
    let engine = base.with_git_store(Arc::new(a4n_prompt_git::GitPromptStore::open_or_init(
        config.data_dir.join("prompts.git"),
    )?))?;
    #[cfg(not(feature = "embedded-prompt-git"))]
    let engine = base;
    Ok(engine)
}
