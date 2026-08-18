use a4n_core::models::ai::{embedding_config, image_gen_config, llm_config, video_gen_config};

use super::super::models::{
    embedding_model_from_runtime, image_model_from_runtime, llm_model_from_runtime,
    video_model_from_runtime,
};
use super::context::current_snapshot;

pub fn current_llm_model() -> Option<llm_config::Model> {
    current_llm_model_for_id(None)
}

pub fn current_image_model() -> Option<image_gen_config::Model> {
    current_image_model_for_id(None)
}

pub fn current_video_model() -> Option<video_gen_config::Model> {
    current_video_model_for_id(None)
}

pub fn current_embedding_model() -> Option<embedding_config::Model> {
    current_snapshot()
        .and_then(|snapshot| snapshot.embedding)
        .map(embedding_model_from_runtime)
}

pub fn current_llm_model_for_id(config_id: Option<i64>) -> Option<llm_config::Model> {
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
            .map(llm_model_from_runtime),
        None => snapshot.llm.map(llm_model_from_runtime),
    }
}

pub fn current_image_model_for_id(config_id: Option<i64>) -> Option<image_gen_config::Model> {
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
            .map(image_model_from_runtime),
        None => snapshot.image.map(image_model_from_runtime),
    }
}

pub fn current_video_model_for_id(config_id: Option<i64>) -> Option<video_gen_config::Model> {
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
            .map(video_model_from_runtime),
        None => snapshot.video.map(video_model_from_runtime),
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
