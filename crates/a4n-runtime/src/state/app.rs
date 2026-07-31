use std::{
    collections::BTreeMap,
    fs::{File, OpenOptions, TryLockError},
    future::Future,
    path::Path,
    sync::Arc,
};

use a4n_core::config::app_config::AppConfig;
use a4n_core::event::bridge::BroadcastEventBus as CoreEventBus;
use a4n_db::character::snapshot_store::Sha256SnapshotStore;
use a4n_orchestrator::chapter_generation::ChapterGenerationService;
use a4n_orchestrator::task_queue::PersistentTaskQueue;

use super::database::{
    acquire_generation_owner_lock, run_startup_migrations, validate_runtime_config,
};
use super::profile::RuntimeProfile;
use super::prompt_engine::create_prompt_engine;
use super::rag::create_rag_store;

/// 阶段 11：宿主组装壳。
///
/// 业务字段统一在 [`a4n_orchestrator::OrchestratorContext`]；本类型仅额外持有
/// 进程生命周期相关的 managed sidecars，并通过窄 facade 暴露领域入口（禁止 Deref 回流）。
#[derive(Clone)]
pub struct AppState {
    /// 业务编排唯一入口（含原 AppState 业务字段）。
    orchestrator: a4n_orchestrator::OrchestratorContext,
    /// 运行时 profile（留在 AppState，避免与 orch 循环依赖）。
    runtime_profile: RuntimeProfile,
    destructive_lifecycle: Arc<
        crate::actions::generation_tasks::destructive_lifecycle::DestructiveLifecycleCoordinator,
    >,
    generation_executions: Arc<GenerationExecutionOwner>,
    _generation_owner_lock: Arc<sea_orm::DatabaseConnection>,
    /// ponytail: 本地 data directory 只允许一个写入 Host；明确支持共享写存储时再升级为 artifact 级数据库 lease。
    _data_dir_lock: Arc<File>,
    _managed_sidecars: Arc<tokio::sync::Mutex<Vec<a4n_service_manager::ManagedSidecarProcess>>>,
}

#[derive(Default)]
struct GenerationExecutionOwner {
    workers: std::sync::Mutex<BTreeMap<String, tokio::task::JoinHandle<()>>>,
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

impl Drop for GenerationExecutionOwner {
    fn drop(&mut self) {
        for worker in self
            .workers
            .get_mut()
            .expect("generation execution mutex poisoned")
            .values()
        {
            worker.abort();
        }
    }
}

impl AppState {
    /// 在 Host 退出前停止其拥有的 managed sidecar；可重复调用。
    pub async fn shutdown_managed_sidecars(&self) -> Result<(), String> {
        let processes = std::mem::take(&mut *self._managed_sidecars.lock().await);
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

    pub(crate) fn app_paths(&self) -> a4n_core::files::paths::AppPaths {
        self.orchestrator.app_paths()
    }

    pub(crate) fn default_fastembed_cache_dir(&self) -> std::path::PathBuf {
        self.orchestrator.default_fastembed_cache_dir()
    }

    pub(crate) fn configured_database_url(&self) -> &str {
        self.orchestrator.configured_database_url()
    }

    pub(crate) fn required_jwt_secret(&self) -> Result<&str, a4n_core::error::AppError> {
        self.orchestrator.required_jwt_secret()
    }

    pub(crate) fn register_task_handle(
        &self,
        handle: a4n_core::event::channel::TaskHandle,
    ) -> Arc<a4n_core::event::channel::TaskHandle> {
        self.orchestrator.register_task_handle(handle)
    }

    pub(crate) fn remove_task_handle(
        &self,
        task_id: &str,
    ) -> Option<Arc<a4n_core::event::channel::TaskHandle>> {
        self.orchestrator.remove_task_handle(task_id)
    }

    pub(crate) fn task_handle(
        &self,
        task_id: &str,
    ) -> Option<Arc<a4n_core::event::channel::TaskHandle>> {
        self.orchestrator.task_handle(task_id)
    }

    pub(crate) fn cancel_task_handle(&self, task_id: &str) -> bool {
        self.orchestrator.cancel_task_handle(task_id)
    }

    pub(crate) fn cancel_or_remember_task_handle(&self, task_id: &str, user_id: i64) -> bool {
        self.orchestrator
            .cancel_or_remember_task_handle(task_id, user_id)
    }

    pub(crate) fn active_task_snapshots(&self) -> Vec<a4n_core::event::channel::TaskSnapshot> {
        self.orchestrator.active_task_snapshots()
    }

    pub fn subscribe_internal_events(
        &self,
    ) -> tokio::sync::broadcast::Receiver<a4n_core::event::bridge::InternalEvent> {
        self.orchestrator.subscribe_internal_events()
    }

    pub(crate) fn prompt_records(&self) -> super::prompt_records::PromptRecordsRuntimeFacade<'_> {
        super::prompt_records::PromptRecordsRuntimeFacade::new(
            &self.orchestrator,
            self.runtime_profile,
        )
    }

