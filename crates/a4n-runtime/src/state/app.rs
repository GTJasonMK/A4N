use std::{
    collections::{HashMap, HashSet},
    fs::{File, OpenOptions, TryLockError},
    future::Future,
    path::Path,
    pin::Pin,
    sync::Arc,
};

use a4n_api_contract::generation_tasks::GenerationTaskHandle;
use a4n_core::config::app_config::AppConfig;
use a4n_core::event::bridge::BroadcastEventBus as CoreEventBus;
use a4n_db::character::Sha256SnapshotStore;
use a4n_orchestrator::chapter_generation::ChapterGenerationService;
use a4n_orchestrator::task_queue::PersistentTaskQueue;
use serde_json::Value;

use super::database::{
    acquire_generation_owner_lock, run_startup_migrations, validate_runtime_config,
};
use super::profile::RuntimeProfile;
use super::prompt_engine::create_prompt_engine;
use super::rag::create_rag_store;

/// 业务字段统一在 [`a4n_orchestrator::OrchestratorContext`]；本类型仅额外持有
/// 进程生命周期相关的 managed sidecars。
#[derive(Clone)]
pub struct AppState {
    /// 业务编排唯一入口（含原 AppState 业务字段）。
    orchestrator: a4n_orchestrator::OrchestratorContext,
    /// 运行时 profile（留在 AppState，避免与 orch 循环依赖）。
    runtime_profile: RuntimeProfile,
    destructive_lifecycle: Arc<
        crate::actions::generation_tasks::destructive_lifecycle::DestructiveLifecycleCoordinator,
    >,
    _generation_owner_lock: Arc<sea_orm::DatabaseConnection>,
    /// ponytail: 本地 data directory 只允许一个写入 Host；明确支持共享写存储时再升级为 artifact 级数据库 lease。
    _data_dir_lock: Arc<File>,
    managed_sidecars: Arc<tokio::sync::Mutex<Vec<a4n_service_manager::ManagedSidecarProcess>>>,
    /// Agent worker 只由 Runtime 持有生命周期；Run 状态和执行记录仍由
    /// `AgentApplication`/数据库负责。
    agent_worker_cancel: tokio_util::sync::CancellationToken,
    agent_worker_notify: Arc<tokio::sync::Notify>,
    agent_worker_handle: Arc<tokio::sync::Mutex<Option<tokio::task::JoinHandle<()>>>>,
    /// Run admission and worker claiming share one gate so a freshly-persisted
    /// Run cannot be claimed before its process-local secret snapshot is bound.
    agent_admission: Arc<tokio::sync::Mutex<()>>,
    /// In-flight provider calls are cancelled immediately when the durable Run
    /// is cancelled or its lease can no longer be renewed.
    agent_run_cancellations: Arc<tokio::sync::Mutex<HashMap<i64, AgentRunCancellation>>>,
    /// 完整运行时配置可能含 API key，只按 Run 保存在当前进程内。
    agent_runtime_snapshots: Arc<tokio::sync::RwLock<HashMap<i64, AgentRuntimeSnapshot>>>,
    /// Host 注册的统一命令 dispatch。Agent 只通过该入口执行动态发现的
    /// manifest 工具，避免在 Agent runtime 维护第二套命令实现。
    agent_host_dispatcher: Arc<tokio::sync::RwLock<Option<AgentHostCommandDispatcher>>>,
}

pub type AgentHostCommandFuture = Pin<Box<dyn Future<Output = Result<Value, String>> + Send>>;
pub type AgentHostCommandDispatcher =
    Arc<dyn Fn(i64, String, Value) -> AgentHostCommandFuture + Send + Sync>;

#[derive(Clone)]
struct AgentRunCancellation {
    parent_run_id: Option<i64>,
    token: tokio_util::sync::CancellationToken,
}

#[derive(Clone)]
struct AgentRuntimeSnapshot {
    parent_run_id: Option<i64>,
    value: crate::runtime_config::RuntimeConfigSnapshot,
}

