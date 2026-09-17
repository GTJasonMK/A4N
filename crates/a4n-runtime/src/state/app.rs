use std::{
    collections::{HashMap, HashSet},
    fs::{File, OpenOptions, TryLockError},
    future::Future,
    ops::Deref,
    path::Path,
    pin::Pin,
    sync::Arc,
};

use a4n_api_contract::generation_tasks::GenerationTaskHandle;
use a4n_config::app_config::AppConfig;
use a4n_db::character::Sha256SnapshotStore;
use a4n_event::{BroadcastEventBus, InternalEvent};
use a4n_task_runtime::persistent::PersistentTaskQueue;
use a4n_worker_contract::{
    AGENT_HOST_COMMAND_OPERATION, AgentHostCommandPayload, AgentRunPayload, WorkerRequest,
};
use serde_json::Value;

use super::application_ports::ApplicationPorts;
use super::database::{
    acquire_generation_owner_lock, run_startup_migrations, validate_runtime_config,
};
use super::profile::{RuntimeProfile, RuntimeRole};
use super::prompt_engine::create_prompt_engine;
use super::rag::create_rag_store;
use crate::sidecar_clients::{
    RuntimeCapabilities, pure_transform_typed_service_request, typed_response_payload,
};

/// Runtime owns Host lifecycle state and receives business behavior through
/// explicit application ports.
#[derive(Clone)]
pub struct AppState {
    /// Runtime-owned configuration view used by Host paths and adapters.
    config: AppConfig,
    /// Process-local task handles and snapshots are owned by a4n-task-runtime.
    task_manager: Arc<a4n_task_runtime::channel::TaskManager>,
    /// Managed service health queries stay with the service manager owner.
    service_health_application:
        Arc<dyn a4n_api_contract::service_health::ServiceHealthApplicationPort>,
    /// Standalone queue cancellation boundary for tasks without a workflow.
    task_cancellation_application: Arc<dyn a4n_api_contract::task::TaskCancellationPort>,
    /// Host-facing scheduler statistics boundary.
    task_queue_stats_application: Arc<dyn a4n_api_contract::task::TaskQueueStatsApplicationPort>,
    application_ports: ApplicationPorts,
    /// Runtime-owned capability registry used by adapters and lifecycle code.
    capabilities: RuntimeCapabilities,
    /// Runtime-owned event bus used by Host bridges.
    event_bus: Arc<BroadcastEventBus>,
    /// Runtime provider port captured during bootstrap; provider execution does
    /// not reach back into the orchestrator facade.
    provider_runtime: crate::provider_runtime::DynamicProviderFacade,
    /// RAG 搜索 application port；搜索 action 不得依赖完整 orchestrator。
    rag_search: Arc<dyn a4n_rag_contract::RagSearchPort>,
    /// RAG ingest application port；Runtime ingest 不得依赖完整 orchestrator。
    rag_ingest: Arc<dyn a4n_rag_contract::RagIngestPort>,
    /// 与 orchestrator 共用的 managed service owner；Runtime 只在生命周期边界
    /// 启停进程，不复制包归属或准入策略。
    service_manager: Arc<a4n_service_manager::ServiceManager>,
    /// 运行时 profile（留在 AppState，避免与 orch 循环依赖）。
    runtime_profile: RuntimeProfile,
    destructive_lifecycle: Arc<
        crate::actions::generation_tasks::destructive_lifecycle::DestructiveLifecycleCoordinator,
    >,
    _generation_owner_lock: Option<Arc<sea_orm::DatabaseConnection>>,
    /// ponytail: 本地 data directory 只允许一个写入 Host；明确支持共享写存储时再升级为 artifact 级数据库 lease。
    _data_dir_lock: Option<Arc<File>>,
    managed_sidecars: Arc<tokio::sync::Mutex<Vec<a4n_service_manager::ManagedSidecarProcess>>>,
    /// Serialize package/startup/Host-shutdown transitions so a temporarily
    /// detached process handle is never mistaken for an unowned service.
    managed_sidecar_lifecycle: Arc<tokio::sync::Mutex<()>>,
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

impl Deref for AppState {
    type Target = ApplicationPorts;

    fn deref(&self) -> &Self::Target {
        &self.application_ports
    }
}

pub type AgentHostCommandFuture = Pin<Box<dyn Future<Output = Result<Value, String>> + Send>>;
pub type AgentHostCommandDispatcher = Arc<
    dyn Fn(
            i64,
            String,
            AgentHostCommandPayload,
            WorkerRequest<AgentHostCommandPayload>,
        ) -> AgentHostCommandFuture
        + Send
        + Sync,
>;

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

impl AppState {
    /// 启动唯一的 Agent durable-run worker；重复调用不会创建第二个 worker。
    ///
    /// Hosts must call this after installing their Agent Host-command
    /// dispatcher. Keeping startup outside `RuntimeBootstrap::finish` prevents a
    /// recovered Run from reaching a dynamic tool while the Host is still
    /// wiring its command registry.
    pub async fn spawn_agent_worker(&self) -> Result<(), a4n_error::AppError> {
        if self.agent_host_dispatcher.read().await.is_none() {
            return Err(a4n_error::AppError::InvalidState {
                current: "当前 Host 未注册 Agent 动态命令 dispatch".into(),
                operation: "启动 Agent worker".into(),
            });
        }
        let mut slot = self.agent_worker_handle.lock().await;
        if slot.is_some() {
            return Ok(());
        }
        let state = self.clone();
        let cancel = self.agent_worker_cancel.clone();
        let notify = self.agent_worker_notify.clone();
        *slot = Some(tokio::spawn(crate::actions::agent_runtime::run_worker(
            state, cancel, notify,
        )));
        Ok(())
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
    ) -> Result<(), a4n_error::AppError> {
        let slot = self.agent_host_dispatcher.clone();
        // Startup calls this synchronously before any user-visible Agent work.
        // Avoid exposing a second async initialization protocol just for one
        // callback slot.
        let mut guard = slot
            .try_write()
            .map_err(|_| a4n_error::AppError::InvalidState {
                current: "Agent host dispatcher 正在使用，无法替换".into(),
                operation: "注册 Agent Host command dispatch".into(),
            })?;
        *guard = Some(dispatcher);
        Ok(())
    }

    pub(crate) async fn dispatch_agent_host_command(
        &self,
        user_id: i64,
        scope: &a4n_api_contract::agent::AgentScope,
        command: String,
        args: Value,
        worker_request: &WorkerRequest<AgentRunPayload>,
        execution_id: Option<i64>,
        execution: Option<&a4n_agent_runtime_contract::AgentExecutionSnapshot>,
    ) -> Result<Value, a4n_error::AppError> {
        let run_id = worker_request.payload.run_id;
        let runtime_config_snapshot = self
            .agent_runtime_snapshot(run_id)
            .await
            .map(serde_json::to_value)
            .transpose()?;
        let generation_start_request_id = if command == "send_content_optimization_message" {
            execution
                .and_then(|execution| execution.resource_preconditions_json.as_ref())
                .and_then(|preconditions| preconditions.get("generation_start_request_id"))
                .and_then(Value::as_str)
                .map(str::to_owned)
        } else {
            None
        };
        let payload = AgentHostCommandPayload {
            run_id,
            scope: scope.clone(),
            execution_id,
            command: command.clone(),
            arguments: args,
            generation_start_request_id,
            runtime_config_snapshot,
        };
        payload
            .validate()
            .map_err(|error| a4n_error::AppError::InvalidState {
                current: format!("Agent Host command payload 无效: {error}"),
                operation: "构造 Agent Host command request".into(),
            })?;
        let host_key = execution_id
            .map(|id| format!("execution:{id}"))
            .unwrap_or_else(|| format!("command:{command}"));
        let request_id = format!("{}:host:{host_key}", worker_request.request_id);
        let request = WorkerRequest {
            schema_version: worker_request.schema_version,
            request_id: request_id.clone(),
            trace_id: worker_request.trace_id.clone(),
            principal: worker_request.principal.clone(),
            operation_id: AGENT_HOST_COMMAND_OPERATION.into(),
            idempotency_key: format!("{}:host:{host_key}", worker_request.idempotency_key),
            project_id: worker_request.project_id,
            root_project_id: worker_request.root_project_id,
            target_revision: worker_request.target_revision,
            runtime_config_snapshot_ref: worker_request.runtime_config_snapshot_ref.clone(),
            lease: worker_request.lease.clone(),
            payload: payload.clone(),
        };
        request
            .validate()
            .map_err(|error| a4n_error::AppError::InvalidState {
                current: format!("Agent Host command request 无效: {error}"),
                operation: "构造 Agent Host command request".into(),
            })?;
        let dispatcher = self.agent_host_dispatcher.read().await.clone();
        let Some(dispatcher) = dispatcher else {
            return Err(a4n_error::AppError::InvalidState {
                current: "当前 Host 未注册 Agent 动态命令 dispatch".into(),
                operation: format!("执行 Agent Host command {command}"),
            });
        };
        dispatcher(user_id, command, payload, request)
            .await
            .map_err(|error| a4n_error::AppError::InvalidState {
                current: error,
                operation: "执行 Agent 动态 Host command".into(),
            })
    }

    /// Authorizes a Host command at the durable lease boundary.
    pub async fn authorize_agent_host_command(
        &self,
        request: &WorkerRequest<AgentHostCommandPayload>,
    ) -> Result<i64, a4n_error::AppError> {
        self.agent_worker_application
            .authorize_worker_host_command(request)
            .await
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
        let _lifecycle = self.managed_sidecar_lifecycle.lock().await;
        self.shutdown_agent_worker().await;
        let processes = std::mem::take(&mut *self.managed_sidecars.lock().await);
        let results =
            futures::future::join_all(processes.into_iter().map(|mut process| async move {
                let result = process.stop_in_place().await;
                (process, result)
            }))
            .await;
        let mut failed_processes = Vec::new();
        let mut failures = Vec::new();
        for (process, result) in results {
            if let Err(error) = result {
                failures.push(error.to_string());
                failed_processes.push(process);
            }
        }
        if !failed_processes.is_empty() {
            self.managed_sidecars.lock().await.extend(failed_processes);
        }

        if failures.is_empty() {
            Ok(())
        } else {
            Err(format!(
                "停止 managed sidecars 失败: {}",
                failures.join("; ")
            ))
        }
    }

    pub async fn recover_cleanup_outbox(&self) {
        crate::cleanup_outbox::recover_cleanup_outbox(self).await;
    }

