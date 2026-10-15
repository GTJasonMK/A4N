use std::sync::Arc;

use a4n_db::DbHandle;
use a4n_error::AppError;
use a4n_event::InternalEvent;
use a4n_prompt_client::PromptClient;
use a4n_provider_runtime_contract::ProviderRuntimePort;
use a4n_rag_core::vector_store::VectorStore;
use a4n_service_manager::ServiceManager;

use super::{
    AssetApplication, AuthApplication, CanvasApplication, ChapterContentApplication,
    CharacterApplication, CollaborationApplication, ConfigApplication, GalgameApplication,
    GenerationApplication, ImportReverseApplication, ImportReviewApplication,
    InspirationApplication, LlmContextApplication, MangaApplication, OptimizeApplication,
    OrchestratorContext, ProfileMaterialApplication, ProjectBlueprintApplication,
    ProjectChapterContextApplication, ProjectChapterRecordsApplication,
    ProjectDuplicationApplication, ProjectFamilyApplication, ProjectMutationApplication,
    ProjectOutlineRecordsApplication, ProjectReadApplication, ProjectSceneRecordsApplication,
    ProjectStatusApplication, PromptOverrideApplication, RagApplication, SceneProfileApplication,
    ShootingScriptApplication, SourceSyncApplication, TextAnchorApplication, VideoApplication,
    WorkflowApplication,
};
use crate::chapter_generation::ChapterGenerationService;
use crate::context::dynamic_provider::ProviderRuntimeAdapter;
use crate::import_analysis::ImportAnalysisExecutionLocks;
use a4n_agent_application::agent::AgentApplication;
use a4n_task_runtime::persistent::PersistentTaskQueue;

impl OrchestratorContext {
    pub(crate) fn document_export_renderer(
        &self,
    ) -> Arc<dyn a4n_export_contract::ExportDocumentRenderer> {
        self.document_export_renderer.clone()
    }

    pub(crate) fn prompt_client(&self) -> Option<Arc<dyn PromptClient>> {
        self.prompt_client.clone()
    }

    pub(crate) fn chapter_generation_service(&self) -> ChapterGenerationService {
        self.chapter.clone()
    }

    /// 引导构造：仅用于初始化 LLM/embedding provider 的短暂阶段。
    pub fn bootstrap(
        db: DbHandle,
        prompt_client: Option<Arc<dyn PromptClient>>,
        service_manager: Arc<ServiceManager>,
        config: a4n_config::app_config::AppConfig,
        event_bus: Arc<a4n_event::BroadcastEventBus>,
        task_manager: Arc<a4n_task_runtime::channel::TaskManager>,
        rag_store: Arc<dyn VectorStore>,
        task_queue: Arc<PersistentTaskQueue>,
        engine: Arc<a4n_prompt::tera_engine::TeraPromptEngine>,
        document_export_renderer: Arc<dyn a4n_export_contract::ExportDocumentRenderer>,
        shooting_script_publication: Arc<
            dyn a4n_api_contract::shooting_script::ShootingScriptRecordMutationApplicationPort,
        >,
        provider_factory: impl FnOnce(
            Arc<dyn ProviderRuntimePort>,
        ) -> Result<
            (
                Arc<dyn a4n_llm_provider_contract::provider::LlmProvider>,
                Arc<dyn a4n_embedding_provider_contract::EmbeddingProvider>,
            ),
            AppError,
        >,
    ) -> Result<Self, AppError> {
        let chapter = crate::chapter_generation::ChapterGenerationService::new(db.clone());
        let agent = Arc::new(AgentApplication::new(db.clone()));
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
        let shooting_script =
            ShootingScriptApplication::new(db.clone(), shooting_script_publication);
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
            agent,
            asset,
            auth,
            canvas,
            chapter_content,
            character,
            collaboration,
            galgame,
            generation,
            import_analysis_execution: ImportAnalysisExecutionLocks::default(),
            import_reverse,
            import_review,
            inspiration,
            manga,
            optimize,
            profile_material,
            project_blueprint,
            project_chapter_context,
            project_chapter_records,
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
            prompt_client,
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
            document_export_renderer,
        })
    }

    pub(crate) fn agent(&self) -> &AgentApplication {
        self.agent.as_ref()
    }

    pub fn agent_resource_access_application(
        &self,
    ) -> Arc<dyn a4n_api_contract::agent::AgentResourceAccessApplicationPort> {
        self.agent.clone()
    }

    pub fn agent_worker_application(
        &self,
    ) -> Arc<dyn a4n_agent_runtime_contract::AgentWorkerApplicationPort> {
        self.agent.clone()
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
    ) -> Result<a4n_embedding_provider_contract::EmbeddingModelIdentity, AppError> {
        self.embedding
            .model_identity()
            .await
            .map_err(AppError::from)
    }

    /// 数据目录。
    pub(crate) fn data_dir(&self) -> &std::path::Path {
        &self.config.data_dir
    }

    /// 取消已注册任务句柄。
    pub(crate) fn cancel_task_handle(&self, task_id: &str) -> bool {
        self.task_manager.cancel(task_id)
    }
}