fn inherit_generation_runtime_context<F>(future: F) -> impl Future<Output = F::Output>
where
    F: Future,
{
    let user_id = crate::runtime_config::current_user_id();
    let snapshot = crate::runtime_config::current_snapshot();
    async move {
        match (user_id, snapshot) {
            (Some(user_id), snapshot) => {
                crate::runtime_config::request_scope(user_id, snapshot, future).await
            }
            (None, Some(snapshot)) => crate::runtime_config::scope(snapshot, future).await,
            (None, None) => future.await,
        }
    }
}

impl AppState {
    /// 启动唯一的 Agent durable-run worker；重复调用不会创建第二个 worker。
    ///
    /// Hosts must call this after installing their Agent Host-command
    /// dispatcher.  Keeping startup outside `new_with_profile` prevents a
    /// recovered Run from reaching a dynamic tool while the Host is still
    /// wiring its command registry.
    pub async fn spawn_agent_worker(&self) {
        let mut slot = self.agent_worker_handle.lock().await;
        if slot.is_some() {
            return;
        }
        let state = self.clone();
        let cancel = self.agent_worker_cancel.clone();
        let notify = self.agent_worker_notify.clone();
        *slot = Some(tokio::spawn(crate::actions::agent_runtime::run_worker(
            state, cancel, notify,
        )));
    }

    /// 通知 Agent worker 立即扫描可运行 Run。通知是 best-effort 的，worker 同时
    /// 保留周期扫描，因此进程调度抖动不会丢失已持久化的 Run。
    pub(crate) fn trigger_agent_worker(&self) {
        self.agent_worker_notify.notify_one();
    }

