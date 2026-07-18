use std::sync::Arc;

use a4n_core::config::app_config::AppConfig;
use a4n_core::event::bridge::BroadcastEventBus as CoreEventBus;
use a4n_db::character::snapshot_store::Sha256SnapshotStore;
use a4n_orchestrator::chapter_generation::ChapterGenerationService;
use a4n_orchestrator::task_queue::PersistentTaskQueue;

use super::database::{run_startup_migrations, validate_runtime_config};
use super::profile::RuntimeProfile;
use super::prompt_engine::create_prompt_engine;
use super::rag::create_rag_store;

/// 阶段 11：宿主组装壳。
///
/// 业务字段统一在 [`a4n_orchestrator::OrchestratorContext`]；本类型仅额外持有
/// 进程生命周期相关的 managed sidecars，并通过 Deref 暴露编排上下文。
#[derive(Clone)]
pub struct AppState {
    /// 业务编排唯一入口（含原 AppState 业务字段）。
    pub orchestrator: a4n_orchestrator::OrchestratorContext,
    /// 运行时 profile（留在 AppState，避免与 orch 循环依赖）。
    pub runtime_profile: RuntimeProfile,
    pub managed_sidecars: Arc<tokio::sync::Mutex<Vec<a4n_service_manager::ManagedSidecarProcess>>>,
}

impl std::ops::Deref for AppState {
    type Target = a4n_orchestrator::OrchestratorContext;

    fn deref(&self) -> &Self::Target {
        &self.orchestrator
    }
}

impl AppState {
    pub fn dynamic_llm_provider_for_profile(
        &self,
    ) -> Result<crate::dynamic_llm::DynamicLlmProvider, a4n_core::error::AppError> {
        if self.runtime_profile.uses_sidecars() {
            let client = self.capabilities().llm.clone().ok_or_else(|| {
                a4n_core::error::AppError::ConfigError(
                    "LLM sidecar capability 未配置，不能回退到 embedded provider".to_string(),
                )
            })?;
            return Ok(
                crate::dynamic_llm::DynamicLlmProvider::new_with_sidecar_client(
                    self.orchestrator.clone(),
                    self.task_queue.clone(),
                    client,
                ),
            );
        }
        Ok(crate::dynamic_llm::DynamicLlmProvider::new(
            self.orchestrator.clone(),
            self.task_queue.clone(),
        ))
    }

    pub async fn new(config: AppConfig) -> Result<Self, a4n_core::error::AppError> {
        Self::new_with_profile(config, RuntimeProfile::from_env()?).await
    }

    pub async fn new_with_profile(
        config: AppConfig,
        runtime_profile: RuntimeProfile,
    ) -> Result<Self, a4n_core::error::AppError> {
        runtime_profile.validate_build_capabilities()?;
        validate_runtime_config(&config)?;

        let db = sea_orm::Database::connect(&config.database_url)
            .await
            .map_err(a4n_core::error::AppError::Database)?;
        let db = Arc::new(db);

        run_startup_migrations(db.as_ref()).await?;
        let capabilities = crate::sidecar_clients::runtime_capabilities(runtime_profile)
            .map_err(a4n_core::error::AppError::ConfigError)?;
        let service_manager = Arc::new(
            crate::sidecar_clients::runtime_service_manager(runtime_profile, &config)
                .map_err(a4n_core::error::AppError::ConfigError)?,
        );
        let event_bus = Arc::new(CoreEventBus::new(config.system.event_bus_capacity));
        let task_manager = Arc::new(a4n_core::event::channel::TaskManager::new());
        let task_queue = Arc::new(PersistentTaskQueue::new(db.clone()).await?);
        let prompt_engine = create_prompt_engine(db.as_ref(), &config).await?;
        let chapter = ChapterGenerationService::new(db.clone());
        let rag_store = create_rag_store(&config, runtime_profile, &capabilities).await?;
        let snapshot_store = Arc::new(Sha256SnapshotStore::new(db.clone()));
        let mut orchestrator = a4n_orchestrator::OrchestratorContext::bootstrap(
            a4n_db::DbHandle::new(db.clone()),
            capabilities.clone(),
            service_manager.clone(),
            Some(config.data_dir.join("services")),
            config.clone(),
            event_bus.clone(),
            task_manager.clone(),
            chapter,
            rag_store,
            task_queue.clone(),
            Arc::new(prompt_engine),
            snapshot_store,
        );
        crate::baseline_configs::ensure_baseline_model_configs(&orchestrator, &config).await?;

        let managed_sidecar_processes = crate::sidecar_clients::start_managed_sidecars(
            runtime_profile,
            service_manager.as_ref(),
        )
        .await
        .map_err(a4n_core::error::AppError::ConfigError)?;
        let managed_sidecar_pids = Arc::new(std::sync::RwLock::new(
            managed_sidecar_processes
                .iter()
                .filter_map(|process| {
                    process
                        .id()
                        .map(|pid| (process.service_name().to_string(), pid))
                })
                .collect(),
        ));
        let managed_sidecars = Arc::new(tokio::sync::Mutex::new(managed_sidecar_processes));
        orchestrator = orchestrator.with_managed_service_pids(managed_sidecar_pids);
        let embedding = crate::sidecar_clients::create_embedding_provider(
            task_queue.clone(),
            orchestrator.clone(),
            &config,
            runtime_profile,
            &capabilities,
        )?;
        let llm = crate::sidecar_clients::create_llm_provider(
            task_queue.clone(),
            orchestrator.clone(),
            runtime_profile,
            &capabilities,
        )?;
        let orchestrator = orchestrator.with_providers(llm, embedding);

        let state = Self {
            orchestrator,
            runtime_profile,
            managed_sidecars,
        };
        crate::cleanup_outbox::recover_project_cleanup_outbox(&state).await;
        crate::cleanup_outbox::spawn_project_cleanup_worker(state.clone());
        Ok(state)
    }
}

impl std::fmt::Debug for AppState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AppState")
            .field("config", &self.orchestrator.config)
            .finish()
    }
}
