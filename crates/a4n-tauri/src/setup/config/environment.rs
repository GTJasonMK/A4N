use a4n_core::config::app_config::{AppConfig, apply_storage_env_overrides};
use a4n_core::config::env::{optional_non_empty_env_string, validate_postgres_database_url};
use a4n_core::error::AppError;
use std::path::Path;

pub(super) fn apply_env_overrides(
    config: &mut AppConfig,
    portable_home: &Path,
) -> Result<(), AppError> {
    apply_storage_env_overrides(config, portable_home)?;
    let database_url = optional_non_empty_env_string("A4N_DATABASE_URL")?;
    let jwt_secret = optional_non_empty_env_string("A4N_JWT_SECRET")?;
    let openai_api_key = optional_non_empty_env_string("A4N_OPENAI_API_KEY")?;
    let openai_base_url = optional_non_empty_env_string("A4N_OPENAI_BASE_URL")?;

    if let Some(db_url) = database_url {
        validate_postgres_database_url("A4N_DATABASE_URL", &db_url)?;
        config.database_url = db_url;
    }
    if let Some(jwt_secret) = jwt_secret {
        config.security.jwt_secret = Some(jwt_secret);
    }
    if let Some(api_key) = openai_api_key {
        config.llm.api_key = Some(api_key);
        config.embedding.api_key = config.llm.api_key.clone();
    }
    if let Some(base_url) = openai_base_url {
        config.llm.base_url = base_url;
    }
    Ok(())
}
