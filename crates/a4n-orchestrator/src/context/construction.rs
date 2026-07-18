use std::path::PathBuf;
use std::sync::Arc;

use a4n_capabilities::A4nCapabilities;
use a4n_db::DbHandle;
use a4n_rag_core::vector_store::VectorStore;
use a4n_service_manager::ServiceManager;

use super::{BootstrapUnavailableEmbedding, BootstrapUnavailableLlm, OrchestratorContext};
use crate::chapter_generation::ChapterGenerationService;
use crate::task_queue::PersistentTaskQueue;

impl OrchestratorContext {
    pub fn capabilities(&self) -> &A4nCapabilities {
        &self.capabilities
    }

    /// 引导构造：仅用于初始化 LLM/embedding provider 的短暂阶段。
    /// 业务请求不得使用该状态下的占位 provider。
    pub fn bootstrap(
        db: DbHandle,
        capabilities: A4nCapabilities,
        service_manager: Arc<ServiceManager>,
        service_pid_dir: Option<PathBuf>,
        config: a4n_core::config::app_config::AppConfig,
        event_bus: Arc<a4n_core::event::bridge::BroadcastEventBus>,
        task_manager: Arc<a4n_core::event::channel::TaskManager>,
        chapter: ChapterGenerationService,
        rag_store: Arc<dyn VectorStore>,
        task_queue: Arc<PersistentTaskQueue>,
        engine: Arc<dyn a4n_prompt::engine::PromptEngine>,
        snapshot_store: Arc<a4n_db::character::snapshot_store::Sha256SnapshotStore>,
    ) -> Self {
        Self {
            db,
            capabilities,
            service_manager,
            service_pid_dir,
            managed_service_pids: None,
            config,
            event_bus,
            task_manager,
            chapter,
            rag_store,
            task_queue,
            llm: Arc::new(BootstrapUnavailableLlm),
            engine,
            embedding: Arc::new(BootstrapUnavailableEmbedding),
            snapshot_store,
        }
    }

    /// 用真实 provider 替换引导期占位实现。
    pub fn with_providers(
        mut self,
        llm: Arc<dyn a4n_llm::provider::LlmProvider>,
        embedding: Arc<dyn a4n_embedding::provider::EmbeddingProvider>,
    ) -> Self {
        self.llm = llm;
        self.embedding = embedding;
        self
    }
}
