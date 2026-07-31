use std::path::{Path, PathBuf};
use std::sync::Arc;

use a4n_capabilities::A4nCapabilities;
use a4n_core::config::app_config::AppConfig;
use a4n_core::event::bridge::BroadcastEventBus;
use a4n_core::event::channel::TaskManager;
use a4n_db::{DbHandle, SharedDatabaseConnection, character::snapshot_store::Sha256SnapshotStore};
use a4n_embedding::provider::EmbeddingProvider;
use a4n_llm::provider::LlmProvider;
use a4n_rag_core::{memory_store::MemoryVectorStore, vector_store::VectorStore};
use a4n_service_manager::ServiceManager;
use a4n_test_support::embedding::UnavailableEmbeddingProvider;
use a4n_test_support::llm::UnavailableLlmProvider;

use crate::OrchestratorContext;
use crate::chapter_generation::ChapterGenerationService;
use crate::task_queue::PersistentTaskQueue;

const DEFAULT_LLM_ERROR: &str = "测试 OrchestratorContext 未配置 LLM 响应";
const DEFAULT_LLM_STREAM_ERROR: &str = "测试 OrchestratorContext 未配置 LLM 流响应";
const DEFAULT_EMBEDDING_ERROR: &str = "测试 OrchestratorContext 未配置 embedding 响应";

pub(crate) struct TestOrchestratorBuilder {
    db: SharedDatabaseConnection,
    capabilities: A4nCapabilities,
    config: AppConfig,
    prompt_dir: PathBuf,
    llm: Arc<dyn LlmProvider>,
    embedding: Arc<dyn EmbeddingProvider>,
}

impl TestOrchestratorBuilder {
    pub(crate) fn new(db: SharedDatabaseConnection) -> Self {
        Self {
            db,
            capabilities: A4nCapabilities::empty(),
            config: AppConfig::default(),
            prompt_dir: Path::new(env!("CARGO_MANIFEST_DIR")).join("../../prompts"),
            llm: Arc::new(UnavailableLlmProvider::new(
                DEFAULT_LLM_ERROR,
                DEFAULT_LLM_STREAM_ERROR,
            )),
            embedding: Arc::new(UnavailableEmbeddingProvider::new(DEFAULT_EMBEDDING_ERROR)),
        }
    }

    pub(crate) fn with_llm(mut self, llm: Arc<dyn LlmProvider>) -> Self {
        self.llm = llm;
        self
    }

    pub(crate) fn with_embedding(mut self, embedding: Arc<dyn EmbeddingProvider>) -> Self {
        self.embedding = embedding;
        self
    }

    pub(crate) fn with_capabilities(mut self, capabilities: A4nCapabilities) -> Self {
        self.capabilities = capabilities;
        self
    }

    pub(crate) fn with_config(mut self, config: AppConfig) -> Self {
        self.config = config;
        self
    }

    pub(crate) fn with_prompt_dir(mut self, prompt_dir: impl Into<PathBuf>) -> Self {
        self.prompt_dir = prompt_dir.into();
        self
    }

    pub(crate) async fn build(self) -> OrchestratorContext {
        let db_handle = DbHandle::new(self.db.clone());
        let event_bus = Arc::new(BroadcastEventBus::new(64));
        let task_manager = Arc::new(TaskManager::new());
        let chapter = ChapterGenerationService::new(self.db.clone());
        let rag_store: Arc<dyn VectorStore> = Arc::new(MemoryVectorStore::new());
        let task_queue = Arc::new(
            PersistentTaskQueue::new(self.db.clone())
                .await
                .expect("test task queue should initialize"),
        );
        let engine = Arc::new(
            a4n_prompt::tera_engine::TeraPromptEngine::from_directory(&self.prompt_dir)
                .expect("test prompts should load"),
        );
        let snapshot_store = Arc::new(Sha256SnapshotStore::new(self.db));

        OrchestratorContext::bootstrap(
            db_handle,
            self.capabilities,
            Arc::new(
                ServiceManager::new(Vec::new()).expect("empty service manager should initialize"),
            ),
            None,
            self.config,
            event_bus,
            task_manager,
            chapter,
            rag_store,
            task_queue,
            engine,
            snapshot_store,
        )
        .with_providers(self.llm, self.embedding)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use a4n_db::DatabaseConnection;

    #[test]
    fn builder_keeps_explicit_test_overrides() {
        let db = Arc::new(DatabaseConnection::Disconnected);
        let llm: Arc<dyn LlmProvider> = Arc::new(UnavailableLlmProvider::new(
            "configured chat failure",
            "configured stream failure",
        ));
        let embedding: Arc<dyn EmbeddingProvider> = Arc::new(UnavailableEmbeddingProvider::new(
            "configured embedding failure",
        ));
        let prompt_dir = PathBuf::from("test-prompts");
        let config = AppConfig {
            data_dir: PathBuf::from("test-data"),
            ..Default::default()
        };

        let builder = TestOrchestratorBuilder::new(db)
            .with_llm(Arc::clone(&llm))
            .with_embedding(Arc::clone(&embedding))
            .with_capabilities(A4nCapabilities::empty())
            .with_config(config)
            .with_prompt_dir(prompt_dir.clone());

        assert!(Arc::ptr_eq(&builder.llm, &llm));
        assert!(Arc::ptr_eq(&builder.embedding, &embedding));
        assert!(builder.capabilities.llm.is_none());
        assert_eq!(builder.config.data_dir, PathBuf::from("test-data"));
        assert_eq!(builder.prompt_dir, prompt_dir);
    }
}
