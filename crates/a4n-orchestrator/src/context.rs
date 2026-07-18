use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, RwLock};

use a4n_capabilities::A4nCapabilities;
use a4n_db::DbHandle;
use a4n_domain::{ProjectId, UserId};
use a4n_rag_core::vector_store::VectorStore;
use a4n_service_manager::ServiceManager;
use serde::{Deserialize, Serialize};

use crate::chapter_generation::ChapterGenerationService;
use crate::task_queue::PersistentTaskQueue;

mod construction;
mod service_health;
mod unavailable_providers;

use unavailable_providers::{BootstrapUnavailableEmbedding, BootstrapUnavailableLlm};

pub type ManagedServicePidMap = Arc<RwLock<HashMap<String, u32>>>;

/// 阶段 11：宿主业务编排上下文。
///
/// 除数据库/capability/service manager 外，还承载原 AppState 中的进程内运行时字段，
/// 使 tauri/server 可只依赖本类型作为业务入口，而不再穿透 services/provider。
#[derive(Clone)]
pub struct OrchestratorContext {
    pub(crate) db: DbHandle,
    capabilities: A4nCapabilities,
    service_manager: Arc<ServiceManager>,
    service_pid_dir: Option<PathBuf>,
    managed_service_pids: Option<ManagedServicePidMap>,

    /// 应用配置（含 data_dir）。
    pub config: a4n_core::config::app_config::AppConfig,
    pub event_bus: Arc<a4n_core::event::bridge::BroadcastEventBus>,
    pub task_manager: Arc<a4n_core::event::channel::TaskManager>,
    /// 章节生成服务（进程内 generation lock，必须共享实例）。
    pub chapter: ChapterGenerationService,
    pub(crate) rag_store: Arc<dyn VectorStore>,
    pub task_queue: Arc<PersistentTaskQueue>,
    pub llm: Arc<dyn a4n_llm::provider::LlmProvider>,
    pub engine: Arc<dyn a4n_prompt::engine::PromptEngine>,
    pub embedding: Arc<dyn a4n_embedding::provider::EmbeddingProvider>,
    pub snapshot_store: Arc<a4n_db::character::snapshot_store::Sha256SnapshotStore>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct OrchestratorScope {
    pub user_id: UserId,
    pub project_id: Option<ProjectId>,
}