    pub(crate) async fn stop_managed_sidecars_for_services(
        &self,
        service_names: &[String],
    ) -> Result<(), String> {
        let _lifecycle = self.managed_sidecar_lifecycle.lock().await;
        if service_names.is_empty() {
            return Ok(());
        }
        let processes = {
            let mut managed = self.managed_sidecars.lock().await;
            let mut selected = Vec::new();
            let mut retained = Vec::with_capacity(managed.len());
            for process in std::mem::take(&mut *managed) {
                if service_names
                    .iter()
                    .any(|service| service == process.service_name())
                {
                    selected.push(process);
                } else {
                    retained.push(process);
                }
            }
            *managed = retained;
            selected
        };
        let mut failed_processes = Vec::new();
        let mut failures = Vec::new();
        for mut process in processes {
            if let Err(error) = process.stop_in_place().await {
                failures.push(error.to_string());
                failed_processes.push(process);
            }
        }
        if !failed_processes.is_empty() {
            self.managed_sidecars.lock().await.extend(failed_processes);
        }
        if failures.is_empty() {
            Ok(())
        } else {
            Err(format!(
                "停止功能包 managed sidecars 失败: {}",
                failures.join("; ")
            ))
        }
    }

    pub(crate) async fn start_managed_sidecars_for_services(
        &self,
        service_names: &[String],
    ) -> Result<(), String> {
        let _lifecycle = self.managed_sidecar_lifecycle.lock().await;
        let mut started = Vec::new();
        for service_name in service_names {
            let config = self
                .service_manager
                .config(service_name)
                .map_err(|error| error.to_string())?;
            if config.mode != a4n_service_manager::ServiceMode::Sidecar {
                continue;
            }
            let status = self
                .service_manager
                .process_status(service_name)
                .await
                .map_err(|error| error.to_string())?;
            let owner_exists = self
                .managed_sidecars
                .lock()
                .await
                .iter()
                .any(|process| process.service_name() == service_name);
            match status.as_ref().map(|status| status.state) {
                None
                | Some(
                    a4n_service_manager::ManagedSidecarState::Stopped
                    | a4n_service_manager::ManagedSidecarState::CircuitOpen,
                ) => {}
                Some(
                    a4n_service_manager::ManagedSidecarState::Starting
                    | a4n_service_manager::ManagedSidecarState::Running
                    | a4n_service_manager::ManagedSidecarState::BackingOff,
                ) if owner_exists => continue,
                Some(a4n_service_manager::ManagedSidecarState::Stopping) => {
                    return Err(format!("service {service_name} 正在停止，不能并发启动"));
                }
                Some(_) => {
                    return Err(format!(
                        "service {service_name} 已处于 managed 状态但 Host owner 缺失"
                    ));
                }
            }

            // A crashed supervisor can leave its finished process handle in the
            // Host collection. Remove that stale handle before replacing the
            // service-manager status entry.
            let stale = {
                let mut managed = self.managed_sidecars.lock().await;
                let mut stale = Vec::new();
                let mut retained = Vec::with_capacity(managed.len());
                for process in std::mem::take(&mut *managed) {
                    if process.service_name() == service_name {
                        stale.push(process);
                    } else {
                        retained.push(process);
                    }
                }
                *managed = retained;
                stale
            };
            drop(stale);

            match self.service_manager.start_sidecar(service_name).await {
                Ok(process) => started.push(process),
                Err(error) => {
                    // Keep already-started handles owned by Host. A retry can
                    // observe them and only start the missing service.
                    self.managed_sidecars.lock().await.extend(started);
                    return Err(format!("启动功能包 managed sidecar 失败: {error}"));
                }
            }
        }
        if !started.is_empty() {
            self.managed_sidecars.lock().await.extend(started);
        }
        Ok(())
    }

    /// 数据目录（来自 AppConfig）。
    pub fn data_dir(&self) -> &Path {
        &self.config.data_dir
    }

    pub(crate) async fn execute_media(
        &self,
        input: a4n_media_contract::MediaExecuteInput,
    ) -> Result<a4n_media_contract::MediaExecuteOutput, String> {
        if self.runtime_profile.uses_sidecars() {
            let client = self.capabilities.media.as_ref().ok_or_else(|| {
                "media sidecar capability 未配置，无法执行本地媒体处理".to_string()
            })?;
            let request = pure_transform_typed_service_request(
                a4n_media_contract::MEDIA_FFMPEG_CAPABILITY_VERSION,
                input,
                None,
                None,
            )?;
            let response = client
                .execute(request)
                .await
                .map_err(|error| error.message)?;
            return typed_response_payload(response, "media");
        }

        let data_dir = self.config.data_dir.clone();
        let temp_dir = self.config.temp_dir.clone();
        tokio::task::spawn_blocking(move || {
            let roots = a4n_media_processing_core::MediaRoots::new(&data_dir, &temp_dir)?;
            a4n_media_processing_core::execute(&roots, input)
        })
        .await
        .map_err(|error| format!("本地媒体处理任务异常终止：{error}"))?
    }

    pub(crate) fn data_media_file_ref(
        &self,
        path: &Path,
        label: &str,
    ) -> Result<a4n_media_contract::MediaFileRef, String> {
        self.media_file_ref(path, a4n_media_contract::MediaRoot::Data, label)
    }

    pub(crate) fn temp_media_file_ref(
        &self,
        path: &Path,
        label: &str,
    ) -> Result<a4n_media_contract::MediaFileRef, String> {
        self.media_file_ref(path, a4n_media_contract::MediaRoot::Temp, label)
    }

    pub(crate) async fn probe_media_video_properties(
        &self,
        path: &Path,
        root: a4n_media_contract::MediaRoot,
        label: &str,
    ) -> Result<a4n_media_processing_core::VideoProbeProperties, String> {
        let input = self.media_file_ref(path, root, label)?;
        let output = self
            .execute_media(a4n_media_contract::MediaExecuteInput::ProbeVideoProperties { input })
            .await?;
        match output {
            a4n_media_contract::MediaExecuteOutput::VideoProbeProperties {
                width,
                height,
                duration_secs,
                fps,
                codec_name,
                profile,
                has_b_frames,
            } => Ok(a4n_media_processing_core::VideoProbeProperties {
                width,
                height,
                duration_secs,
                fps,
                codec_name,
                profile,
                has_b_frames,
            }),
            output => Err(format!("media worker 返回了不匹配的输出：{output:?}")),
        }
    }

    pub(crate) async fn probe_media_has_audio(
        &self,
        path: &Path,
        root: a4n_media_contract::MediaRoot,
        label: &str,
    ) -> Result<bool, String> {
        let input = self.media_file_ref(path, root, label)?;
        let output = self
            .execute_media(a4n_media_contract::MediaExecuteInput::ProbeVideoHasAudio { input })
            .await?;
        match output {
            a4n_media_contract::MediaExecuteOutput::HasAudio { has_audio } => Ok(has_audio),
            output => Err(format!("media worker 返回了不匹配的输出：{output:?}")),
        }
    }

    pub(crate) async fn render_media_scrub_proxy(
        &self,
        input_path: &Path,
        output_path: &Path,
        include_audio: bool,
    ) -> Result<(), String> {
        let input = self.data_media_file_ref(input_path, "剪辑源视频")?;
        let output = self.temp_media_file_ref(output_path, "剪辑输出媒体")?;
        let request = a4n_media_contract::MediaExecuteInput::GenerateVideoScrubProxy {
            input,
            output,
            include_audio,
        };
        match self.execute_media(request).await? {
            a4n_media_contract::MediaExecuteOutput::Unit => Ok(()),
            output => Err(format!("media worker 返回了不匹配的输出：{output:?}")),
        }
    }

    pub(crate) async fn render_media_scrub_frames(
        &self,
        input_path: &Path,
        output_path: &Path,
    ) -> Result<(), String> {
        let input = self.data_media_file_ref(input_path, "剪辑源视频")?;
        let output = self.temp_media_file_ref(output_path, "剪辑输出媒体")?;
        let request =
            a4n_media_contract::MediaExecuteInput::GenerateVideoScrubFrames { input, output };
        match self.execute_media(request).await? {
            a4n_media_contract::MediaExecuteOutput::Unit => Ok(()),
            output => Err(format!("media worker 返回了不匹配的输出：{output:?}")),
        }
    }

    fn media_file_ref(
        &self,
        path: &Path,
        root: a4n_media_contract::MediaRoot,
        label: &str,
    ) -> Result<a4n_media_contract::MediaFileRef, String> {
        let root_path = match root {
            a4n_media_contract::MediaRoot::Data => &self.config.data_dir,
            a4n_media_contract::MediaRoot::Temp => &self.config.temp_dir,
        };
        let root_path = root_path
            .canonicalize()
            .map_err(|error| format!("无法解析{label}根目录：{error}"))?;
        let resolved = if path.exists() {
            path.canonicalize()
                .map_err(|error| format!("无法解析{label}：{error}"))?
        } else {
            let parent = path
                .parent()
                .ok_or_else(|| format!("{label}路径缺少父目录"))?
                .canonicalize()
                .map_err(|error| format!("无法解析{label}目录：{error}"))?;
            parent.join(
                path.file_name()
                    .ok_or_else(|| format!("{label}路径缺少文件名"))?,
            )
        };
        let relative = resolved
            .strip_prefix(&root_path)
            .map_err(|_| format!("{label}不在允许的本地目录内"))?;
        a4n_media_contract::MediaFileRef::new(root, relative.to_string_lossy().replace('\\', "/"))
    }

    /// Host 窗口/路径初始化用配置视图。
    pub fn host_app_config(&self) -> &a4n_config::app_config::AppConfig {
        &self.config
    }

    pub(crate) fn runtime_profile(&self) -> RuntimeProfile {
        self.runtime_profile
    }

    pub(crate) fn capabilities(&self) -> &RuntimeCapabilities {
        &self.capabilities
    }

    pub(crate) fn log_dir(&self) -> &Path {
        &self.config.log_dir
    }

    pub(crate) fn temp_dir(&self) -> &Path {
        &self.config.temp_dir
    }

    pub(crate) fn embedding_models_dir(&self) -> std::path::PathBuf {
        self.config.embedding_models_dir()
    }

    pub(crate) fn required_jwt_secret(&self) -> Result<&str, a4n_error::AppError> {
        self.config
            .security
            .jwt_secret
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .ok_or_else(|| a4n_error::AppError::InvalidState {
                current: "jwt_secret 未配置".to_string(),
                operation: "读取 JWT 密钥".to_string(),
            })
    }

    pub fn subscribe_internal_events(&self) -> tokio::sync::broadcast::Receiver<InternalEvent> {
        self.event_bus.subscribe()
    }

    pub(crate) fn auth_read_application(
        &self,
    ) -> &dyn a4n_api_contract::auth::AuthReadApplicationPort {
        self.auth_read_application.as_ref()
    }