    pub(crate) fn admin(&self) -> super::admin::AdminRuntimeFacade<'_> {
        super::admin::AdminRuntimeFacade::new(&self.orchestrator)
    }

    pub(crate) fn auth(&self) -> super::auth::AuthRuntimeFacade<'_> {
        super::auth::AuthRuntimeFacade::new(&self.orchestrator)
    }

    pub(crate) fn collaboration(&self) -> super::collaboration::CollaborationRuntimeFacade<'_> {
        super::collaboration::CollaborationRuntimeFacade::new(&self.orchestrator)
    }

    pub(crate) fn project_read(&self) -> super::project_read::ProjectReadRuntimeFacade<'_> {
        super::project_read::ProjectReadRuntimeFacade::new(&self.orchestrator)
    }

    pub(crate) fn project_mutation(
        &self,
    ) -> super::project_mutation::ProjectMutationRuntimeFacade<'_> {
        super::project_mutation::ProjectMutationRuntimeFacade::new(&self.orchestrator)
    }

    pub(crate) fn project_blueprint(
        &self,
    ) -> super::project_blueprint::ProjectBlueprintRuntimeFacade<'_> {
        super::project_blueprint::ProjectBlueprintRuntimeFacade::new(&self.orchestrator)
    }

    pub(crate) fn project_duplication(
        &self,
    ) -> super::project_duplication::ProjectDuplicationRuntimeFacade<'_> {
        super::project_duplication::ProjectDuplicationRuntimeFacade::new(&self.orchestrator)
    }

    pub(crate) fn video_studio_transfer(
        &self,
    ) -> super::video_studio_transfer::VideoStudioTransferRuntimeFacade<'_> {
        super::video_studio_transfer::VideoStudioTransferRuntimeFacade::new(&self.orchestrator)
    }

    pub(crate) fn project_status(&self) -> super::project_status::ProjectStatusRuntimeFacade<'_> {
        super::project_status::ProjectStatusRuntimeFacade::new(&self.orchestrator)
    }

    pub(crate) fn project_dependency_graph(
        &self,
    ) -> super::project_dependency_graph::ProjectDependencyGraphRuntimeFacade<'_> {
        super::project_dependency_graph::ProjectDependencyGraphRuntimeFacade::new(
            &self.orchestrator,
        )
    }

    pub(crate) fn project_chapter_records(
        &self,
    ) -> super::project_chapter_records::ProjectChapterRecordsRuntimeFacade<'_> {
        super::project_chapter_records::ProjectChapterRecordsRuntimeFacade::new(&self.orchestrator)
    }

    pub(crate) fn project_outline_records(
        &self,
    ) -> super::project_outline_records::ProjectOutlineRecordsRuntimeFacade<'_> {
        super::project_outline_records::ProjectOutlineRecordsRuntimeFacade::new(&self.orchestrator)
    }

    pub(crate) fn project_scene_records(
        &self,
    ) -> super::project_scene_records::ProjectSceneRecordsRuntimeFacade<'_> {
        super::project_scene_records::ProjectSceneRecordsRuntimeFacade::new(&self.orchestrator)
    }

    pub(crate) fn project_family(&self) -> super::project_family::ProjectFamilyRuntimeFacade<'_> {
        super::project_family::ProjectFamilyRuntimeFacade::new(&self.orchestrator)
    }

    pub(crate) fn settings(&self) -> super::settings::SettingsRuntimeFacade<'_> {
        super::settings::SettingsRuntimeFacade::new(&self.orchestrator)
    }

    pub(crate) fn import_review(&self) -> super::import_review::ImportReviewRuntimeFacade<'_> {
        super::import_review::ImportReviewRuntimeFacade::new(&self.orchestrator)
    }

    pub(crate) fn export(&self) -> super::export::ExportRuntimeFacade<'_> {
        super::export::ExportRuntimeFacade::new(&self.orchestrator)
    }

    pub(crate) fn asset(&self) -> super::asset::AssetRuntimeFacade<'_> {
        super::asset::AssetRuntimeFacade::new(&self.orchestrator)
    }

    pub(crate) fn scene_profile(&self) -> super::scene_profile::SceneProfileRuntimeFacade<'_> {
        super::scene_profile::SceneProfileRuntimeFacade::new(&self.orchestrator)
    }

    pub(crate) fn chapter_content(
        &self,
    ) -> super::chapter_content::ChapterContentRuntimeFacade<'_> {
        super::chapter_content::ChapterContentRuntimeFacade::new(&self.orchestrator)
    }

    pub(crate) fn character(&self) -> super::character::CharacterRuntimeFacade<'_> {
        super::character::CharacterRuntimeFacade::new(&self.orchestrator)
    }

    pub(crate) fn outline(&self) -> super::outline::OutlineRuntimeFacade<'_> {
        super::outline::OutlineRuntimeFacade::new(&self.orchestrator)
    }

    pub(crate) fn portrait(&self) -> super::portrait::PortraitRuntimeFacade<'_> {
        super::portrait::PortraitRuntimeFacade::new(&self.orchestrator)
    }

    pub(crate) fn rag(&self) -> super::rag::RagRuntimeFacade<'_> {
        super::rag::RagRuntimeFacade::new(&self.orchestrator)
    }

    pub(crate) fn optimize(&self) -> super::optimize::OptimizeRuntimeFacade<'_> {
        super::optimize::OptimizeRuntimeFacade::new(&self.orchestrator)
    }

    pub(crate) fn galgame(&self) -> super::galgame::GalgameRuntimeFacade<'_> {
        super::galgame::GalgameRuntimeFacade::new(&self.orchestrator)
    }

    pub(crate) fn generation_tasks(
        &self,
    ) -> super::generation_tasks::GenerationTasksRuntimeFacade<'_> {
        super::generation_tasks::GenerationTasksRuntimeFacade::new(&self.orchestrator)
    }

    pub(crate) fn spawn_generation_execution(
        &self,
        task_id: String,
        future: impl Future<Output = ()> + Send + 'static,
    ) {
        self.spawn_generation_execution_scoped(task_id, a4n_queue::scope::with_llm_slot(future));
    }

    pub(crate) fn spawn_image_generation_execution(
        &self,
        task_id: String,
        future: impl Future<Output = ()> + Send + 'static,
    ) {
        self.spawn_generation_execution_scoped(task_id, a4n_queue::scope::with_image_slot(future));
    }

    pub(crate) fn spawn_video_generation_execution(
        &self,
        task_id: String,
        future: impl Future<Output = ()> + Send + 'static,
    ) {
        self.spawn_generation_execution_scoped(task_id, a4n_queue::scope::with_video_slot(future));
    }

    fn spawn_generation_execution_scoped(
        &self,
        task_id: String,
        future: impl Future<Output = ()> + Send + 'static,
    ) {
        let mut workers = self
            .generation_executions
            .workers
            .lock()
            .expect("generation execution mutex poisoned");
        workers.retain(|_, worker| !worker.is_finished());
        assert!(
            !workers.contains_key(&task_id),
            "generation execution owner 已存在: {task_id}"
        );
        workers.insert(
            task_id,
            tokio::spawn(inherit_generation_runtime_context(future)),
        );
    }

    pub(crate) fn destructive_lifecycle(
        &self,
    ) -> &Arc<
        crate::actions::generation_tasks::destructive_lifecycle::DestructiveLifecycleCoordinator,
    > {
        &self.destructive_lifecycle
    }

    pub(crate) fn video(&self) -> super::video::VideoRuntimeFacade<'_> {
        super::video::VideoRuntimeFacade::new(&self.orchestrator)
    }

    pub(crate) fn canvas(&self) -> super::canvas::CanvasRuntimeFacade<'_> {
        super::canvas::CanvasRuntimeFacade::new(&self.orchestrator)
    }

    pub(crate) fn source_sync(&self) -> super::source_sync::SourceSyncRuntimeFacade<'_> {
        super::source_sync::SourceSyncRuntimeFacade::new(&self.orchestrator)
    }

    pub(crate) fn inspiration(&self) -> super::inspiration::InspirationRuntimeFacade<'_> {
        super::inspiration::InspirationRuntimeFacade::new(&self.orchestrator)
    }

    pub(crate) fn profile_material(
        &self,
    ) -> super::profile_material::ProfileMaterialRuntimeFacade<'_> {
        super::profile_material::ProfileMaterialRuntimeFacade::new(&self.orchestrator)
    }

    pub(crate) fn import_reverse(&self) -> super::import_reverse::ImportReverseRuntimeFacade<'_> {
        super::import_reverse::ImportReverseRuntimeFacade::new(&self.orchestrator)
    }

    pub(crate) fn asset_lineage_sync(
        &self,
    ) -> super::asset_lineage_sync::AssetLineageSyncRuntimeFacade<'_> {
        super::asset_lineage_sync::AssetLineageSyncRuntimeFacade::new(&self.orchestrator)
    }

    pub(crate) fn chapter_analysis(
        &self,
    ) -> super::chapter_analysis::ChapterAnalysisRuntimeFacade<'_> {
        super::chapter_analysis::ChapterAnalysisRuntimeFacade::new(&self.orchestrator)
    }

    pub(crate) fn chapter_generation(
        &self,
    ) -> super::chapter_generation::ChapterGenerationRuntimeFacade<'_> {
        super::chapter_generation::ChapterGenerationRuntimeFacade::new(&self.orchestrator)
    }

    pub(crate) fn import_analysis(
        &self,
    ) -> super::import_analysis::ImportAnalysisRuntimeFacade<'_> {
        super::import_analysis::ImportAnalysisRuntimeFacade::new(&self.orchestrator)
    }

    pub(crate) fn task_queue(&self) -> super::task_queue::TaskQueueRuntimeFacade<'_> {
        super::task_queue::TaskQueueRuntimeFacade::new(&self.orchestrator)
    }

    pub(crate) fn media(&self) -> super::media::MediaRuntimeFacade<'_> {
        super::media::MediaRuntimeFacade::new(&self.orchestrator)
    }

    pub(crate) fn manga(&self) -> super::manga::MangaRuntimeFacade<'_> {
        super::manga::MangaRuntimeFacade::new(&self.orchestrator)
    }

    pub(crate) fn workflow(&self) -> super::workflow::WorkflowRuntimeFacade<'_> {
        super::workflow::WorkflowRuntimeFacade::new(&self.orchestrator)
    }

    pub(crate) fn shooting_script(
        &self,
    ) -> super::shooting_script::ShootingScriptRuntimeFacade<'_> {
        super::shooting_script::ShootingScriptRuntimeFacade::new(&self.orchestrator)
    }

    pub fn dynamic_llm_provider_for_profile(
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

    pub async fn new(config: AppConfig) -> Result<Self, a4n_core::error::AppError> {
        Self::new_with_profile(config, RuntimeProfile::from_env()?).await
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
        let prompt_engine = create_prompt_engine(db.as_ref(), &config).await?;
        let chapter = ChapterGenerationService::new(db.clone());
        let rag_store = create_rag_store(&config, runtime_profile, &capabilities).await?;
        let snapshot_store = Arc::new(Sha256SnapshotStore::new(db.clone()));
        let orchestrator = a4n_orchestrator::OrchestratorContext::bootstrap(
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
        let _managed_sidecars = Arc::new(tokio::sync::Mutex::new(managed_sidecar_processes));
        let provider_facade = orchestrator.dynamic_provider_facade();
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
        let orchestrator = orchestrator.with_providers(llm, embedding);

        let state = Self {
            orchestrator,
            runtime_profile,
            destructive_lifecycle: Arc::new(
                crate::actions::generation_tasks::destructive_lifecycle::DestructiveLifecycleCoordinator::default(),
            ),
            generation_executions: Arc::new(GenerationExecutionOwner::default()),
            _generation_owner_lock: generation_owner_lock,
            _data_dir_lock: data_dir_lock,
            _managed_sidecars,
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
        crate::actions::storage_paths::recover_unreferenced_managed_media_files(&state).await;
        crate::cleanup_outbox::recover_cleanup_outbox(&state).await;
        crate::actions::storage_paths::recover_orphan_project_data_dirs(&state).await;
        crate::cleanup_outbox::spawn_cleanup_worker(state.clone());
        Ok(state)
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
                "id": null,
                "name": "request image config",
                "backend": "openai-compatible-image",
                "base_url": "https://example.test/v1",
                "api_key_encrypted": "sk-test",
                "model": "image-test",
                "max_concurrent": 1,
                "default_width": 512,
                "default_height": 512,
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
