use super::OrchestratorContext;
use crate::video::{validate_video_config_model, video_provider_config_from_model};
use a4n_api_contract::config::{
    ConfigProviderKind, CreateEmbeddingInput, CreateLlmInput, EmbeddingConfigSummary,
    ImageConfigForConnectivityTest, ImageConfigInput, ImageConfigSummary, LlmConfigSummary,
    UpdateEmbeddingInput, UpdateImageConfigInput, UpdateLlmInput, UpdateVideoConfigInput,
    VideoConfigInput, VideoConfigSummary,
};
use a4n_api_contract::video::VideoProviderConfigPort;
use a4n_config::{
    ensure_default_config_delete_allowed, image_sizes_to_json, validate_embedding_config_params,
    validate_image_config_params,
};
use a4n_db::{
    DbHandle,
    config::{
        EmbeddingConfigRecordInput, ImageConfigRecordInput, LlmConfigRecordInput,
        ProviderConfigKind, VideoConfigRecordInput, validate_embedding_config_model,
        validate_image_config_model,
    },
};
use a4n_db_models::models::{
    ai::{embedding_config, image_gen_config, llm_config, video_gen_config},
    user::theme_config,
};
use a4n_domain::cost::{ModelUsageRecordCompletion, ModelUsageRecordStart};
use a4n_domain::{
    ThemeDocument, ThemeMode, embedding::CreateEmbeddingConfigParams,
    image::CreateImageConfigParams,
};
use a4n_error::AppError;
use a4n_llm_config::{
    CreateLlmConfigInput, LlmConfigSnapshot, NormalizedLlmConfig, UpdateLlmConfigInput,
    normalize_create_llm_config_input, normalize_update_llm_config_input, validate_llm_config,
};
use a4n_provider_contract::{
    EmbeddingProviderConfig, ImageProviderConfig, LlmProviderConfig, VideoProviderConfig,
};
use a4n_video::{VideoConfigParams, validate_video_config_params};
use serde_json::Value;
use std::str::FromStr;

/// AI 配置、主题与系统设置的私有 application owner。
///
/// 与 `OrchestratorContext.config`（AppConfig 基础设施）区分：本 owner 只持有数据库句柄，
/// 负责配置记录的读写，不暴露完整连接或通用 getter。
#[derive(Clone)]
pub(super) struct ConfigApplication {
    database: DbHandle,
}

impl ConfigApplication {
    pub(super) fn new(database: DbHandle) -> Self {
        Self { database }
    }
}

mod application;
mod contracts;
mod facade;

pub(super) use contracts::helpers::*;

#[cfg(test)]
mod tests;