    pub(crate) fn auth_mutation_application(
        &self,
    ) -> &dyn a4n_api_contract::auth::AuthMutationApplicationPort {
        self.auth_mutation_application.as_ref()
    }

    pub(crate) fn admin_read_application(
        &self,
    ) -> &dyn a4n_api_contract::admin::AdminReadApplicationPort {
        self.admin_read_application.as_ref()
    }

    pub(crate) fn model_usage_read_application(
        &self,
    ) -> &dyn a4n_api_contract::cost::ModelUsageReadApplicationPort {
        self.model_usage_read_application.as_ref()
    }

    pub(crate) fn admin_mutation_application(
        &self,
    ) -> &dyn a4n_api_contract::admin::AdminMutationApplicationPort {
        self.admin_mutation_application.as_ref()
    }

    pub(crate) fn project_status_application(
        &self,
    ) -> &dyn a4n_api_contract::project::ProjectStatusApplicationPort {
        self.project_status_application.as_ref()
    }

    pub(crate) fn project_collaboration_application(
        &self,
    ) -> &dyn a4n_api_contract::collaboration::ProjectCollaborationApplicationPort {
        self.project_collaboration_application.as_ref()
    }

    pub(crate) fn project_access_application(
        &self,
    ) -> &dyn a4n_api_contract::collaboration::ProjectAccessApplicationPort {
        self.project_access_application.as_ref()
    }

    pub(crate) fn project_read_application(
        &self,
    ) -> &dyn a4n_api_contract::project::ProjectReadApplicationPort {
        self.project_read_application.as_ref()
    }

    pub(crate) fn project_mutation_application(
        &self,
    ) -> &dyn a4n_api_contract::project::ProjectMutationApplicationPort {
        self.project_mutation_application.as_ref()
    }

    pub(crate) fn project_asset_graph_application(
        &self,
    ) -> &dyn a4n_api_contract::project::ProjectAssetGraphApplicationPort {
        self.project_asset_graph_application.as_ref()
    }

    pub(crate) fn project_duplication_application(
        &self,
    ) -> &dyn a4n_api_contract::project::ProjectDuplicationApplicationPort {
        self.project_duplication_application.as_ref()
    }

    pub(crate) fn project_family_transfer_application(
        &self,
    ) -> &dyn a4n_api_contract::project_transfer::ProjectFamilyTransferApplicationPort {
        self.project_family_transfer_application.as_ref()
    }

    pub(crate) fn video_studio_transfer_application(
        &self,
    ) -> &dyn a4n_api_contract::project_transfer::VideoStudioProjectTransferApplicationPort {
        self.video_studio_transfer_application.as_ref()
    }

    pub(crate) fn chapter_scene_read_application(
        &self,
    ) -> &dyn a4n_api_contract::chapter::ChapterSceneReadApplicationPort {
        self.chapter_scene_read_application.as_ref()
    }

    pub(crate) fn chapter_scene_mutation_application(
        &self,
    ) -> &dyn a4n_api_contract::chapter::ChapterSceneMutationApplicationPort {
        self.chapter_scene_mutation_application.as_ref()
    }

    pub(crate) fn chapter_scene_generation_application(
        &self,
    ) -> &dyn a4n_novel_host_contract::ChapterSceneGenerationApplicationPort {
        self.chapter_scene_generation_application.as_ref()
    }

    pub(crate) fn optimize_chat_generation_application(
        &self,
    ) -> &dyn a4n_optimize_host_contract::OptimizeChatGenerationApplicationPort {
        self.optimize_chat_generation_application.as_ref()
    }

    pub(crate) fn optimize_analysis_generation_application(
        &self,
    ) -> &dyn a4n_optimize_host_contract::OptimizeAnalysisGenerationApplicationPort {
        self.optimize_analysis_generation_application.as_ref()
    }

    pub(crate) fn optimize_patch_generation_application(
        &self,
    ) -> &dyn a4n_optimize_host_contract::OptimizePatchGenerationApplicationPort {
        self.optimize_patch_generation_application.as_ref()
    }

    pub(crate) fn optimize_session_read_application(
        &self,
    ) -> &dyn a4n_api_contract::optimize::OptimizeSessionReadApplicationPort {
        self.optimize_session_read_application.as_ref()
    }

    pub(crate) fn optimize_session_mutation_application(
        &self,
    ) -> &dyn a4n_optimize_host_contract::OptimizeSessionMutationApplicationPort {
        self.optimize_session_mutation_application.as_ref()
    }

    pub(crate) fn optimize_patch_application(
        &self,
    ) -> &dyn a4n_optimize_host_contract::OptimizePatchApplicationPort {
        self.optimize_patch_application.as_ref()
    }

    pub(crate) fn optimize_patch_preview_application(
        &self,
    ) -> &dyn a4n_optimize_host_contract::OptimizePatchPreviewApplicationPort {
        self.optimize_patch_preview_application.as_ref()
    }

    pub(crate) fn optimize_analysis_preview_application(
        &self,
    ) -> &dyn a4n_optimize_host_contract::OptimizeAnalysisPreviewApplicationPort {
        self.optimize_analysis_preview_application.as_ref()
    }

    pub(crate) fn optimize_chat_preview_application(
        &self,
    ) -> &dyn a4n_optimize_host_contract::OptimizeChatPreviewApplicationPort {
        self.optimize_chat_preview_application.as_ref()
    }

    pub(crate) fn scene_profile_sync_application(
        &self,
    ) -> &dyn a4n_scene_host_contract::SceneProfileSyncApplicationPort {
        self.scene_profile_sync_application.as_ref()
    }

    pub(crate) fn service_health_application(
        &self,
    ) -> &dyn a4n_api_contract::service_health::ServiceHealthApplicationPort {
        self.service_health_application.as_ref()
    }

    pub(crate) fn cleanup_outbox_application(
        &self,
    ) -> &dyn a4n_cleanup_contract::CleanupOutboxApplicationPort {
        self.cleanup_outbox_application.as_ref()
    }

    pub(crate) fn character_discovery_application(
        &self,
    ) -> &dyn a4n_character_host_contract::CharacterDiscoveryApplicationPort {
        self.character_discovery_application.as_ref()
    }

    pub(crate) fn character_profile_sync_application(
        &self,
    ) -> &dyn a4n_character_host_contract::CharacterProfileSyncApplicationPort {
        self.character_profile_sync_application.as_ref()
    }

    pub(crate) fn workflow_read_application(
        &self,
    ) -> &dyn a4n_api_contract::workflow::WorkflowReadApplicationPort {
        self.workflow_read_application.as_ref()
    }

    pub(crate) fn workflow_task_lookup_application(
        &self,
    ) -> &dyn a4n_api_contract::workflow::WorkflowTaskLookupApplicationPort {
        self.workflow_task_lookup_application.as_ref()
    }

    pub(crate) fn task_read_application(
        &self,
    ) -> &dyn a4n_api_contract::task::TaskReadApplicationPort {
        self.task_read_application.as_ref()
    }

    pub(crate) fn task_cancellation_application(
        &self,
    ) -> &dyn a4n_api_contract::task::TaskCancellationPort {
        self.task_cancellation_application.as_ref()
    }

    pub(crate) fn task_queue_stats_application(
        &self,
    ) -> &dyn a4n_api_contract::task::TaskQueueStatsApplicationPort {
        self.task_queue_stats_application.as_ref()
    }

    pub(crate) fn workflow_cancellation_application(
        &self,
    ) -> &dyn a4n_api_contract::workflow::WorkflowCancellationPort {
        self.workflow_cancellation_application.as_ref()
    }

    pub(crate) fn object_profile_read_application(
        &self,
    ) -> &dyn a4n_api_contract::profile::ObjectProfileReadApplicationPort {
        self.object_profile_read_application.as_ref()
    }

    pub(crate) fn object_profile_mutation_application(
        &self,
    ) -> &dyn a4n_api_contract::profile::ObjectProfileMutationApplicationPort {
        self.object_profile_mutation_application.as_ref()
    }

    pub(crate) fn scene_profile_read_application(
        &self,
    ) -> &dyn a4n_api_contract::profile::SceneProfileReadApplicationPort {
        self.scene_profile_read_application.as_ref()
    }

    pub(crate) fn character_profile_read_application(
        &self,
    ) -> &dyn a4n_api_contract::character::CharacterProfileReadApplicationPort {
        self.character_profile_read_application.as_ref()
    }

    pub(crate) fn character_profile_mutation_application(
        &self,
    ) -> &dyn a4n_api_contract::character::CharacterProfileMutationApplicationPort {
        self.character_profile_mutation_application.as_ref()
    }

    pub(crate) fn character_profile_timeline_mutation_application(
        &self,
    ) -> &dyn a4n_api_contract::character::CharacterProfileTimelineMutationApplicationPort {
        self.character_profile_timeline_mutation_application
            .as_ref()
    }

    pub(crate) fn character_profile_attribute_mutation_application(
        &self,
    ) -> &dyn a4n_api_contract::character::CharacterProfileAttributeMutationApplicationPort {
        self.character_profile_attribute_mutation_application
            .as_ref()
    }

    pub(crate) fn character_profile_attribute_deletion_application(
        &self,
    ) -> &dyn a4n_api_contract::character::CharacterProfileAttributeDeletionApplicationPort {
        self.character_profile_attribute_deletion_application
            .as_ref()
    }

    pub(crate) fn character_implicit_review_check_application(
        &self,
    ) -> &dyn a4n_character_host_contract::CharacterImplicitReviewCheckApplicationPort {
        self.character_implicit_review_check_application.as_ref()
    }

    pub(crate) fn character_implicit_review_preview_application(
        &self,
    ) -> &dyn a4n_character_host_contract::CharacterImplicitReviewPreviewApplicationPort {
        self.character_implicit_review_preview_application.as_ref()
    }

    pub(crate) fn character_implicit_review_apply_application(
        &self,
    ) -> &dyn a4n_character_host_contract::CharacterImplicitReviewApplyApplicationPort {
        self.character_implicit_review_apply_application.as_ref()
    }

    pub(crate) fn character_implicit_review_read_application(
        &self,
    ) -> &dyn a4n_api_contract::character::CharacterImplicitReviewReadApplicationPort {
        self.character_implicit_review_read_application.as_ref()
    }

    pub(crate) fn character_profile_deletion_application(
        &self,
    ) -> &dyn a4n_api_contract::character::CharacterProfileDeletionApplicationPort {
        self.character_profile_deletion_application.as_ref()
    }

    pub(crate) fn character_conflict_application(
        &self,
    ) -> &dyn a4n_character_host_contract::CharacterConflictApplicationPort {
        self.character_conflict_application.as_ref()
    }

    pub(crate) fn scene_profile_mutation_application(
        &self,
    ) -> &dyn a4n_api_contract::profile::SceneProfileMutationApplicationPort {
        self.scene_profile_mutation_application.as_ref()
    }

