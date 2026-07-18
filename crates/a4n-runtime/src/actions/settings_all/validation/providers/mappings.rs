use super::super::super::types::{EmbeddingRow, ImageRow, VideoRow};
use a4n_core::config::{CreateEmbeddingConfigParams, CreateImageConfigParams};
use a4n_orchestrator::video::VideoConfigParams;

pub(super) fn embedding_params_from_row(row: &EmbeddingRow) -> CreateEmbeddingConfigParams {
    CreateEmbeddingConfigParams {
        name: row.name.clone(),
        backend: row.backend.clone(),
        base_url: row.base_url.clone(),
        api_key_encrypted: row.api_key_encrypted.clone(),
        model: row.model.clone(),
        dimension: row.dimension,
        batch_size: row.batch_size,
        local_model_path: row.local_model_path.clone(),
    }
}

pub(super) fn apply_embedding_params_to_row(
    row: &mut EmbeddingRow,
    params: CreateEmbeddingConfigParams,
) {
    row.name = params.name;
    row.backend = params.backend;
    row.base_url = params.base_url;
    row.api_key_encrypted = params.api_key_encrypted;
    row.model = params.model;
    row.dimension = params.dimension;
    row.batch_size = params.batch_size;
    row.local_model_path = params.local_model_path;
}

pub(super) fn image_params_from_row(row: &ImageRow) -> CreateImageConfigParams {
    CreateImageConfigParams {
        name: row.name.clone(),
        backend: row.backend.clone(),
        base_url: row.base_url.clone().unwrap_or_default(),
        api_key_encrypted: row.api_key_encrypted.clone(),
        model: row.model.clone(),
        max_concurrent: row.max_concurrent,
        default_width: row.default_width,
        default_height: row.default_height,
        provider_options_json: row.provider_options_json.clone(),
    }
}

pub(super) fn apply_image_params_to_row(row: &mut ImageRow, params: CreateImageConfigParams) {
    row.name = params.name;
    row.backend = params.backend;
    row.base_url = Some(params.base_url);
    row.api_key_encrypted = params.api_key_encrypted;
    row.model = params.model;
    row.max_concurrent = params.max_concurrent;
    row.default_width = params.default_width;
    row.default_height = params.default_height;
    row.provider_options_json = params.provider_options_json;
}

pub(super) fn video_params_from_row(row: &VideoRow) -> VideoConfigParams {
    VideoConfigParams {
        name: row.name.clone(),
        backend: row.backend.clone(),
        base_url: row.base_url.clone().unwrap_or_default(),
        api_key_encrypted: row.api_key_encrypted.clone(),
        model: row.model.clone(),
        max_concurrent: row.max_concurrent,
        default_width: row.default_width,
        default_height: row.default_height,
        default_duration_secs: row.default_duration_secs,
        min_duration_secs: row.min_duration_secs,
        max_duration_secs: row.max_duration_secs,
        duration_mode: row.duration_mode.clone(),
        supported_durations_json: row.supported_durations_json.clone(),
        default_fps: row.default_fps,
        provider_options_json: row.provider_options_json.clone(),
    }
}

pub(super) fn apply_video_params_to_row(row: &mut VideoRow, params: VideoConfigParams) {
    row.name = params.name;
    row.backend = params.backend;
    row.base_url = Some(params.base_url);
    row.api_key_encrypted = params.api_key_encrypted;
    row.model = params.model;
    row.max_concurrent = params.max_concurrent;
    row.default_width = params.default_width;
    row.default_height = params.default_height;
    row.default_duration_secs = params.default_duration_secs;
    row.min_duration_secs = params.min_duration_secs;
    row.max_duration_secs = params.max_duration_secs;
    row.duration_mode = params.duration_mode;
    row.supported_durations_json = params.supported_durations_json;
    row.default_fps = params.default_fps;
    row.provider_options_json = params.provider_options_json;
}
