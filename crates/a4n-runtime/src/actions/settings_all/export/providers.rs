use super::super::types::{EmbeddingRow, ImageRow, LlmRow, VideoRow};
use crate::AppState;
use a4n_core::error::AppError;

pub(super) struct ProviderExportRows {
    pub(super) llm: Vec<LlmRow>,
    pub(super) embedding: Vec<EmbeddingRow>,
    pub(super) image: Vec<ImageRow>,
    pub(super) video: Vec<VideoRow>,
}

pub(super) async fn provider_export_rows(state: &AppState) -> Result<ProviderExportRows, AppError> {
    let llm = state
        .orchestrator
        .settings_all_export_llm_configs()
        .await?
        .into_iter()
        .map(|model| LlmRow {
            name: model.name,
            provider: model.provider,
            base_url: model.base_url,
            api_key_encrypted: model.api_key_encrypted,
            model: model.model,
            max_concurrent: model.max_concurrent,
            timeout_secs: model.timeout_secs,
            max_context_tokens: model.max_context_tokens,
            max_output_tokens: model.max_output_tokens,
            request_options_json: model.request_options_json,
            is_default: model.is_default,
        })
        .collect();
    let embedding = state
        .orchestrator
        .settings_all_export_embedding_configs()
        .await?
        .into_iter()
        .map(|model| EmbeddingRow {
            name: model.name,
            backend: model.backend,
            base_url: model.base_url,
            api_key_encrypted: model.api_key_encrypted,
            model: model.model,
            dimension: model.dimension,
            batch_size: model.batch_size,
            local_model_path: model.local_model_path,
            is_default: model.is_default,
        })
        .collect();
    let image = state
        .orchestrator
        .settings_all_export_image_configs()
        .await?
        .into_iter()
        .map(|model| ImageRow {
            name: model.name,
            backend: model.backend,
            base_url: model.base_url,
            api_key_encrypted: model.api_key_encrypted,
            model: model.model,
            max_concurrent: model.max_concurrent,
            default_width: model.default_width,
            default_height: model.default_height,
            provider_options_json: model.provider_options_json,
            is_default: model.is_default,
        })
        .collect();
    let video = state
        .orchestrator
        .settings_all_export_video_configs()
        .await?
        .into_iter()
        .map(|model| VideoRow {
            name: model.name,
            backend: model.backend,
            base_url: model.base_url,
            api_key_encrypted: model.api_key_encrypted,
            model: model.model,
            max_concurrent: model.max_concurrent,
            default_width: model.default_width,
            default_height: model.default_height,
            default_duration_secs: model.default_duration_secs,
            min_duration_secs: model.min_duration_secs,
            max_duration_secs: model.max_duration_secs,
            duration_mode: model.duration_mode,
            supported_durations_json: model.supported_durations_json,
            default_fps: model.default_fps,
            provider_options_json: model.provider_options_json,
            is_default: model.is_default,
        })
        .collect();
    Ok(ProviderExportRows {
        llm,
        embedding,
        image,
        video,
    })
}