    pub(crate) fn foreshadowing_read_application(
        &self,
    ) -> &dyn a4n_api_contract::profile::ForeshadowingReadApplicationPort {
        self.foreshadowing_read_application.as_ref()
    }

    pub(crate) fn foreshadowing_mutation_application(
        &self,
    ) -> &dyn a4n_api_contract::profile::ForeshadowingMutationApplicationPort {
        self.foreshadowing_mutation_application.as_ref()
    }

    pub(crate) fn blueprint_read_application(
        &self,
    ) -> &dyn a4n_api_contract::blueprint::BlueprintReadApplicationPort {
        self.blueprint_read_application.as_ref()
    }

    pub(crate) fn blueprint_generation_preview_application(
        &self,
    ) -> &dyn a4n_blueprint_contract::BlueprintGenerationPreviewApplicationPort {
        self.blueprint_generation_preview_application.as_ref()
    }

    pub(crate) fn blueprint_refinement_preview_application(
        &self,
    ) -> &dyn a4n_blueprint_contract::BlueprintRefinementPreviewApplicationPort {
        self.blueprint_refinement_preview_application.as_ref()
    }

    pub(crate) fn blueprint_generation_application(
        &self,
    ) -> &dyn a4n_blueprint_contract::BlueprintGenerationApplicationPort {
        self.blueprint_generation_application.as_ref()
    }

    pub(crate) fn blueprint_refinement_application(
        &self,
    ) -> &dyn a4n_blueprint_contract::BlueprintRefinementApplicationPort {
        self.blueprint_refinement_application.as_ref()
    }

    pub(crate) fn blueprint_mutation_application(
        &self,
    ) -> &dyn a4n_api_contract::blueprint::BlueprintMutationApplicationPort {
        self.blueprint_mutation_application.as_ref()
    }

    pub(crate) fn galgame_read_application(
        &self,
    ) -> &dyn a4n_api_contract::galgame::GalgameReadApplicationPort {
        self.galgame_read_application.as_ref()
    }

    pub(crate) fn inspiration_history_read_application(
        &self,
    ) -> &dyn a4n_api_contract::inspiration::InspirationHistoryReadApplicationPort {
        self.inspiration_history_read_application.as_ref()
    }

    pub(crate) fn inspiration_generation_application(
        &self,
    ) -> &dyn a4n_inspiration_host_contract::InspirationGenerationApplicationPort {
        self.inspiration_generation_application.as_ref()
    }

    pub(crate) fn inspiration_request_application(
        &self,
    ) -> &dyn a4n_inspiration_host_contract::InspirationRequestPreviewApplicationPort {
        self.inspiration_request_application.as_ref()
    }

    pub(crate) fn video_provider_config(
        &self,
    ) -> &dyn a4n_api_contract::video::VideoProviderConfigPort {
        self.video_provider_config.as_ref()
    }

    pub(crate) fn video_provider_execution(
        &self,
    ) -> &dyn a4n_api_contract::video::VideoProviderExecutionApplicationPort {
        self.video_provider_execution.as_ref()
    }

    pub(crate) fn video_generation_application(
        &self,
    ) -> &dyn a4n_api_contract::video::VideoGenerationApplicationPort {
        self.video_generation_application.as_ref()
    }

    pub(crate) fn canvas_video_request_preview_application(
        &self,
    ) -> &dyn a4n_api_contract::video::CanvasVideoRequestPreviewApplicationPort {
        self.canvas_video_request_preview_application.as_ref()
    }

    pub(crate) fn chapter_video_request_preview_application(
        &self,
    ) -> &dyn a4n_api_contract::video::ChapterVideoRequestPreviewApplicationPort {
        self.chapter_video_request_preview_application.as_ref()
    }

    pub(crate) fn reference_media_video_request_preview_application(
        &self,
    ) -> &dyn a4n_api_contract::video::ReferenceMediaVideoRequestPreviewApplicationPort {
        self.reference_media_video_request_preview_application
            .as_ref()
    }

    pub(crate) fn chapter_content_read_application(
        &self,
    ) -> &dyn a4n_api_contract::chapter::ChapterContentReadApplicationPort {
        self.chapter_content_read_application.as_ref()
    }

    pub(crate) fn chapter_content_mutation_application(
        &self,
    ) -> &dyn a4n_api_contract::chapter::ChapterContentMutationApplicationPort {
        self.chapter_content_mutation_application.as_ref()
    }

    pub(crate) fn chapter_content_sync_application(
        &self,
    ) -> &dyn a4n_api_contract::chapter::ChapterContentSyncApplicationPort {
        self.chapter_content_sync_application.as_ref()
    }

    pub(crate) fn chapter_generation_application(
        &self,
    ) -> &dyn a4n_chapter_generation_contract::ChapterGenerationApplicationPort {
        self.chapter_generation_application.as_ref()
    }

    pub(crate) fn chapter_generation_request_preview_application(
        &self,
    ) -> &dyn a4n_chapter_generation_contract::ChapterGenerationRequestPreviewApplicationPort {
        self.chapter_generation_request_preview_application.as_ref()
    }

    pub(crate) fn chapter_generation_rag_preparation_application(
        &self,
    ) -> &dyn a4n_chapter_generation_contract::ChapterGenerationRagPreparationApplicationPort {
        self.chapter_generation_rag_preparation_application.as_ref()
    }

    pub(crate) fn outline_read_application(
        &self,
    ) -> &dyn a4n_api_contract::outline::OutlineReadApplicationPort {
        self.outline_read_application.as_ref()
    }

    pub(crate) fn outline_mutation_application(
        &self,
    ) -> &dyn a4n_api_contract::outline::OutlineMutationApplicationPort {
        self.outline_mutation_application.as_ref()
    }

    pub(crate) fn outline_generation_application(
        &self,
    ) -> &dyn a4n_outline_contract::OutlineGenerationApplicationPort {
        self.outline_generation_application.as_ref()
    }

    pub(crate) fn outline_part_generation_preview_application(
        &self,
    ) -> &dyn a4n_outline_contract::OutlinePartGenerationPreviewApplicationPort {
        self.outline_part_generation_preview_application.as_ref()
    }

    pub(crate) fn outline_part_mutation_preview_application(
        &self,
    ) -> &dyn a4n_outline_contract::OutlinePartMutationPreviewApplicationPort {
        self.outline_part_mutation_preview_application.as_ref()
    }

    pub(crate) fn outline_part_chapter_generation_preview_application(
        &self,
    ) -> &dyn a4n_outline_contract::OutlinePartChapterGenerationPreviewApplicationPort {
        self.outline_part_chapter_generation_preview_application
            .as_ref()
    }

    pub(crate) fn outline_batch_part_generation_preview_application(
        &self,
    ) -> &dyn a4n_outline_contract::OutlineBatchPartGenerationPreviewApplicationPort {
        self.outline_batch_part_generation_preview_application
            .as_ref()
    }

    pub(crate) fn outline_chapter_count_generation_preview_application(
        &self,
    ) -> &dyn a4n_outline_contract::OutlineChapterCountGenerationPreviewApplicationPort {
        self.outline_chapter_count_generation_preview_application
            .as_ref()
    }

    pub(crate) fn outline_chapter_regeneration_preview_application(
        &self,
    ) -> &dyn a4n_outline_contract::OutlineChapterRegenerationPreviewApplicationPort {
        self.outline_chapter_regeneration_preview_application
            .as_ref()
    }

    pub(crate) fn outline_beat_mapping_preview_application(
        &self,
    ) -> &dyn a4n_outline_contract::OutlineBeatMappingPreviewApplicationPort {
        self.outline_beat_mapping_preview_application.as_ref()
    }

    pub(crate) fn theme_read_application(
        &self,
    ) -> &dyn a4n_api_contract::theme::ThemeReadApplicationPort {
        self.theme_read_application.as_ref()
    }

    pub(crate) fn theme_mutation_application(
        &self,
    ) -> &dyn a4n_api_contract::theme::ThemeMutationApplicationPort {
        self.theme_mutation_application.as_ref()
    }

    pub(crate) fn config_read_application(
        &self,
    ) -> &dyn a4n_api_contract::config::ConfigReadApplicationPort {
        self.config_read_application.as_ref()
    }

    pub(crate) fn config_mutation_application(
        &self,
    ) -> &dyn a4n_api_contract::config::ConfigMutationApplicationPort {
        self.config_mutation_application.as_ref()
    }

    pub(crate) fn settings_export_application(
        &self,
    ) -> &dyn a4n_api_contract::config::SettingsExportApplicationPort {
        self.settings_export_application.as_ref()
    }

    pub(crate) fn settings_import_application(
        &self,
    ) -> &dyn a4n_api_contract::config::SettingsImportApplicationPort {
        self.settings_import_application.as_ref()
    }

    pub(crate) fn prompt_read_application(
        &self,
    ) -> &dyn a4n_api_contract::prompt::PromptReadApplicationPort {
        self.prompt_read_application.as_ref()
    }

    pub(crate) fn prompt_mutation_application(
        &self,
    ) -> &dyn a4n_api_contract::prompt::PromptMutationApplicationPort {
        self.prompt_mutation_application.as_ref()
    }

    pub(crate) fn agent_thread_mutation_application(
        &self,
    ) -> &dyn a4n_api_contract::agent::AgentThreadMutationApplicationPort {
        self.agent_thread_mutation_application.as_ref()
    }

    pub(crate) fn agent_thread_read_application(
        &self,
    ) -> &dyn a4n_api_contract::agent::AgentThreadReadApplicationPort {
        self.agent_thread_read_application.as_ref()
    }

    pub(crate) fn agent_run_mutation_application(
        &self,
    ) -> &dyn a4n_api_contract::agent::AgentRunMutationApplicationPort {
        self.agent_run_mutation_application.as_ref()
    }

    pub(crate) fn agent_run_read_application(
        &self,
    ) -> &dyn a4n_api_contract::agent::AgentRunReadApplicationPort {
        self.agent_run_read_application.as_ref()
    }

    pub(crate) fn agent_delegation_application(
        &self,
    ) -> &dyn a4n_api_contract::agent::AgentDelegationApplicationPort {
        self.agent_delegation_application.as_ref()
    }

    pub(crate) fn agent_tool_read_application(
        &self,
    ) -> &dyn a4n_api_contract::agent::AgentToolReadApplicationPort {
        self.agent_tool_read_application.as_ref()
    }

    pub(crate) fn agent_skill_application(
        &self,
    ) -> &dyn a4n_api_contract::agent::AgentSkillApplicationPort {
        self.agent_skill_application.as_ref()
    }