    pub(crate) async fn lock_agent_admission(&self) -> tokio::sync::MutexGuard<'_, ()> {
        self.agent_admission.lock().await
    }

    /// Installs the host-owned command dispatch used by dynamically discovered
    /// Agent tools. Hosts may replace the dispatcher during startup, but a
    /// missing dispatcher is surfaced as an error instead of a fake success.
    pub fn set_agent_host_dispatcher(
        &self,
        dispatcher: AgentHostCommandDispatcher,
    ) -> Result<(), a4n_core::error::AppError> {
        let slot = self.agent_host_dispatcher.clone();
        // Startup calls this synchronously before any user-visible Agent work.
        // Avoid exposing a second async initialization protocol just for one
        // callback slot.
        let mut guard = slot
            .try_write()
            .map_err(|_| a4n_core::error::AppError::InvalidState {
                current: "Agent host dispatcher 正在使用，无法替换".into(),
                operation: "注册 Agent Host command dispatch".into(),
            })?;
        *guard = Some(dispatcher);
        Ok(())
    }

    pub(crate) async fn dispatch_agent_host_command(
        &self,
        user_id: i64,
        command: String,
        args: Value,
    ) -> Result<Value, a4n_core::error::AppError> {
        let dispatcher = self.agent_host_dispatcher.read().await.clone();
        let Some(dispatcher) = dispatcher else {
            return Err(a4n_core::error::AppError::InvalidState {
                current: "当前 Host 未注册 Agent 动态命令 dispatch".into(),
                operation: format!("执行 Agent Host command {command}"),
            });
        };
        dispatcher(user_id, command, args).await.map_err(|error| {
            a4n_core::error::AppError::InvalidState {
                current: error,
                operation: "执行 Agent 动态 Host command".into(),
            }
        })
    }

    pub(crate) async fn register_agent_run_cancellation(
        &self,
        run_id: i64,
        parent_run_id: Option<i64>,
    ) -> tokio_util::sync::CancellationToken {
        let token = tokio_util::sync::CancellationToken::new();
        self.agent_run_cancellations.lock().await.insert(
            run_id,
            AgentRunCancellation {
                parent_run_id,
                token: token.clone(),
            },
        );
        token
    }

    pub(crate) async fn cancel_agent_run_tree(&self, run_id: i64) {
        let cancellations = self.agent_run_cancellations.lock().await;
        for (&active_run_id, cancellation) in cancellations.iter() {
            if is_descendant(active_run_id, run_id, |candidate| {
                cancellations
                    .get(&candidate)
                    .and_then(|run| run.parent_run_id)
            }) {
                cancellation.token.cancel();
            }
        }
    }

    pub(crate) async fn unregister_agent_run_cancellation(&self, run_id: i64) {
        self.agent_run_cancellations.lock().await.remove(&run_id);
    }

    pub(crate) async fn store_agent_runtime_snapshot(
        &self,
        run_id: i64,
        parent_run_id: Option<i64>,
        snapshot: crate::runtime_config::RuntimeConfigSnapshot,
    ) {
        self.agent_runtime_snapshots.write().await.insert(
            run_id,
            AgentRuntimeSnapshot {
                parent_run_id,
                value: snapshot,
            },
        );
    }

    pub(crate) async fn agent_runtime_snapshot(
        &self,
        run_id: i64,
    ) -> Option<crate::runtime_config::RuntimeConfigSnapshot> {
        self.agent_runtime_snapshots
            .read()
            .await
            .get(&run_id)
            .map(|entry| entry.value.clone())
    }

    /// Run 进入终态后取走并清除完整配置，避免密钥在进程内无期限驻留。
    pub(crate) async fn take_agent_runtime_snapshot(
        &self,
        run_id: i64,
    ) -> Option<crate::runtime_config::RuntimeConfigSnapshot> {
        self.agent_runtime_snapshots
            .write()
            .await
            .remove(&run_id)
            .map(|entry| entry.value)
    }

    pub(crate) async fn remove_agent_runtime_snapshot_tree(&self, run_id: i64) {
        let mut snapshots = self.agent_runtime_snapshots.write().await;
        let run_ids = snapshots
            .keys()
            .copied()
            .filter(|candidate| {
                is_descendant(*candidate, run_id, |candidate| {
                    snapshots
                        .get(&candidate)
                        .and_then(|snapshot| snapshot.parent_run_id)
                })
            })
            .collect::<Vec<_>>();
        for run_id in run_ids {
            snapshots.remove(&run_id);
        }
    }

    /// 停止 Agent worker 并等待其退出；可重复调用。
    pub async fn shutdown_agent_worker(&self) {
        self.agent_worker_cancel.cancel();
        self.agent_worker_notify.notify_waiters();
        let handle = self.agent_worker_handle.lock().await.take();
        if let Some(handle) = handle
            && let Err(error) = handle.await
        {
            tracing::warn!(error = %error, "Agent worker 退出异常");
        }
    }

    /// 在 Host 退出前停止其拥有的 managed sidecar；可重复调用。
    pub async fn shutdown_managed_sidecars(&self) -> Result<(), String> {
        self.shutdown_agent_worker().await;
        let processes = std::mem::take(&mut *self.managed_sidecars.lock().await);
        let failures = futures::future::join_all(
            processes
                .into_iter()
                .map(a4n_service_manager::ManagedSidecarProcess::stop),
        )
        .await
        .into_iter()
        .filter_map(Result::err)
        .map(|error| error.to_string())
        .collect::<Vec<_>>();

        if failures.is_empty() {
            Ok(())
        } else {
            Err(format!(
                "停止 managed sidecars 失败: {}",
                failures.join("; ")
            ))
        }
    }

    /// 数据目录（来自 AppConfig）。
    pub fn data_dir(&self) -> &Path {
        self.orchestrator.data_dir()
    }

    /// Host 窗口/路径初始化用配置视图（不暴露完整 orchestrator）。
    pub fn host_app_config(&self) -> &a4n_core::config::app_config::AppConfig {
        self.orchestrator.host_app_config()
    }

    pub(crate) fn runtime_profile(&self) -> RuntimeProfile {
        self.runtime_profile
    }

    pub(crate) fn capabilities(&self) -> &a4n_capabilities::A4nCapabilities {
        self.orchestrator.capabilities()
    }

    pub(crate) fn log_dir(&self) -> &Path {
        self.orchestrator.log_dir()
    }

    pub(crate) fn temp_dir(&self) -> &Path {
        self.orchestrator.temp_dir()
    }

    pub(crate) fn embedding_models_dir(&self) -> std::path::PathBuf {
        self.orchestrator.embedding_models_dir()
    }

    pub(crate) fn required_jwt_secret(&self) -> Result<&str, a4n_core::error::AppError> {
        self.orchestrator.required_jwt_secret()
    }

    pub fn subscribe_internal_events(
        &self,
    ) -> tokio::sync::broadcast::Receiver<a4n_core::event::bridge::InternalEvent> {
        self.orchestrator.subscribe_internal_events()
    }

    pub(crate) fn orchestrator(&self) -> &a4n_orchestrator::OrchestratorContext {
        &self.orchestrator
    }

    pub(crate) fn spawn_generation_execution<F, Fut>(
        &self,
        handle: GenerationTaskHandle,
        worker_kind: &'static str,
        worker: F,
    ) where
        F: FnOnce(GenerationTaskHandle) -> Fut,
        Fut: Future<Output = Result<(), String>> + Send + 'static,
    {
        let task_id = handle.task_id.clone();
        let attempt = handle.attempt;
        self.spawn_generation_execution_scoped(
            task_id,
            attempt,
            worker_kind,
            a4n_queue::with_llm_slot(worker(handle)),
        );
    }

    pub(crate) fn spawn_image_generation_execution<F, Fut>(
        &self,
        handle: GenerationTaskHandle,
        worker_kind: &'static str,
        worker: F,
    ) where
        F: FnOnce(GenerationTaskHandle) -> Fut,
        Fut: Future<Output = Result<(), String>> + Send + 'static,
    {
        let task_id = handle.task_id.clone();
        let attempt = handle.attempt;
        self.spawn_generation_execution_scoped(
            task_id,
            attempt,
            worker_kind,
            a4n_queue::with_image_slot(worker(handle)),
        );
    }

    pub(crate) fn spawn_video_generation_execution<F, Fut>(
        &self,
        handle: GenerationTaskHandle,
        worker_kind: &'static str,
        worker: F,
    ) where
        F: FnOnce(GenerationTaskHandle) -> Fut,
        Fut: Future<Output = Result<(), String>> + Send + 'static,
    {
        let task_id = handle.task_id.clone();
        let attempt = handle.attempt;
        self.spawn_generation_execution_scoped(
            task_id,
            attempt,
            worker_kind,
            a4n_queue::with_video_slot(worker(handle)),
        );
    }

    fn spawn_generation_execution_scoped(
        &self,
        task_id: String,
        attempt: u32,
        worker_kind: &'static str,
        future: impl Future<Output = Result<(), String>> + Send + 'static,
    ) {
        drop(tokio::spawn(inherit_generation_runtime_context(
            async move {
                if let Err(error) = future.await {
                    tracing::error!(
                        task_id = %task_id,
                        attempt,
                        worker = worker_kind,
                        error = %error,
                        "generation worker 失败"
                    );
                }
            },
        )));
    }

    pub(crate) fn destructive_lifecycle(
        &self,
    ) -> &Arc<
        crate::actions::generation_tasks::destructive_lifecycle::DestructiveLifecycleCoordinator,
    > {
        &self.destructive_lifecycle
    }

    pub(crate) fn dynamic_llm_provider_for_profile(
        &self,
    ) -> Result<crate::dynamic_llm::DynamicLlmProvider, a4n_core::error::AppError> {
        let facade = self.orchestrator.dynamic_provider_facade();
        if self.runtime_profile.uses_sidecars() {
            let client = self
                .orchestrator
                .capabilities()
                .llm
                .clone()
                .ok_or_else(|| {
                    a4n_core::error::AppError::ConfigError(
                        "LLM sidecar capability 未配置，不能回退到 embedded provider".to_string(),
                    )
                })?;
            return Ok(
                crate::dynamic_llm::DynamicLlmProvider::new_with_sidecar_client(facade, client),
            );
        }
        Ok(crate::dynamic_llm::DynamicLlmProvider::new(facade))
    }

    pub async fn new_with_profile(
        config: AppConfig,
        runtime_profile: RuntimeProfile,
    ) -> Result<Self, a4n_core::error::AppError> {
        runtime_profile.validate_build_capabilities()?;
        validate_runtime_config(&config)?;
        let data_dir_lock = Arc::new(acquire_data_dir_lock(&config.data_dir)?);

        let db = sea_orm::Database::connect(&config.database_url)
            .await
            .map_err(a4n_core::error::AppError::Database)?;
        let db = Arc::new(db);
        let generation_owner_lock =
            Arc::new(acquire_generation_owner_lock(&config.database_url).await?);

        run_startup_migrations(db.as_ref()).await?;
        let service_manager = Arc::new(
            crate::sidecar_clients::runtime_service_manager(runtime_profile, &config)
                .map_err(a4n_core::error::AppError::ConfigError)?,
        );
        let capabilities =
            crate::sidecar_clients::runtime_capabilities(runtime_profile, service_manager.as_ref())
                .map_err(a4n_core::error::AppError::ConfigError)?;
        let event_bus = Arc::new(CoreEventBus::new(config.system.event_bus_capacity));
        let task_manager = Arc::new(a4n_core::event::channel::TaskManager::new());
        let task_queue = Arc::new(PersistentTaskQueue::new(db.clone()).await?);
        let (prompt_engine_result, rag_store_result) = futures::join!(
            create_prompt_engine(db.as_ref(), &config),
            create_rag_store(runtime_profile, &capabilities),
        );
        let prompt_engine = prompt_engine_result?;
        let chapter = ChapterGenerationService::new(db.clone());
        let rag_store = rag_store_result?;
        let snapshot_store = Arc::new(Sha256SnapshotStore::new(db.clone()));
        let managed_sidecar_processes = crate::sidecar_clients::start_managed_sidecars(
            runtime_profile,
            service_manager.as_ref(),
        )
        .await
        .map_err(a4n_core::error::AppError::ConfigError)?;
        let managed_sidecars = Arc::new(tokio::sync::Mutex::new(managed_sidecar_processes));
        let orchestrator = a4n_orchestrator::OrchestratorContext::bootstrap(
            a4n_db::DbHandle::new(db.clone()),
            capabilities.clone(),
            service_manager.clone(),
            config.clone(),
            event_bus.clone(),
            task_manager.clone(),
            chapter,
            rag_store,
            task_queue.clone(),
            Arc::new(prompt_engine),
            snapshot_store,
            |provider_facade| {
                let embedding = crate::sidecar_clients::create_embedding_provider(
                    provider_facade.clone(),
                    &config,
                    runtime_profile,
                    &capabilities,
                )?;
                let llm = crate::sidecar_clients::create_llm_provider(
                    provider_facade,
                    runtime_profile,
                    &capabilities,
                )?;
                Ok((llm, embedding))
            },
        )?;
        crate::baseline_configs::ensure_baseline_model_configs(&orchestrator, &config).await?;

        let state = Self {
            orchestrator,
            runtime_profile,
            destructive_lifecycle: Arc::new(
                crate::actions::generation_tasks::destructive_lifecycle::DestructiveLifecycleCoordinator::default(),
            ),
            _generation_owner_lock: generation_owner_lock,
            _data_dir_lock: data_dir_lock,
            managed_sidecars,
            agent_worker_cancel: tokio_util::sync::CancellationToken::new(),
            agent_worker_notify: Arc::new(tokio::sync::Notify::new()),
            agent_worker_handle: Arc::new(tokio::sync::Mutex::new(None)),
            agent_admission: Arc::new(tokio::sync::Mutex::new(())),
            agent_run_cancellations: Arc::new(tokio::sync::Mutex::new(HashMap::new())),
            agent_runtime_snapshots: Arc::new(tokio::sync::RwLock::new(HashMap::new())),
            agent_host_dispatcher: Arc::new(tokio::sync::RwLock::new(None)),
        };
        state
            .destructive_lifecycle()
            .continue_pending(&state)
            .await
            .map_err(|current| a4n_core::error::AppError::InvalidState {
                current,
                operation: "恢复未完成 generation lifecycle operation".to_string(),
            })?;
        crate::actions::video_records::recover_expired_uploaded_video_sources(state.temp_dir());
        crate::actions::video_records::recover_expired_assembly_caches(&state).await;
        crate::cleanup_outbox::recover_cleanup_outbox(&state).await;
        crate::cleanup_outbox::spawn_cleanup_worker(state.clone());
        Ok(state)
    }
}

