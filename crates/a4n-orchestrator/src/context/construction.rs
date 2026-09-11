use std::sync::Arc;

use a4n_capabilities::A4nCapabilities;
use a4n_db::DbHandle;
use a4n_error::AppError;
use a4n_event::InternalEvent;
use a4n_provider_runtime_contract::ProviderRuntimePort;
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
use crate::agent::AgentApplication;
use crate::chapter_generation::ChapterGenerationService;
use crate::context::dynamic_provider::ProviderRuntimeAdapter;
use crate::task_queue::PersistentTaskQueue;

impl OrchestratorContext {
    pub(crate) fn capabilities(&self) -> &A4nCapabilities {
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
        config: a4n_config::app_config::AppConfig,
        event_bus: Arc<a4n_event::BroadcastEventBus>,
        task_manager: Arc<a4n_task_runtime::channel::TaskManager>,
        chapter: ChapterGenerationService,
        rag_store: Arc<dyn VectorStore>,
        task_queue: Arc<PersistentTaskQueue>,
        engine: Arc<a4n_prompt::tera_engine::TeraPromptEngine>,
        snapshot_store: Arc<a4n_db::character::Sha256SnapshotStore>,
        provider_factory: impl FnOnce(
            Arc<dyn ProviderRuntimePort>,
        ) -> Result<
            (
                Arc<dyn a4n_llm::provider::LlmProvider>,
                Arc<dyn a4n_embedding::provider::EmbeddingProvider>,
            ),
            AppError,
        >,
    ) -> Result<Self, AppError> {
        let admin = AdminApplication::new(db.clone());
        let agent = AgentApplication::new(db.clone());
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
        let provider_port: Arc<dyn ProviderRuntimePort> =
            Arc::new(ProviderRuntimeAdapter::from_applications(
                Arc::clone(&task_queue),
                settings.clone(),
                generation.clone(),
                llm_context.clone(),
                project_read.clone(),
            ));
        let (llm, embedding) = provider_factory(provider_port)?;
        Ok(Self {
            database: db,
            admin,
            agent,
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

    pub fn agent(&self) -> &AgentApplication {
        &self.agent
    }

    /// 发布内部事件（状态/蓝图等），统一经 EventBus，避免旁路直连。
    pub(crate) fn publish_internal_event(&self, event: InternalEvent) -> Result<usize, AppError> {
        self.event_bus.publish(event)
    }
}

impl OrchestratorContext {
    /// embedding 批大小（runtime/RAG 同步用，不暴露完整 provider）。
    pub(crate) fn embedding_batch_size(&self) -> usize {
        self.embedding.batch_size()
    }

    /// embedding 模型身份（runtime/RAG 同步用）。
    pub(crate) async fn embedding_model_identity(
        &self,
    ) -> Result<a4n_embedding::provider::EmbeddingModelIdentity, AppError> {
        self.embedding
            .model_identity()
            .await
            .map_err(AppError::from)
    }

    /// 数据目录。
    pub(crate) fn data_dir(&self) -> &std::path::Path {
        &self.config.data_dir
    }

    /// 日志目录。
    pub(crate) fn log_dir(&self) -> &std::path::Path {
        &self.config.log_dir
    }

    /// 临时目录。
    pub(crate) fn temp_dir(&self) -> &std::path::Path {
        &self.config.temp_dir
    }

    /// Embedding 模型缓存目录。
    pub(crate) fn embedding_models_dir(&self) -> std::path::PathBuf {
        self.config.embedding_models_dir()
    }

    /// 取消已注册任务句柄。
    pub(crate) fn cancel_task_handle(&self, task_id: &str) -> bool {
        self.task_manager.cancel(task_id)
    }

    /// 按内容哈希加载角色快照（不暴露 snapshot_store 句柄）。
    pub(crate) async fn character_snapshot_by_hash(
        &self,
        content_hash: &str,
    ) -> Result<Option<a4n_domain::snapshot::Snapshot>, AppError> {
        self.snapshot_store.load_snapshot(content_hash).await
    }
}