    pub(crate) fn agent_event_read_application(
        &self,
    ) -> &dyn a4n_api_contract::agent::AgentEventReadApplicationPort {
        self.agent_event_read_application.as_ref()
    }

    pub(crate) fn agent_memory_read_application(
        &self,
    ) -> &dyn a4n_api_contract::agent::AgentMemoryReadApplicationPort {
        self.agent_memory_read_application.as_ref()
    }

    pub(crate) fn agent_memory_mutation_application(
        &self,
    ) -> &dyn a4n_api_contract::agent::AgentMemoryMutationApplicationPort {
        self.agent_memory_mutation_application.as_ref()
    }

    pub(crate) fn agent_approval_application(
        &self,
    ) -> &dyn a4n_api_contract::agent::AgentApprovalApplicationPort {
        self.agent_approval_application.as_ref()
    }

    pub(crate) fn agent_resource_access_application(
        &self,
    ) -> &dyn a4n_api_contract::agent::AgentResourceAccessApplicationPort {
        self.agent_resource_access_application.as_ref()
    }

    pub(crate) fn agent_external_task_wait_application(
        &self,
    ) -> &dyn a4n_agent_runtime_contract::AgentExternalTaskWaitApplicationPort {
        self.agent_worker_application.as_ref()
    }

    pub(crate) fn agent_editor_wait_application(
        &self,
    ) -> &dyn a4n_agent_runtime_contract::AgentEditorWaitApplicationPort {
        self.agent_worker_application.as_ref()
    }

    pub(crate) fn agent_editor_claim_application(
        &self,
    ) -> &dyn a4n_agent_runtime_contract::AgentEditorClaimApplicationPort {
        self.agent_worker_application.as_ref()
    }

    pub(crate) fn agent_generation_cancel_application(
        &self,
    ) -> &dyn a4n_agent_runtime_contract::AgentGenerationCancelApplicationPort {
        self.agent_worker_application.as_ref()
    }

    pub(crate) fn agent_workflow_cancel_application(
        &self,
    ) -> &dyn a4n_agent_runtime_contract::AgentWorkflowCancelApplicationPort {
        self.agent_worker_application.as_ref()
    }

    pub(crate) fn agent_worker_budget_application(
        &self,
    ) -> &dyn a4n_agent_runtime_contract::AgentWorkerBudgetApplicationPort {
        self.agent_worker_application.as_ref()
    }

    pub(crate) fn agent_approval_expiry_application(
        &self,
    ) -> &dyn a4n_agent_runtime_contract::AgentApprovalExpiryApplicationPort {
        self.agent_worker_application.as_ref()
    }

    pub(crate) fn agent_run_lease_recovery_application(
        &self,
    ) -> &dyn a4n_agent_runtime_contract::AgentRunLeaseRecoveryApplicationPort {
        self.agent_worker_application.as_ref()
    }

    pub(crate) fn agent_worker_lease_application(
        &self,
    ) -> Arc<dyn a4n_agent_runtime_contract::AgentWorkerLeaseApplicationPort> {
        self.agent_worker_application.clone()
    }

    pub(crate) fn agent_worker_claim_application(
        &self,
    ) -> &dyn a4n_agent_runtime_contract::AgentWorkerClaimApplicationPort {
        self.agent_worker_application.as_ref()
    }

    pub(crate) fn agent_worker_failure_application(
        &self,
    ) -> Arc<dyn a4n_agent_runtime_contract::AgentWorkerFailureApplicationPort> {
        self.agent_worker_application.clone()
    }

    pub(crate) fn agent_worker_wait_application(
        &self,
    ) -> &dyn a4n_agent_runtime_contract::AgentWorkerWaitApplicationPort {
        self.agent_worker_application.as_ref()
    }

    pub(crate) fn agent_worker_read_application(
        &self,
    ) -> &dyn a4n_agent_runtime_contract::AgentWorkerReadApplicationPort {
        self.agent_worker_application.as_ref()
    }

    pub(crate) fn agent_worker_context_application(
        &self,
    ) -> &dyn a4n_agent_runtime_contract::AgentWorkerContextApplicationPort {
        self.agent_worker_application.as_ref()
    }

    pub(crate) fn agent_worker_execution_application(
        &self,
    ) -> &dyn a4n_agent_runtime_contract::AgentWorkerExecutionApplicationPort {
        self.agent_worker_application.as_ref()
    }

    pub(crate) fn agent_worker_approval_application(
        &self,
    ) -> &dyn a4n_agent_runtime_contract::AgentWorkerApprovalApplicationPort {
        self.agent_worker_application.as_ref()
    }

    pub(crate) fn agent_worker_completion_application(
        &self,
    ) -> &dyn a4n_agent_runtime_contract::AgentWorkerCompletionApplicationPort {
        self.agent_worker_application.as_ref()
    }

    pub(crate) fn agent_worker_delegation_application(
        &self,
    ) -> &dyn a4n_agent_runtime_contract::AgentWorkerDelegationApplicationPort {
        self.agent_worker_application.as_ref()
    }

    pub(crate) fn agent_worker_history_application(
        &self,
    ) -> &dyn a4n_agent_runtime_contract::AgentWorkerHistoryApplicationPort {
        self.agent_worker_application.as_ref()
    }

    pub(crate) fn agent_worker_inbox_application(
        &self,
    ) -> &dyn a4n_agent_runtime_contract::AgentWorkerInboxApplicationPort {
        self.agent_worker_application.as_ref()
    }

    pub(crate) fn agent_worker_message_application(
        &self,
    ) -> &dyn a4n_agent_runtime_contract::AgentWorkerMessageApplicationPort {
        self.agent_worker_application.as_ref()
    }

    pub(crate) fn agent_worker_usage_application(
        &self,
    ) -> &dyn a4n_agent_runtime_contract::AgentWorkerUsageApplicationPort {
        self.agent_worker_application.as_ref()
    }

    pub(crate) fn agent_editor_response_application(
        &self,
    ) -> &dyn a4n_agent_runtime_contract::AgentEditorResponseApplicationPort {
        self.agent_worker_application.as_ref()
    }

    pub(crate) fn agent_worker_recovery_application(
        &self,
    ) -> &dyn a4n_agent_runtime_contract::AgentWorkerRecoveryApplicationPort {
        self.agent_worker_application.as_ref()
    }

    pub(crate) fn agent_worker_activation_application(
        &self,
    ) -> &dyn a4n_agent_runtime_contract::AgentWorkerActivationApplicationPort {
        self.agent_worker_application.as_ref()
    }

    pub(crate) fn agent_workflow_binding_application(
        &self,
    ) -> &dyn a4n_agent_runtime_contract::AgentWorkflowBindingApplicationPort {
        self.agent_worker_application.as_ref()
    }

    pub(crate) fn canvas_read_application(
        &self,
    ) -> &dyn a4n_api_contract::canvas::CanvasReadApplicationPort {
        self.canvas_read_application.as_ref()
    }

    pub(crate) fn canvas_mutation_application(
        &self,
    ) -> &dyn a4n_api_contract::canvas::CanvasMutationApplicationPort {
        self.canvas_mutation_application.as_ref()
    }

    pub(crate) fn canvas_render_text_application(
        &self,
    ) -> &dyn a4n_canvas_contract::CanvasRenderTextNodeApplicationPort {
        self.canvas_render_text_application.as_ref()
    }

    pub(crate) fn canvas_llm_text_application(
        &self,
    ) -> &dyn a4n_canvas_host_contract::CanvasLlmTextNodeApplicationPort {
        self.canvas_llm_text_application.as_ref()
    }

    pub(crate) fn canvas_image_application(
        &self,
    ) -> &dyn a4n_canvas_host_contract::CanvasImageNodeApplicationPort {
        self.canvas_image_application.as_ref()
    }

    pub(crate) fn canvas_video_application(
        &self,
    ) -> &dyn a4n_canvas_host_contract::CanvasVideoNodeApplicationPort {
        self.canvas_video_application.as_ref()
    }

    pub(crate) fn canvas_preview_application(
        &self,
    ) -> &dyn a4n_canvas_host_contract::CanvasPreviewApplicationPort {
        self.canvas_preview_application.as_ref()
    }

    pub(crate) fn canvas_editing_application(
        &self,
    ) -> &dyn a4n_canvas_contract::CanvasEditingNodeApplicationPort {
        self.canvas_editing_application.as_ref()
    }

    pub(crate) fn canvas_probe_application(
        &self,
    ) -> &dyn a4n_canvas_contract::CanvasProbeNodeApplicationPort {
        self.canvas_probe_application.as_ref()
    }

    pub(crate) fn canvas_output_application(
        &self,
    ) -> &dyn a4n_canvas_contract::CanvasOutputNodeApplicationPort {
        self.canvas_output_application.as_ref()
    }

    pub(crate) fn canvas_image_import_application(
        &self,
    ) -> &dyn a4n_canvas_contract::CanvasImageImportApplicationPort {
        self.canvas_image_import_application.as_ref()
    }

    pub(crate) fn canvas_video_import_application(
        &self,
    ) -> &dyn a4n_canvas_contract::CanvasVideoImportApplicationPort {
        self.canvas_video_import_application.as_ref()
    }

    pub(crate) fn canvas_audio_import_application(
        &self,
    ) -> &dyn a4n_canvas_contract::CanvasAudioImportApplicationPort {
        self.canvas_audio_import_application.as_ref()
    }

    pub(crate) fn canvas_graph_preparation_application(
        &self,
    ) -> &dyn a4n_canvas_contract::CanvasGraphPreparationApplicationPort {
        self.canvas_graph_preparation_application.as_ref()
    }

    pub(crate) fn canvas_graph_execution_application(
        &self,
    ) -> &dyn a4n_canvas_contract::CanvasGraphExecutionApplicationPort {
        self.canvas_graph_execution_application.as_ref()
    }

    pub(crate) fn canvas_graph_failure_application(
        &self,
    ) -> &dyn a4n_canvas_contract::CanvasGraphFailureApplicationPort {
        self.canvas_graph_failure_application.as_ref()
    }

    pub(crate) fn package_read_application(
        &self,
    ) -> &dyn a4n_api_contract::package::PackageReadApplicationPort {
        self.package_read_application.as_ref()
    }

    pub(crate) fn package_mutation_application(
        &self,
    ) -> &dyn a4n_api_contract::package::PackageMutationApplicationPort {
        self.package_mutation_application.as_ref()
    }

    pub(crate) fn import_analysis_generation_application(
        &self,
    ) -> &dyn a4n_import_analysis_host_contract::ImportAnalysisGenerationApplicationPort {
        self.import_analysis_generation_application.as_ref()
    }

    pub(crate) fn import_analysis_cancellation_application(
        &self,
    ) -> &dyn a4n_api_contract::import::ImportAnalysisCancellationApplicationPort {
        self.import_analysis_cancellation_application.as_ref()
    }

