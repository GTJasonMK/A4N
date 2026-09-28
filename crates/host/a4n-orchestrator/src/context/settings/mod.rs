use super::OrchestratorContext;
use crate::video::video_provider_config_from_model;
use a4n_api_contract::video::VideoProviderConfigPort;
use a4n_db::DbHandle;
use a4n_db_models::models::ai::{embedding_config, image_gen_config, llm_config, video_gen_config};
use a4n_domain::cost::{ModelUsageRecordCompletion, ModelUsageRecordStart};
use a4n_error::AppError;
use a4n_provider_contract::{ImageProviderConfig, VideoProviderConfig};

/// 生成所需配置读取与用量记录的私有 application owner。
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
