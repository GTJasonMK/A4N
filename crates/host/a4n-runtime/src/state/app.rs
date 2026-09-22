use std::{
    fs::{File, OpenOptions, TryLockError},
    future::Future,
    ops::Deref,
    path::Path,
    pin::Pin,
    sync::Arc,
};

use a4n_agent_runtime::AgentRuntimeShared;
use a4n_api_contract::generation_tasks::GenerationTaskHandle;
use a4n_config::app_config::AppConfig;
use a4n_db::character::Sha256SnapshotStore;
use a4n_event::{BroadcastEventBus, InternalEvent};
use a4n_task_runtime::persistent::PersistentTaskQueue;
use a4n_worker_contract::{
    AGENT_HOST_COMMAND_OPERATION, AgentHostCommandPayload, AgentRunPayload, WorkerEnvelope,
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

mod agent;
mod application_accessors;
mod bootstrap;
mod generation_accessors;
mod generation_execution;
mod media_accessors;
mod sidecar_media;
mod traits;

pub use bootstrap::RuntimeBootstrap;

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
    /// Agent lifecycle is shared with the common engine; durable state remains
    /// in `AgentApplication`/database.
    agent_worker_cancel: tokio_util::sync::CancellationToken,
    agent_worker_notify: Arc<tokio::sync::Notify>,
    agent_worker_handle: Arc<tokio::sync::Mutex<Option<tokio::task::JoinHandle<()>>>>,
    agent_runtime_shared: AgentRuntimeShared,
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
            WorkerEnvelope<AgentHostCommandPayload>,
        ) -> AgentHostCommandFuture
        + Send
        + Sync,
>;
#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::bootstrap::{acquire_data_dir_lock, include_runtime_sidecar_services};
    use crate::runtime_config::inherit_runtime_context;
    use crate::state::RuntimeProfile;

    #[test]
    fn sidecar_bootstrap_includes_media_service_once() {
        let mut services = vec!["video".to_string()];
        include_runtime_sidecar_services(RuntimeProfile::Sidecar, &mut services);
        include_runtime_sidecar_services(RuntimeProfile::Sidecar, &mut services);

        assert_eq!(
            services,
            vec!["video", a4n_media_contract::MEDIA_SERVICE_NAME]
        );

        let mut minimal_services = Vec::new();
        include_runtime_sidecar_services(RuntimeProfile::Minimal, &mut minimal_services);
        assert!(minimal_services.is_empty());
    }

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