fn is_descendant<F>(mut candidate: i64, ancestor: i64, mut parent_of: F) -> bool
where
    F: FnMut(i64) -> Option<i64>,
{
    let mut seen = HashSet::new();
    loop {
        if candidate == ancestor {
            return true;
        }
        if !seen.insert(candidate) {
            return false;
        }
        let Some(parent) = parent_of(candidate) else {
            return false;
        };
        candidate = parent;
    }
}

fn acquire_data_dir_lock(data_dir: &Path) -> Result<File, a4n_core::error::AppError> {
    let path = data_dir.join(".a4n-host.lock");
    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(&path)
        .map_err(|error| {
            a4n_core::error::AppError::ConfigError(format!(
                "无法打开 data directory ownership lock {}：{error}",
                path.display()
            ))
        })?;
    match file.try_lock() {
        Ok(()) => Ok(file),
        Err(TryLockError::WouldBlock) => Err(a4n_core::error::AppError::ConfigError(format!(
            "data directory 已被另一个 A4N Host 占用：{}；同一目录只允许一个写入 Host",
            data_dir.display()
        ))),
        Err(TryLockError::Error(error)) => Err(a4n_core::error::AppError::ConfigError(format!(
            "无法锁定 data directory {}：{error}",
            data_dir.display()
        ))),
    }
}

