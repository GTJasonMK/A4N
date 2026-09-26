use std::path::{Path, PathBuf};
use std::sync::Arc;

use a4n_config::app_config::AppConfig;
use a4n_db::{DatabaseConnection, DbHandle, SharedDatabaseConnection};
use a4n_embedding_provider_contract::EmbeddingProvider;
use a4n_event::BroadcastEventBus;
use a4n_llm_provider_contract::provider::LlmProvider;
use a4n_prompt_client::PromptClient;
use a4n_rag_core::{memory_store::MemoryVectorStore, vector_store::VectorStore};
use a4n_service_manager::ServiceManager;
use a4n_task_runtime::channel::TaskManager;
use a4n_test_support::embedding::UnavailableEmbeddingProvider;
use a4n_test_support::llm::UnavailableLlmProvider;

use crate::OrchestratorContext;
use a4n_task_runtime::persistent::PersistentTaskQueue;

const DEFAULT_LLM_STREAM_ERROR: &str = "测试 OrchestratorContext 未配置 LLM 流响应";
const DEFAULT_EMBEDDING_ERROR: &str = "测试 OrchestratorContext 未配置 embedding 响应";

pub(crate) async fn create_test_user(db: &DatabaseConnection) -> i64 {
    a4n_db::auth::create_user(
        db,
        "orchestrator-test-user",
        "orchestrator-test-user@example.invalid",
        "test-password-123",
        a4n_db_models::models::user::role::ROLE_USER,
    )
    .await
    .expect("test user should be created")
    .id
}

pub(crate) struct TestOrchestratorBuilder {
    db: SharedDatabaseConnection,
    prompt_client: Option<Arc<dyn PromptClient>>,
    config: AppConfig,
    prompt_dir: PathBuf,
    llm: Arc<dyn LlmProvider>,
    embedding: Arc<dyn EmbeddingProvider>,
}

impl TestOrchestratorBuilder {
    pub(crate) fn new(db: SharedDatabaseConnection) -> Self {
        Self {
            db,
            prompt_client: None,
            config: AppConfig::default(),
            prompt_dir: Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../prompts"),
            llm: Arc::new(UnavailableLlmProvider::new(DEFAULT_LLM_STREAM_ERROR)),
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

    pub(crate) fn with_config(mut self, config: AppConfig) -> Self {
        self.config = config;
        self
    }

    pub(crate) fn with_prompt_dir(mut self, prompt_dir: impl Into<PathBuf>) -> Self {
        self.prompt_dir = prompt_dir.into();
        self
    }

    pub(crate) async fn build(self) -> OrchestratorContext {
        let db_connection = self.db.clone();
        let db_handle = DbHandle::new(db_connection.clone());
        let event_bus = Arc::new(BroadcastEventBus::new(64));
        let task_manager = Arc::new(TaskManager::new());
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
        let shooting_script_publication =
            crate::context::test_shooting_script_publication(db_handle.clone());

        let llm = self.llm;
        let embedding = self.embedding;
        OrchestratorContext::bootstrap(
            db_connection,
            self.prompt_client,
            Arc::new(
                ServiceManager::new(Vec::new()).expect("empty service manager should initialize"),
            ),
            self.config,
            event_bus,
            task_manager,
            rag_store,
            task_queue,
            engine,
            None,
            shooting_script_publication,
            move |_| Ok((llm, embedding)),
        )
        .expect("test providers should initialize")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use a4n_db::DatabaseConnection;

    #[test]
    fn builder_keeps_explicit_test_overrides() {
        let db = Arc::new(DatabaseConnection::Disconnected);
        let llm: Arc<dyn LlmProvider> =
            Arc::new(UnavailableLlmProvider::new("configured stream failure"));
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
            .with_config(config)
            .with_prompt_dir(prompt_dir.clone());

        assert!(Arc::ptr_eq(&builder.llm, &llm));
        assert!(Arc::ptr_eq(&builder.embedding, &embedding));
        assert!(builder.prompt_client.is_none());
        assert_eq!(builder.config.data_dir, PathBuf::from("test-data"));
        assert_eq!(builder.prompt_dir, prompt_dir);
    }
}