    pub(crate) fn import_analysis_progress_application(
        &self,
    ) -> &dyn a4n_api_contract::import::ImportAnalysisProgressApplicationPort {
        self.import_analysis_progress_application.as_ref()
    }

    pub(crate) fn import_analysis_preview_application(
        &self,
    ) -> &dyn a4n_import_analysis_host_contract::ImportAnalysisPreviewApplicationPort {
        self.import_analysis_preview_application.as_ref()
    }

    pub(crate) fn import_reverse_preview_application(
        &self,
    ) -> &dyn a4n_novel_host_contract::ImportReversePreviewApplicationPort {
        self.import_reverse_preview_application.as_ref()
    }

    pub(crate) fn import_reverse_generation_application(
        &self,
    ) -> &dyn a4n_novel_host_contract::ImportReverseGenerationApplicationPort {
        self.import_reverse_generation_application.as_ref()
    }

    pub(crate) fn import_reverse_confirmation_application(
        &self,
    ) -> &dyn a4n_api_contract::import_reverse::ImportReverseConfirmationApplicationPort {
        self.import_reverse_confirmation_application.as_ref()
    }

    pub(crate) fn import_reverse_read_application(
        &self,
    ) -> &dyn a4n_api_contract::import_reverse::ImportReverseReadApplicationPort {
        self.import_reverse_read_application.as_ref()
    }

    pub(crate) fn chapter_analysis_generation_application(
        &self,
    ) -> &dyn a4n_chapter_analysis_host_contract::ChapterAnalysisGenerationApplicationPort {
        self.chapter_analysis_generation_application.as_ref()
    }

    pub(crate) fn chapter_analysis_resume_application(
        &self,
    ) -> &dyn a4n_chapter_analysis_host_contract::ChapterAnalysisResumeApplicationPort {
        self.chapter_analysis_resume_application.as_ref()
    }

    pub(crate) fn chapter_analysis_generation_preview_application(
        &self,
    ) -> &dyn a4n_chapter_analysis_host_contract::ChapterAnalysisGenerationPreviewApplicationPort
    {
        self.chapter_analysis_generation_preview_application
            .as_ref()
    }

    pub(crate) fn chapter_analysis_resume_preview_application(
        &self,
    ) -> &dyn a4n_chapter_analysis_host_contract::ChapterAnalysisResumePreviewApplicationPort {
        self.chapter_analysis_resume_preview_application.as_ref()
    }

    pub(crate) fn import_review_ingestion_application(
        &self,
    ) -> &dyn a4n_novel_host_contract::ImportReviewIngestionApplicationPort {
        self.import_review_ingestion_application.as_ref()
    }

    pub(crate) fn import_review_acceptance_application(
        &self,
    ) -> &dyn a4n_api_contract::import_review::ImportReviewAcceptanceApplicationPort {
        self.import_review_acceptance_application.as_ref()
    }

    pub(crate) fn import_review_mutation_application(
        &self,
    ) -> &dyn a4n_api_contract::import_review::ImportReviewMutationApplicationPort {
        self.import_review_mutation_application.as_ref()
    }

    pub(crate) fn import_review_read_application(
        &self,
    ) -> &dyn a4n_api_contract::import_review::ImportReviewReadApplicationPort {
        self.import_review_read_application.as_ref()
    }

    pub(crate) fn project_export_application(
        &self,
    ) -> &dyn a4n_api_contract::export::ProjectExportApplicationPort {
        self.project_export_application.as_ref()
    }

    pub(crate) fn galgame_export_application(
        &self,
    ) -> &dyn a4n_api_contract::export::GalgameExportApplicationPort {
        self.galgame_export_application.as_ref()
    }

    pub(crate) fn video_record_read_application(
        &self,
    ) -> &dyn a4n_api_contract::video::VideoRecordReadApplicationPort {
        self.video_record_read_application.as_ref()
    }

    pub(crate) fn video_record_mutation_application(
        &self,
    ) -> &dyn a4n_api_contract::video::VideoRecordMutationApplicationPort {
        self.video_record_mutation_application.as_ref()
    }

    pub(crate) fn video_chapter_import_application(
        &self,
    ) -> &dyn a4n_api_contract::video::ChapterVideoImportApplicationPort {
        self.video_chapter_import_application.as_ref()
    }

    pub(crate) fn video_reference_media_import_application(
        &self,
    ) -> &dyn a4n_api_contract::video::ReferenceMediaVideoImportApplicationPort {
        self.video_reference_media_import_application.as_ref()
    }

    pub(crate) fn video_assembly_import_application(
        &self,
    ) -> &dyn a4n_api_contract::video::AssemblyVideoImportApplicationPort {
        self.video_assembly_import_application.as_ref()
    }

    pub(crate) fn video_canvas_import_application(
        &self,
    ) -> &dyn a4n_api_contract::video::CanvasVideoImportApplicationPort {
        self.video_canvas_import_application.as_ref()
    }

    pub(crate) fn video_trim_application(
        &self,
    ) -> &dyn a4n_api_contract::video::VideoTrimApplicationPort {
        self.video_trim_application.as_ref()
    }

    pub(crate) fn video_assembly_planning_application(
        &self,
    ) -> &dyn a4n_api_contract::video::VideoAssemblyPlanningApplicationPort {
        self.video_assembly_planning_application.as_ref()
    }

    pub(crate) fn video_assembly_push_application(
        &self,
    ) -> &dyn a4n_api_contract::video::VideoAssemblyPushApplicationPort {
        self.video_assembly_push_application.as_ref()
    }

    pub(crate) fn video_assembly_postprocess_application(
        &self,
    ) -> &dyn a4n_api_contract::video::VideoAssemblyPostprocessApplicationPort {
        self.video_assembly_postprocess_application.as_ref()
    }

    pub(crate) fn video_record_lifecycle_application(
        &self,
    ) -> &dyn a4n_api_contract::video::VideoRecordLifecycleApplicationPort {
        self.video_record_lifecycle_application.as_ref()
    }

    pub(crate) fn video_record_completion_application(
        &self,
    ) -> &dyn a4n_api_contract::video::VideoRecordCompletionApplicationPort {
        self.video_record_completion_application.as_ref()
    }

    pub(crate) fn video_tail_frame_application(
        &self,
    ) -> &dyn a4n_api_contract::video::VideoTailFrameApplicationPort {
        self.video_tail_frame_application.as_ref()
    }

    pub(crate) fn video_record_failure_application(
        &self,
    ) -> &dyn a4n_api_contract::video::VideoRecordFailureApplicationPort {
        self.video_record_failure_application.as_ref()
    }

    pub(crate) fn video_canvas_media_application(
        &self,
    ) -> &dyn a4n_api_contract::video::VideoCanvasMediaApplicationPort {
        self.video_canvas_media_application.as_ref()
    }

    pub(crate) fn chapter_visual_plan_read_application(
        &self,
    ) -> &dyn a4n_api_contract::chapter_visual_plan::ChapterVisualPlanReadApplicationPort {
        self.chapter_visual_plan_read_application.as_ref()
    }

    pub(crate) fn chapter_visual_plan_mutation_application(
        &self,
    ) -> &dyn a4n_api_contract::chapter_visual_plan::ChapterVisualPlanMutationApplicationPort {
        self.chapter_visual_plan_mutation_application.as_ref()
    }

    pub(crate) fn video_prompt_read_application(
        &self,
    ) -> &dyn a4n_api_contract::video::VideoPromptReadApplicationPort {
        self.video_prompt_read_application.as_ref()
    }

    pub(crate) fn video_prompt_mutation_application(
        &self,
    ) -> &dyn a4n_api_contract::video::VideoPromptMutationApplicationPort {
        self.video_prompt_mutation_application.as_ref()
    }

    pub(crate) fn video_prompt_timeline_application(
        &self,
    ) -> &dyn a4n_api_contract::video::VideoPromptTimelineApplicationPort {
        self.video_prompt_timeline_application.as_ref()
    }

    pub(crate) fn video_prompt_draft_application(
        &self,
    ) -> &dyn a4n_api_contract::video::VideoPromptDraftApplicationPort {
        self.video_prompt_draft_application.as_ref()
    }

    pub fn image_record_read_application(
        &self,
    ) -> &dyn a4n_api_contract::image::ImageRecordReadApplicationPort {
        self.image_record_read_application.as_ref()
    }

    pub(crate) fn image_record_mutation_application(
        &self,
    ) -> &dyn a4n_api_contract::image::ImageRecordMutationApplicationPort {
        self.image_record_mutation_application.as_ref()
    }

    pub(crate) fn image_import_preparation_application(
        &self,
    ) -> &dyn a4n_image_host_contract::ImageImportPreparationApplicationPort {
        self.image_import_preparation_application.as_ref()
    }

    pub fn image_import_publication_application(
        &self,
    ) -> &dyn a4n_image_host_contract::ImageImportPublicationApplicationPort {
        self.image_import_publication_application.as_ref()
    }

    pub(crate) fn image_publication_recovery_application(
        &self,
    ) -> &dyn a4n_image_host_contract::ImagePublicationRecoveryPort {
        self.image_publication_recovery_application.as_ref()
    }

    pub fn image_publication_recovery_read_application(
        &self,
    ) -> &dyn a4n_api_contract::image::ImagePublicationRecoveryReadPort {
        self.image_publication_recovery_read_application.as_ref()
    }

    pub(crate) fn image_media_reference_application(
        &self,
    ) -> &dyn a4n_api_contract::image::ImageMediaReferencePort {
        self.image_media_reference_application.as_ref()
    }

    pub(crate) fn image_provider_config(
        &self,
    ) -> &dyn a4n_api_contract::image::ImageProviderConfigPort {
        self.image_provider_config.as_ref()
    }

    pub(crate) fn galgame_image_generation_application(
        &self,
    ) -> &dyn a4n_image_host_contract::GalgameImageGenerationApplicationPort {
        self.galgame_image_generation_application.as_ref()
    }

    pub(crate) fn image_canvas_request_preview_application(
        &self,
    ) -> &dyn a4n_image_host_contract::ImageCanvasRequestPreviewApplicationPort {
        self.image_canvas_request_preview_application.as_ref()
    }

    pub(crate) fn image_batch_request_preview_application(
        &self,
    ) -> &dyn a4n_image_host_contract::ImageBatchRequestPreviewApplicationPort {
        self.image_batch_request_preview_application.as_ref()
    }

    pub(crate) fn image_object_profile_request_preview_application(
        &self,
    ) -> &dyn a4n_image_host_contract::ImageObjectProfileRequestPreviewApplicationPort {
        self.image_object_profile_request_preview_application
            .as_ref()
    }

