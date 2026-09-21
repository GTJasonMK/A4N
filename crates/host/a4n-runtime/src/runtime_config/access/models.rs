use a4n_provider_contract::{
    EmbeddingProviderConfig, ImageProviderConfig, LlmProviderConfig, VideoProviderConfig,
};

use super::super::models::{
    embedding_provider_config_from_runtime, image_provider_config_from_runtime,
    llm_provider_config_from_runtime, video_provider_config_from_runtime,
};
use super::context::current_snapshot;

pub fn current_video_provider_config() -> Option<VideoProviderConfig> {
    current_video_provider_config_for_id(None)
}

pub fn current_embedding_provider_config() -> Option<EmbeddingProviderConfig> {
    current_snapshot()
        .and_then(|snapshot| snapshot.embedding)
        .map(embedding_provider_config_from_runtime)
}

pub fn current_llm_provider_config() -> Option<LlmProviderConfig> {
    current_llm_provider_config_for_id(None)
}

pub fn current_llm_provider_config_for_id(config_id: Option<i64>) -> Option<LlmProviderConfig> {
    let snapshot = current_snapshot()?;
    match config_id {
        Some(id) => snapshot
            .llm
            .filter(|config| config.id == id)
            .or_else(|| {
                snapshot
                    .llm_configs
                    .into_iter()
                    .find(|config| config.id == id)
            })
            .map(llm_provider_config_from_runtime),
        None => snapshot.llm.map(llm_provider_config_from_runtime),
    }
}

pub fn current_image_provider_config_for_id(config_id: Option<i64>) -> Option<ImageProviderConfig> {
    let snapshot = current_snapshot()?;
    match config_id {
        Some(id) => snapshot
            .image
            .filter(|config| config.id == id)
            .or_else(|| {
                snapshot
                    .image_configs
                    .into_iter()
                    .find(|config| config.id == id)
            })
            .map(image_provider_config_from_runtime),
        None => snapshot.image.map(image_provider_config_from_runtime),
    }
}

pub fn current_video_provider_config_for_id(config_id: Option<i64>) -> Option<VideoProviderConfig> {
    let snapshot = current_snapshot()?;
    match config_id {
        Some(id) => snapshot
            .video
            .filter(|config| config.id == id)
            .or_else(|| {
                snapshot
                    .video_configs
                    .into_iter()
                    .find(|config| config.id == id)
            })
            .map(video_provider_config_from_runtime),
        None => snapshot.video.map(video_provider_config_from_runtime),
    }
}

pub fn current_llm_snapshot_scope_present() -> bool {
    current_snapshot()
        .map(|snapshot| snapshot.llm.is_some() || !snapshot.llm_configs.is_empty())
        .unwrap_or(false)
}

pub fn current_image_snapshot_scope_present() -> bool {
    current_snapshot()
        .map(|snapshot| snapshot.image.is_some() || !snapshot.image_configs.is_empty())
        .unwrap_or(false)
}

pub fn current_video_snapshot_scope_present() -> bool {
    current_snapshot()
        .map(|snapshot| snapshot.video.is_some() || !snapshot.video_configs.is_empty())
        .unwrap_or(false)
}
