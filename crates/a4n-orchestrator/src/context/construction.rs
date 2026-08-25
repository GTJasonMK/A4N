use std::sync::Arc;

use a4n_capabilities::A4nCapabilities;
use a4n_core::error::AppError;
use a4n_core::event::bridge::InternalEvent;
use a4n_db::DbHandle;
use a4n_rag_core::vector_store::VectorStore;
use a4n_service_manager::ServiceManager;

use super::{
    AdminApplication, AssetApplication, AuthApplication, CanvasApplication,
    ChapterContentApplication, CharacterApplication, CollaborationApplication, ConfigApplication,
    GalgameApplication, GenerationApplication, ImportReverseApplication, ImportReviewApplication,
    InspirationApplication, LlmContextApplication, MangaApplication, OptimizeApplication,
    OrchestratorContext, ProfileMaterialApplication, ProjectBlueprintApplication,
    ProjectChapterContextApplication, ProjectChapterRecordsApplication,
    ProjectDependencyGraphApplication, ProjectDuplicationApplication, ProjectFamilyApplication,
    ProjectMutationApplication, ProjectOutlineRecordsApplication, ProjectReadApplication,
    ProjectSceneRecordsApplication, ProjectStatusApplication, PromptOverrideApplication,
    RagApplication, SceneProfileApplication, ShootingScriptApplication, SourceSyncApplication,
    TextAnchorApplication, VideoApplication, WorkflowApplication,
};
use crate::chapter_generation::ChapterGenerationService;
use crate::context::dynamic_provider::DynamicProviderFacade;
use crate::task_queue::PersistentTaskQueue;

impl OrchestratorContext {
    pub fn capabilities(&self) -> &A4nCapabilities {
        &self.capabilities
    }

    pub(crate) fn chapter_generation_service(&self) -> ChapterGenerationService {
        self.chapter.clone()
    }

    /// 引导构造：仅用于初始化 LLM/embedding provider 的短暂阶段。
    pub fn bootstrap(
        db: DbHandle,
        capabilities: A4nCapabilities,
        service_manager: Arc<ServiceManager>,
        config: a4n_core::config::app_config::AppConfig,
        event_bus: Arc<a4n_core::event::bridge::BroadcastEventBus>,
        task_manager: Arc<a4n_core::event::channel::TaskManager>,
        chapter: ChapterGenerationService,
        rag_store: Arc<dyn VectorStore>,
        task_queue: Arc<PersistentTaskQueue>,
        engine: Arc<a4n_prompt::tera_engine::TeraPromptEngine>,
        snapshot_store: Arc<a4n_db::character::Sha256SnapshotStore>,
        provider_factory: impl FnOnce(
            DynamicProviderFacade,
        ) -> Result<
            (
                Arc<dyn a4n_llm::provider::LlmProvider>,
                Arc<dyn a4n_embedding::provider::EmbeddingProvider>,
            ),
            AppError,
        >,
    ) -> Result<Self, AppError> {
        let admin = AdminApplication::new(db.clone());
        let asset = AssetApplication::new(db.clone());
        let auth = AuthApplication::new(db.clone());
        let canvas = CanvasApplication::new(db.clone());
        let chapter_content = ChapterContentApplication::new(db.clone());
        let character = CharacterApplication::new(db.clone());
        let collaboration = CollaborationApplication::new(db.clone());
        let galgame = GalgameApplication::new(db.clone());
        let generation = GenerationApplication::new(db.clone(), Arc::clone(&task_queue));
        let import_reverse = ImportReverseApplication::new(db.clone());
        let import_review = ImportReviewApplication::new(db.clone());
        let inspiration = InspirationApplication::new(db.clone());
        let llm_context = LlmContextApplication::new(db.clone());
        let manga = MangaApplication::new(db.clone());
        let optimize = OptimizeApplication::new(db.clone());
        let profile_material = ProfileMaterialApplication::new(db.clone());
        let project_blueprint = ProjectBlueprintApplication::new(db.clone());
        let project_chapter_context = ProjectChapterContextApplication::new(db.clone());
        let project_chapter_records = ProjectChapterRecordsApplication::new(db.clone());
        let project_dependency_graph = ProjectDependencyGraphApplication::new(db.clone());
        let project_duplication = ProjectDuplicationApplication::new(db.clone());
        let project_family = ProjectFamilyApplication::new(db.clone());
        let project_mutation = ProjectMutationApplication::new(db.clone());
        let project_outline_records = ProjectOutlineRecordsApplication::new(db.clone());
        let project_read = ProjectReadApplication::new(db.clone());
        let project_scene_records = ProjectSceneRecordsApplication::new(db.clone());
        let project_status = ProjectStatusApplication::new(db.clone());
        let prompt_override = PromptOverrideApplication::new(db.clone());
        let rag = RagApplication::new(db.clone());
        let scene_profile = SceneProfileApplication::new(db.clone());
        let settings = ConfigApplication::new(db.clone());
        let shooting_script = ShootingScriptApplication::new(db.clone());
        let source_sync = SourceSyncApplication::new(db.clone());
        let text_anchor = TextAnchorApplication::new(db.clone());
        let video = VideoApplication::new(db.clone());
        let workflow = WorkflowApplication::new(db.clone());
        let (llm, embedding) = provider_factory(DynamicProviderFacade::from_applications(
            Arc::clone(&task_queue),
            settings.clone(),
            generation.clone(),
            llm_context.clone(),
            project_read.clone(),
        ))?;
        Ok(Self {
            database: db,
            admin,
            asset,
            auth,
            canvas,
            chapter_content,
            character,
            collaboration,
            galgame,
            generation,
            import_reverse,
            import_review,
            inspiration,
            llm_context,
            manga,
            optimize,
            profile_material,
            project_blueprint,
            project_chapter_context,
            project_chapter_records,
            project_dependency_graph,
            project_duplication,
            project_family,
            project_mutation,
            project_outline_records,
            project_read,
            project_scene_records,
            project_status,
            prompt_override,
            rag,
            scene_profile,
            settings,
            shooting_script,
            source_sync,
            text_anchor,
            video,
            workflow,
            capabilities,
            service_manager,
            config,
            event_bus,
            task_manager,
            chapter,
            rag_store,
            task_queue,
            llm,
            engine,
            embedding,
            snapshot_store,
        })
    }