    pub(crate) fn image_page_request_preview_application(
        &self,
    ) -> &dyn a4n_image_host_contract::ImagePageRequestPreviewApplicationPort {
        self.image_page_request_preview_application.as_ref()
    }

    pub(crate) fn image_scene_request_preview_application(
        &self,
    ) -> &dyn a4n_image_host_contract::ImageSceneRequestPreviewApplicationPort {
        self.image_scene_request_preview_application.as_ref()
    }

    pub(crate) fn image_generation_task_application(
        &self,
    ) -> &dyn a4n_image_host_contract::ImageGenerationTaskApplicationPort {
        self.image_generation_task_application.as_ref()
    }

    pub(crate) fn image_profile_prompt_application(
        &self,
    ) -> &dyn a4n_api_contract::image_generation::ImageProfilePromptApplicationPort {
        self.image_profile_prompt_application.as_ref()
    }

    pub(crate) fn profile_material_read_application(
        &self,
    ) -> &dyn a4n_api_contract::profile::ProfileMaterialReadApplicationPort {
        self.profile_material_read_application.as_ref()
    }

    pub(crate) fn profile_material_mutation_application(
        &self,
    ) -> &dyn a4n_api_contract::profile::ProfileMaterialMutationApplicationPort {
        self.profile_material_mutation_application.as_ref()
    }

    pub(crate) fn profile_material_upload_application(
        &self,
    ) -> &dyn a4n_image_host_contract::ProfileMaterialUploadApplicationPort {
        self.profile_material_upload_application.as_ref()
    }

    pub(crate) fn portrait_record_read_application(
        &self,
    ) -> &dyn a4n_api_contract::portrait::PortraitRecordReadApplicationPort {
        self.portrait_record_read_application.as_ref()
    }

    pub(crate) fn portrait_record_mutation_application(
        &self,
    ) -> &dyn a4n_api_contract::portrait::PortraitRecordMutationApplicationPort {
        self.portrait_record_mutation_application.as_ref()
    }

    pub(crate) fn portrait_prompt_request_preview_application(
        &self,
    ) -> &dyn a4n_portrait_host_contract::PortraitPromptRequestPreviewApplicationPort {
        self.portrait_prompt_request_preview_application.as_ref()
    }

    pub(crate) fn portrait_image_request_preview_application(
        &self,
    ) -> &dyn a4n_portrait_host_contract::PortraitImageRequestPreviewApplicationPort {
        self.portrait_image_request_preview_application.as_ref()
    }

    pub(crate) fn portrait_prompt_application(
        &self,
    ) -> &dyn a4n_portrait_host_contract::PortraitPromptApplicationPort {
        self.portrait_prompt_application.as_ref()
    }

    pub(crate) fn portrait_upload_application(
        &self,
    ) -> &dyn a4n_portrait_host_contract::PortraitUploadApplicationPort {
        self.portrait_upload_application.as_ref()
    }

    pub(crate) fn portrait_generation_application(
        &self,
    ) -> &dyn a4n_portrait_host_contract::PortraitGenerationApplicationPort {
        self.portrait_generation_application.as_ref()
    }

    pub(crate) fn galgame_pipeline_execution_application(
        &self,
    ) -> &dyn a4n_api_contract::galgame::GalgamePipelineExecutionApplicationPort {
        self.galgame_pipeline_execution_application.as_ref()
    }

    pub(crate) fn galgame_pipeline_preview_application(
        &self,
    ) -> &dyn a4n_api_contract::galgame::GalgamePipelinePreviewApplicationPort {
        self.galgame_pipeline_preview_application.as_ref()
    }

    pub(crate) fn galgame_pipeline_read_application(
        &self,
    ) -> &dyn a4n_api_contract::galgame::GalgamePipelineReadApplicationPort {
        self.galgame_pipeline_read_application.as_ref()
    }

    pub(crate) fn galgame_pipeline_cancellation_application(
        &self,
    ) -> &dyn a4n_api_contract::galgame::GalgamePipelineCancellationApplicationPort {
        self.galgame_pipeline_cancellation_application.as_ref()
    }

    pub(crate) fn galgame_asset_generation_application(
        &self,
    ) -> &dyn a4n_galgame_host_contract::GalgameAssetGenerationApplicationPort {
        self.galgame_asset_generation_application.as_ref()
    }

    pub(crate) fn galgame_asset_plan_application(
        &self,
    ) -> &dyn a4n_api_contract::galgame::GalgameAssetPlanApplicationPort {
        self.galgame_asset_plan_application.as_ref()
    }

    pub(crate) fn galgame_record_mutation_application(
        &self,
    ) -> &dyn a4n_api_contract::galgame::GalgameRecordMutationApplicationPort {
        self.galgame_record_mutation_application.as_ref()
    }

    pub(crate) fn galgame_media_mutation_application(
        &self,
    ) -> &dyn a4n_api_contract::galgame::GalgameMediaMutationApplicationPort {
        self.galgame_media_mutation_application.as_ref()
    }

    pub(crate) fn manga_pipeline_execution_application(
        &self,
    ) -> &dyn a4n_api_contract::manga::MangaPipelineExecutionApplicationPort {
        self.manga_pipeline_execution_application.as_ref()
    }

    pub(crate) fn manga_pipeline_preview_application(
        &self,
    ) -> &dyn a4n_api_contract::manga::MangaPipelinePreviewApplicationPort {
        self.manga_pipeline_preview_application.as_ref()
    }

    pub(crate) fn manga_pipeline_read_application(
        &self,
    ) -> &dyn a4n_api_contract::manga::MangaPipelineReadApplicationPort {
        self.manga_pipeline_read_application.as_ref()
    }

    pub(crate) fn manga_pipeline_cancellation_application(
        &self,
    ) -> &dyn a4n_api_contract::manga::MangaPipelineCancellationApplicationPort {
        self.manga_pipeline_cancellation_application.as_ref()
    }

    pub(crate) fn manga_record_read_application(
        &self,
    ) -> &dyn a4n_api_contract::manga::MangaRecordReadApplicationPort {
        self.manga_record_read_application.as_ref()
    }

    pub(crate) fn manga_record_mutation_application(
        &self,
    ) -> &dyn a4n_api_contract::manga::MangaRecordMutationApplicationPort {
        self.manga_record_mutation_application.as_ref()
    }

    pub(crate) fn task_manager(&self) -> &a4n_task_runtime::channel::TaskManager {
        self.task_manager.as_ref()
    }

    pub(crate) fn generation_task_read_application(
        &self,
    ) -> &dyn a4n_generation_host_contract::GenerationTaskReadApplicationPort {
        self.generation_task_read_application.as_ref()
    }

    pub(crate) fn generation_task_mutation_application(
        &self,
    ) -> &dyn a4n_generation_host_contract::GenerationTaskMutationApplicationPort {
        self.generation_task_mutation_application.as_ref()
    }

    pub(crate) fn generation_lifecycle_application(
        &self,
    ) -> &dyn a4n_generation_host_contract::GenerationLifecycleApplicationPort {
        self.generation_lifecycle_application.as_ref()
    }

    pub(crate) fn generation_cleanup_application(
        &self,
    ) -> &dyn a4n_generation_host_contract::GenerationCleanupApplicationPort {
        self.generation_cleanup_application.as_ref()
    }

    pub(crate) fn source_sync_read_application(
        &self,
    ) -> &dyn a4n_api_contract::source_sync::SourceSyncReadApplicationPort {
        self.source_sync_read_application.as_ref()
    }

    pub(crate) fn source_sync_mutation_application(
        &self,
    ) -> &dyn a4n_api_contract::source_sync::SourceSyncMutationApplicationPort {
        self.source_sync_mutation_application.as_ref()
    }

    pub(crate) fn shooting_script_record_read_application(
        &self,
    ) -> &dyn a4n_api_contract::shooting_script::ShootingScriptRecordReadApplicationPort {
        self.shooting_script_record_read_application.as_ref()
    }

    pub(crate) fn shooting_script_record_mutation_application(
        &self,
    ) -> &dyn a4n_api_contract::shooting_script::ShootingScriptRecordMutationApplicationPort {
        self.shooting_script_record_mutation_application.as_ref()
    }

    pub(crate) fn shooting_script_full_generation_application(
        &self,
    ) -> &dyn a4n_novel_host_contract::ShootingScriptFullGenerationApplicationPort {
        self.shooting_script_full_generation_application.as_ref()
    }

    pub(crate) fn shooting_script_next_unit_generation_application(
        &self,
    ) -> &dyn a4n_novel_host_contract::ShootingScriptNextUnitGenerationApplicationPort {
        self.shooting_script_next_unit_generation_application
            .as_ref()
    }

    pub(crate) fn rag_search(&self) -> &dyn a4n_rag_contract::RagSearchPort {
        self.rag_search.as_ref()
    }

    pub(crate) fn rag_ingest(&self) -> &dyn a4n_rag_contract::RagIngestPort {
        self.rag_ingest.as_ref()
    }

