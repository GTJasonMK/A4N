use super::{providers::provider_export_rows, user_state::user_state_export_rows};
use crate::AppState;
use a4n_core::error::AppError;
use a4n_export_core::{ExportDocumentKind, ExportRenderJsonInput};
use chrono::Utc;

use super::super::types::AllConfigsExport;

pub async fn export_all_configs_to_json(state: &AppState) -> Result<String, AppError> {
    let providers = provider_export_rows(state).await?;
    let user_state = user_state_export_rows(state).await?;
    let export = AllConfigsExport {
        version: "1.0".into(),
        export_time: Utc::now().to_rfc3339(),
        export_type: "all".into(),
        llm_configs: providers.llm,
        embedding_configs: providers.embedding,
        image_configs: providers.image,
        video_configs: providers.video,
        prompts_modified: user_state.prompts,
        themes: user_state.themes,
        system_settings: user_state.settings,
    };

    let payload = serde_json::to_value(export).map_err(AppError::Serialization)?;
    let rendered = crate::sidecar_clients::render_export_json(
        state.runtime_profile,
        state.capabilities(),
        ExportRenderJsonInput {
            kind: ExportDocumentKind::SettingsAll,
            payload,
            pretty: true,
        },
        None,
        None,
    )
    .await
    .map_err(AppError::ConfigError)?;
    Ok(rendered.content)
}
