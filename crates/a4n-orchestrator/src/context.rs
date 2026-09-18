use std::sync::Arc;

use a4n_db::DbHandle;
use a4n_prompt_client::PromptClient;

use a4n_rag_core::vector_store::VectorStore;
use a4n_service_manager::ServiceManager;

use crate::chapter_generation::ChapterGenerationService;
use a4n_task_runtime::persistent::PersistentTaskQueue;

mod admin;
mod asset;
mod auth;
mod canvas;
mod chapter_content;
mod chapter_content_projection;
mod character;
mod collaboration;
mod construction;
mod cost;
pub(super) mod dynamic_provider;
mod galgame;
pub(super) mod generation;
pub(crate) use generation::{GenerationTaskExecution, fail_generation_workflow_execution};
mod import_reverse;
mod import_review;
mod inspiration;
pub(crate) use inspiration::inspiration_db_error_to_app;
mod llm_context;
mod manga;
mod optimize;
mod outline;
mod package;
mod profile_material;
mod project_blueprint;
mod project_bundle_transfer;
mod project_chapter_context;
mod project_chapter_records;
mod project_duplication;
mod project_family;
mod project_family_transfer;
mod project_mutation;
mod project_outline_records;
mod project_read;
mod project_scene_records;
mod project_status;
mod prompt_engine;
mod prompt_override;
mod providers;
mod rag;
mod rag_ingest;
mod rag_search;
mod rag_store;
mod scene_profile;
mod settings;
mod shooting_script;
#[cfg(test)]
pub(crate) use shooting_script::test_shooting_script_publication;
mod source_sync;
mod task_queue;
mod text_anchor;
mod video;
mod video_studio_transfer;
mod workflow;

use a4n_agent_application::agent::AgentApplication;
use admin::AdminApplication;
use asset::AssetApplication;
use auth::AuthApplication;
use canvas::CanvasApplication;
use chapter_content::ChapterContentApplication;
use character::CharacterApplication;
use collaboration::CollaborationApplication;
use galgame::GalgameApplication;
use generation::GenerationApplication;
use import_reverse::ImportReverseApplication;
use import_review::ImportReviewApplication;
use inspiration::InspirationApplication;
use llm_context::LlmContextApplication;
use manga::MangaApplication;
use optimize::OptimizeApplication;
use profile_material::ProfileMaterialApplication;
use project_blueprint::ProjectBlueprintApplication;
use project_chapter_context::ProjectChapterContextApplication;
use project_chapter_records::ProjectChapterRecordsApplication;
use project_duplication::ProjectDuplicationApplication;
use project_family::ProjectFamilyApplication;
use project_mutation::ProjectMutationApplication;
use project_outline_records::ProjectOutlineRecordsApplication;
use project_read::ProjectReadApplication;
use project_scene_records::ProjectSceneRecordsApplication;
use project_status::ProjectStatusApplication;
use prompt_override::PromptOverrideApplication;
use rag::RagApplication;
use scene_profile::SceneProfileApplication;
use settings::ConfigApplication;
use shooting_script::ShootingScriptApplication;
use source_sync::SourceSyncApplication;
use text_anchor::TextAnchorApplication;
use video::VideoApplication;
use workflow::WorkflowApplication;

/// 宿主业务编排上下文：领域 application owner + 基础设施能力。
#[derive(Clone)]
pub struct OrchestratorContext {
    /// 仅供尚未迁入 application owner 的遗留领域路径；runtime 不得直连。
    pub(crate) database: DbHandle,
    admin: AdminApplication,
    agent: Arc<AgentApplication>,
    asset: AssetApplication,
    auth: AuthApplication,
    canvas: CanvasApplication,
    chapter_content: ChapterContentApplication,
    character: CharacterApplication,
    collaboration: CollaborationApplication,
    galgame: GalgameApplication,
    generation: GenerationApplication,
    import_reverse: ImportReverseApplication,
    import_review: ImportReviewApplication,
    inspiration: InspirationApplication,
    manga: MangaApplication,
    optimize: OptimizeApplication,
    profile_material: ProfileMaterialApplication,
    project_blueprint: ProjectBlueprintApplication,
    project_chapter_context: ProjectChapterContextApplication,
    project_chapter_records: ProjectChapterRecordsApplication,
    project_duplication: ProjectDuplicationApplication,
    project_family: ProjectFamilyApplication,
    project_mutation: ProjectMutationApplication,
    project_outline_records: ProjectOutlineRecordsApplication,
    project_read: ProjectReadApplication,
    project_scene_records: ProjectSceneRecordsApplication,
    project_status: ProjectStatusApplication,
    prompt_override: PromptOverrideApplication,
    rag: RagApplication,
    scene_profile: SceneProfileApplication,
    settings: ConfigApplication,
    shooting_script: ShootingScriptApplication,
    source_sync: SourceSyncApplication,
    text_anchor: TextAnchorApplication,
    video: VideoApplication,
    workflow: WorkflowApplication,
    prompt_client: Option<Arc<dyn PromptClient>>,
    service_manager: Arc<ServiceManager>,

    /// 应用配置（含 data_dir）；仅 context 子树与用途方法可访问。
    config: a4n_config::app_config::AppConfig,
    event_bus: Arc<a4n_event::BroadcastEventBus>,
    task_manager: Arc<a4n_task_runtime::channel::TaskManager>,
    /// 章节生成服务（进程内 generation lock，必须共享实例）。
    chapter: ChapterGenerationService,
    rag_store: Arc<dyn VectorStore>,
    task_queue: Arc<PersistentTaskQueue>,
    llm: Arc<dyn a4n_llm_provider_contract::provider::LlmProvider>,
    engine: Arc<a4n_prompt::tera_engine::TeraPromptEngine>,
    embedding: Arc<dyn a4n_embedding_provider_contract::EmbeddingProvider>,
    snapshot_store: Arc<a4n_db::character::Sha256SnapshotStore>,
    document_export_renderer: Option<Arc<dyn a4n_export_contract::ExportDocumentRenderer>>,
}