    pub(crate) fn provider_runtime(&self) -> &crate::provider_runtime::DynamicProviderFacade {
        &self.provider_runtime
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
        drop(tokio::spawn(
            crate::runtime_config::inherit_runtime_context(async move {
                if let Err(error) = future.await {
                    tracing::error!(
                        task_id = %task_id,
                        attempt,
                        worker = worker_kind,
                        error = %error,
                        "generation worker 失败"
                    );
                }
            }),
        ));
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
    ) -> Result<crate::dynamic_llm::DynamicLlmProvider, a4n_error::AppError> {
        let facade = self.provider_runtime.clone();
        if self.runtime_profile.uses_sidecars() {
            let client = self.capabilities.llm.clone().ok_or_else(|| {
                a4n_error::AppError::ConfigError(
                    "LLM sidecar capability 未配置，不能回退到 embedded provider".to_string(),
                )
            })?;
            return Ok(
                crate::dynamic_llm::DynamicLlmProvider::new_with_sidecar_client(facade, client),
            );
        }
        Ok(crate::dynamic_llm::DynamicLlmProvider::new(facade))
    }
}

/// Runtime-owned resources prepared before a Host binds application behavior.
pub struct RuntimeBootstrap {
    config: AppConfig,
    runtime_profile: RuntimeProfile,
    runtime_role: RuntimeRole,
    db: Arc<sea_orm::DatabaseConnection>,
    capabilities: RuntimeCapabilities,
    service_manager: Arc<a4n_service_manager::ServiceManager>,
    event_bus: Arc<BroadcastEventBus>,
    task_manager: Arc<a4n_task_runtime::channel::TaskManager>,
    task_queue: Arc<PersistentTaskQueue>,
    prompt_engine: Arc<a4n_prompt::tera_engine::TeraPromptEngine>,
    rag_store: Arc<dyn a4n_rag_core::vector_store::VectorStore>,
    snapshot_store: Arc<Sha256SnapshotStore>,
    generation_owner_lock: Option<Arc<sea_orm::DatabaseConnection>>,
    data_dir_lock: Option<Arc<File>>,
}

impl RuntimeBootstrap {
    pub async fn prepare(
        config: AppConfig,
        runtime_profile: RuntimeProfile,
        runtime_role: RuntimeRole,
    ) -> Result<Self, a4n_error::AppError> {
        runtime_profile.validate_build_capabilities()?;
        validate_runtime_config(&config)?;
        let data_dir_lock = match runtime_role {
            RuntimeRole::Host => Some(Arc::new(acquire_data_dir_lock(&config.data_dir)?)),
            RuntimeRole::AgentWorker => None,
        };

        let db = sea_orm::Database::connect(&config.database_url)
            .await
            .map_err(a4n_error::AppError::Database)?;
        let db = Arc::new(db);
        let generation_owner_lock = match runtime_role {
            RuntimeRole::Host => Some(Arc::new(
                acquire_generation_owner_lock(&config.database_url).await?,
            )),
            RuntimeRole::AgentWorker => None,
        };

        if runtime_role == RuntimeRole::Host {
            run_startup_migrations(db.as_ref()).await?;
        }

        let service_manager = Arc::new(
            crate::sidecar_clients::runtime_service_manager(runtime_profile, &config)
                .map_err(a4n_error::AppError::ConfigError)?,
        );
        let capabilities =
            crate::sidecar_clients::runtime_capabilities(runtime_profile, service_manager.as_ref())
                .map_err(a4n_error::AppError::ConfigError)?;
        let event_bus = Arc::new(BroadcastEventBus::new(config.system.event_bus_capacity));
        let task_manager = Arc::new(a4n_task_runtime::channel::TaskManager::new());
        let task_queue = Arc::new(PersistentTaskQueue::new(db.clone()).await?);
        let (prompt_engine_result, rag_store_result) = futures::join!(
            async { create_prompt_engine(&config) },
            create_rag_store(runtime_profile, capabilities.rag.clone()),
        );

        Ok(Self {
            config,
            runtime_profile,
            runtime_role,
            db: db.clone(),
            capabilities,
            service_manager,
            event_bus,
            task_manager,
            task_queue,
            prompt_engine: Arc::new(prompt_engine_result?),
            rag_store: rag_store_result?,
            snapshot_store: Arc::new(Sha256SnapshotStore::new(db)),
            generation_owner_lock,
            data_dir_lock,
        })
    }

    pub fn database(&self) -> a4n_db::DbHandle {
        a4n_db::DbHandle::new(self.db.clone())
    }

    pub fn config(&self) -> &AppConfig {
        &self.config
    }

    pub fn prompt_client(&self) -> Option<Arc<dyn a4n_prompt_client::PromptClient>> {
        self.capabilities.prompt.clone()
    }

    pub fn service_manager(&self) -> Arc<a4n_service_manager::ServiceManager> {
        self.service_manager.clone()
    }

    pub fn event_bus(&self) -> Arc<BroadcastEventBus> {
        self.event_bus.clone()
    }

    pub fn task_manager(&self) -> Arc<a4n_task_runtime::channel::TaskManager> {
        self.task_manager.clone()
    }

    pub fn rag_store(&self) -> Arc<dyn a4n_rag_core::vector_store::VectorStore> {
        self.rag_store.clone()
    }

    pub fn task_queue(&self) -> Arc<PersistentTaskQueue> {
        self.task_queue.clone()
    }

    pub fn prompt_engine(&self) -> Arc<a4n_prompt::tera_engine::TeraPromptEngine> {
        self.prompt_engine.clone()
    }

    pub fn snapshot_store(&self) -> Arc<Sha256SnapshotStore> {
        self.snapshot_store.clone()
    }

    pub fn document_export_renderer(
        &self,
    ) -> Result<Option<Arc<dyn a4n_export_contract::ExportDocumentRenderer>>, a4n_error::AppError>
    {
        crate::sidecar_clients::document_export_renderer(
            self.runtime_profile,
            self.capabilities.export.clone(),
        )
        .map_err(a4n_error::AppError::ConfigError)
    }

    pub fn create_provider(
        &self,
        provider_port: Arc<dyn a4n_provider_runtime_contract::ProviderRuntimePort>,
    ) -> Result<
        (
            crate::DynamicProviderFacade,
            Arc<dyn a4n_llm_provider_contract::provider::LlmProvider>,
            Arc<dyn a4n_embedding_provider_contract::EmbeddingProvider>,
        ),
        a4n_error::AppError,
    > {
        let provider_facade = crate::provider_runtime::DynamicProviderFacade::new(provider_port);
        let embedding = crate::sidecar_clients::create_embedding_provider(
            provider_facade.clone(),
            &self.config,
            self.runtime_profile,
            self.capabilities.embedding.clone(),
        )?;
        let llm = crate::sidecar_clients::create_llm_provider(
            provider_facade.clone(),
            self.runtime_profile,
            &self.capabilities,
        )?;
        Ok((provider_facade, llm, embedding))
    }

    pub async fn finish(
        self,
        application_ports: ApplicationPorts,
        provider_runtime: crate::DynamicProviderFacade,
        rag_search: Arc<dyn a4n_rag_contract::RagSearchPort>,
        rag_ingest: Arc<dyn a4n_rag_contract::RagIngestPort>,
    ) -> Result<AppState, a4n_error::AppError> {
        let RuntimeBootstrap {
            config,
            runtime_profile,
            runtime_role,
            capabilities,
            service_manager,
            event_bus,
            task_manager,
            task_queue,
            generation_owner_lock,
            data_dir_lock,
            ..
        } = self;

        application_ports
            .prompt_mutation_application
            .hydrate_overrides()
            .await?;

        if runtime_role == RuntimeRole::Host {
            application_ports
                .package_mutation_application
                .initialize_states()
                .await?;
            crate::baseline_configs::ensure_baseline_model_configs(
                application_ports.config_read_application.as_ref(),
                application_ports.config_mutation_application.as_ref(),
                &config,
            )
            .await?;
        }

        let task_queue_application = Arc::new(crate::task_queue::TaskQueueApplication::new(
            task_queue.clone(),
        ));
        let managed_sidecars = Arc::new(tokio::sync::Mutex::new(Vec::new()));
        let state = AppState {
            config,
            task_manager,
            service_health_application: Arc::new(
                crate::service_health::ServiceHealthApplication::new(service_manager.clone()),
            ),
            task_cancellation_application: task_queue_application.clone(),
            task_queue_stats_application: task_queue_application,
            application_ports,
            capabilities,
            event_bus,
            provider_runtime,
            rag_search,
            rag_ingest,
            runtime_profile,
            service_manager,
            destructive_lifecycle: Arc::new(
                crate::actions::generation_tasks::destructive_lifecycle::DestructiveLifecycleCoordinator::default(),
            ),
            _generation_owner_lock: generation_owner_lock,
            _data_dir_lock: data_dir_lock,
            managed_sidecars,
            managed_sidecar_lifecycle: Arc::new(tokio::sync::Mutex::new(())),
            agent_worker_cancel: tokio_util::sync::CancellationToken::new(),
            agent_worker_notify: Arc::new(tokio::sync::Notify::new()),
            agent_worker_handle: Arc::new(tokio::sync::Mutex::new(None)),
            agent_admission: Arc::new(tokio::sync::Mutex::new(())),
            agent_run_cancellations: Arc::new(tokio::sync::Mutex::new(HashMap::new())),
            agent_runtime_snapshots: Arc::new(tokio::sync::RwLock::new(HashMap::new())),
            agent_host_dispatcher: Arc::new(tokio::sync::RwLock::new(None)),
        };

        if runtime_role == RuntimeRole::Host {
            let enabled_services = state.package_read_application().enabled_services().await?;
            state
                .start_managed_sidecars_for_services(&enabled_services)
                .await
                .map_err(a4n_error::AppError::ConfigError)?;
            state
                .destructive_lifecycle()
                .continue_pending(&state)
                .await
                .map_err(|current| a4n_error::AppError::InvalidState {
                    current,
                    operation: "恢复未完成 generation lifecycle operation".to_string(),
                })?;
            crate::actions::video_records::recover_expired_uploaded_video_sources(state.temp_dir());
            crate::actions::video_records::recover_expired_assembly_caches(&state).await;
            crate::cleanup_outbox::recover_cleanup_outbox(&state).await;
            crate::cleanup_outbox::spawn_cleanup_worker(state.clone());
            crate::actions::package::spawn_package_lifecycle_worker(state.clone());
        }

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

fn acquire_data_dir_lock(data_dir: &Path) -> Result<File, a4n_error::AppError> {
    let path = data_dir.join(".a4n-host.lock");
    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(&path)
        .map_err(|error| {
            a4n_error::AppError::ConfigError(format!(
                "无法打开 data directory ownership lock {}：{error}",
                path.display()
            ))
        })?;
    match file.try_lock() {
        Ok(()) => Ok(file),
        Err(TryLockError::WouldBlock) => Err(a4n_error::AppError::ConfigError(format!(
            "data directory 已被另一个 A4N Host 占用：{}；同一目录只允许一个写入 Host",
            data_dir.display()
        ))),
        Err(TryLockError::Error(error)) => Err(a4n_error::AppError::ConfigError(format!(
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

#[async_trait::async_trait]
impl a4n_api_contract::media::MediaAudioProbeHostPort for AppState {
    async fn probe_audio_duration_secs(
        &self,
        relative_data_path: &str,
    ) -> Result<f32, a4n_error::AppError> {
        let path = self.config.data_dir.join(relative_data_path);
        let input = self
            .data_media_file_ref(&path, "音频文件")
            .map_err(|current| a4n_error::AppError::InvalidState {
                current,
                operation: "构造音频探测路径".into(),
            })?;
        let output = self
            .execute_media(a4n_media_contract::MediaExecuteInput::ProbeAudioDuration { input })
            .await
            .map_err(|current| a4n_error::AppError::InvalidState {
                current,
                operation: "探测音频时长".into(),
            })?;
        match output {
            a4n_media_contract::MediaExecuteOutput::AudioDuration { duration_secs } => {
                Ok(duration_secs)
            }
            output => Err(a4n_error::AppError::InvalidState {
                current: format!("media worker 返回了不匹配的输出：{output:?}"),
                operation: "探测音频时长".into(),
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::acquire_data_dir_lock;
    use crate::runtime_config::inherit_runtime_context;

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
            "schema_version": 2,
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
            let worker = tokio::spawn(inherit_runtime_context(async {
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
