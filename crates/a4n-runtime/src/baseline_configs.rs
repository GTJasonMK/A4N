//! Empty-database baseline AI configs.
//!
//! Server-side public configs are seeded only when their table is empty. They
//! keep shared API keys on the backend while still allowing users to override
//! model settings from their local runtime settings snapshot.

mod env;
mod image;
mod llm;
mod video;

use a4n_api_contract::config::{ConfigMutationApplicationPort, ConfigReadApplicationPort};
use a4n_config::app_config::AppConfig;
use a4n_error::AppError;

pub async fn ensure_baseline_model_configs(
    config_read_application: &dyn ConfigReadApplicationPort,
    config_mutation_application: &dyn ConfigMutationApplicationPort,
    config: &AppConfig,
) -> Result<(), AppError> {
    config_mutation_application
        .insert_baseline_embedding_if_empty()
        .await?;
    llm::ensure_public_llm_config(
        config_read_application,
        config_mutation_application,
        &config.llm,
    )
    .await?;
    image::ensure_public_image_config(
        config_read_application,
        config_mutation_application,
        &config.image_gen,
    )
    .await?;
    video::ensure_public_video_config(config_read_application, config_mutation_application).await?;
    Ok(())
}
