//! Empty-database baseline AI configs.
//!
//! Server-side public configs are seeded only when their table is empty. They
//! keep shared API keys on the backend while still allowing users to override
//! model settings from their local runtime settings snapshot.

mod env;
mod image;
mod llm;
mod video;

use a4n_core::{config::app_config::AppConfig, error::AppError};
use a4n_orchestrator::OrchestratorContext;

pub async fn ensure_baseline_model_configs(
    orchestrator: &OrchestratorContext,
    config: &AppConfig,
) -> Result<(), AppError> {
    ensure_baseline_embedding_config(orchestrator).await?;
    llm::ensure_public_llm_config(orchestrator, &config.llm).await?;
    image::ensure_public_image_config(orchestrator, &config.image_gen).await?;
    video::ensure_public_video_config(orchestrator).await?;
    Ok(())
}

async fn ensure_baseline_embedding_config(
    orchestrator: &OrchestratorContext,
) -> Result<(), AppError> {
    orchestrator
        .insert_baseline_embedding_config_if_empty()
        .await?;
    Ok(())
}