impl std::fmt::Debug for AppState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AppState")
            .field("runtime_profile", &self.runtime_profile)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::{acquire_data_dir_lock, inherit_generation_runtime_context};

    #[test]
    fn data_dir_lock_rejects_second_owner_and_releases_on_drop() {
        let temp = tempfile::tempdir().unwrap();
        let first = acquire_data_dir_lock(temp.path()).unwrap();
        let error = acquire_data_dir_lock(temp.path()).unwrap_err();
        assert!(error.to_string().contains("另一个 A4N Host 占用"));

        drop(first);
        acquire_data_dir_lock(temp.path()).unwrap();
    }

    #[tokio::test]
    async fn generation_worker_inherits_runtime_context() {
        let snapshot = serde_json::from_value(json!({
            "schema_version": 1,
            "image": {
                "id": 1,
                "name": "request image config",
                "backend": "openai-compatible-image",
                "base_url": "https://example.test/v1",
                "api_key_encrypted": "sk-test",
                "model": "image-test",
                "max_concurrent": 1,
                "supported_sizes": [{"width": 512, "height": 512}],
                "is_default": true
            }
        }))
        .unwrap();

        crate::runtime_config::request_scope(42, Some(snapshot), async {
            let worker = tokio::spawn(inherit_generation_runtime_context(async {
                let snapshot = crate::runtime_config::current_snapshot().unwrap();
                (
                    crate::runtime_config::current_user_id(),
                    snapshot.image.unwrap().name,
                )
            }));

            assert_eq!(
                worker.await.unwrap(),
                (Some(42), "request image config".to_string())
            );
        })
        .await;
    }
}