    /// 发布内部事件（状态/蓝图等），统一经 EventBus，避免旁路直连。
    pub fn publish_internal_event(&self, event: InternalEvent) -> Result<usize, AppError> {
        self.event_bus.publish(event)
    }

    /// 订阅内部事件流（Host/Tauri 桥接用）。
    pub fn subscribe_internal_events(&self) -> tokio::sync::broadcast::Receiver<InternalEvent> {
        self.event_bus.subscribe()
    }
}

impl OrchestratorContext {
    /// embedding 批大小（runtime/RAG 同步用，不暴露完整 provider）。
    pub fn embedding_batch_size(&self) -> usize {
        self.embedding.batch_size()
    }

    /// embedding 模型身份（runtime/RAG 同步用）。
    pub async fn embedding_model_identity(
        &self,
    ) -> Result<a4n_embedding::provider::EmbeddingModelIdentity, AppError> {
        self.embedding.model_identity().await
    }

    /// 要求配置中已设置非空 JWT secret。
    pub fn required_jwt_secret(&self) -> Result<&str, AppError> {
        let secret = self
            .config
            .security
            .jwt_secret
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .ok_or_else(|| AppError::InvalidState {
                current: "jwt_secret 未配置".to_string(),
                operation: "读取 JWT 密钥".to_string(),
            })?;
        Ok(secret)
    }

    /// Host 窗口/路径初始化用配置视图（不暴露字段）。
    pub fn host_app_config(&self) -> &a4n_core::config::app_config::AppConfig {
        &self.config
    }

    /// 数据目录。
    pub fn data_dir(&self) -> &std::path::Path {
        &self.config.data_dir
    }

    /// 日志目录。
    pub fn log_dir(&self) -> &std::path::Path {
        &self.config.log_dir
    }

    /// 临时目录。
    pub fn temp_dir(&self) -> &std::path::Path {
        &self.config.temp_dir
    }

    /// 应用路径聚合。
    pub fn app_paths(&self) -> a4n_core::files::paths::AppPaths {
        a4n_core::files::paths::AppPaths::from_config(&self.config)
    }

    /// 配置中的数据库 URL。
    pub fn configured_database_url(&self) -> &str {
        &self.config.database_url
    }

    /// Embedding 模型缓存目录。
    pub fn embedding_models_dir(&self) -> std::path::PathBuf {
        self.config.embedding_models_dir()
    }

    /// 注册任务句柄。
    pub fn register_task_handle(
        &self,
        handle: a4n_core::event::channel::TaskHandle,
    ) -> Result<std::sync::Arc<a4n_core::event::channel::TaskHandle>, AppError> {
        self.task_manager.register(handle)
    }

    /// 移除任务句柄。
    pub fn remove_task_handle(
        &self,
        handle: &std::sync::Arc<a4n_core::event::channel::TaskHandle>,
    ) -> Result<(), AppError> {
        self.task_manager.remove(handle)
    }

    pub fn task_handle(
        &self,
        task_id: &str,
    ) -> Option<std::sync::Arc<a4n_core::event::channel::TaskHandle>> {
        self.task_manager.get(task_id)
    }

    /// 取消已注册任务句柄。
    pub fn cancel_task_handle(&self, task_id: &str) -> bool {
        self.task_manager.cancel(task_id)
    }

    pub fn cancel_owned_task_handle(&self, task_id: &str, user_id: i64) -> bool {
        self.task_manager.cancel_owned(task_id, user_id)
    }

    /// 取消任务；任务尚未注册时保留短期取消意图。
    pub fn cancel_or_remember_task_handle(&self, task_id: &str, user_id: i64) -> bool {
        self.task_manager.cancel_or_remember(task_id, user_id)
    }

    /// 当前活跃任务快照。
    pub fn active_task_snapshots(&self) -> Vec<a4n_core::event::channel::TaskSnapshot> {
        self.task_manager.list_active_snapshots()
    }

    /// 按内容哈希加载角色快照（不暴露 snapshot_store 句柄）。
    pub(crate) async fn character_snapshot_by_hash(
        &self,
        content_hash: &str,
    ) -> Result<Option<a4n_core::event::snapshot::Snapshot>, AppError> {
        self.snapshot_store.load_snapshot(content_hash).await
    }
}
