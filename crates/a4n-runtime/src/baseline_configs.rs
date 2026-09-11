//! Empty-database baseline AI configs.
//!
//! Server-side public configs are seeded only when their table is empty. They
//! keep shared API keys on the backend while still allowing users to override
//! model settings from their local runtime settings snapshot.

mod env;
mod image;
mod llm;
mod video;

use a4n_api_contract::config::ConfigApplicationPort;
use a4n_config::app_config::AppConfig;
use a4n_error::AppError;

pub async fn ensure_baseline_model_configs(
    config_application: &dyn ConfigApplicationPort,
    config: &AppConfig,
) -> Result<(), AppError> {
    config_application
        .insert_baseline_embedding_if_empty()
        .await?;
    llm::ensure_public_llm_config(config_application, &config.llm).await?;
    image::ensure_public_image_config(config_application, &config.image_gen).await?;
    video::ensure_public_video_config(config_application).await?;
    Ok(())
}
