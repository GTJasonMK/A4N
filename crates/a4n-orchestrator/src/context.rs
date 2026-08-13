use std::path::PathBuf;
use std::sync::Arc;

use a4n_capabilities::A4nCapabilities;
use a4n_db::DbHandle;

use a4n_rag_core::vector_store::VectorStore;
use a4n_service_manager::ServiceManager;

use crate::chapter_generation::ChapterGenerationService;
use crate::task_queue::PersistentTaskQueue;

mod admin;
mod asset;
mod asset_lineage_sync;
mod auth;
mod canvas;
mod chapter_content;
mod character;
mod collaboration;
mod construction;
pub mod dynamic_provider;
mod galgame;
mod generation;
pub(crate) use generation::GenerationTaskExecution;
pub use generation::{
    GenerationTaskAccessError, GenerationTaskHistorySummaries, GenerationTaskStart,
};
mod import_reverse;
mod import_review;
mod inspiration;
mod llm_context;
mod manga;
mod optimize;
mod outline;
mod profile_material;
mod project_blueprint;
mod project_chapter_context;
mod project_chapter_records;
mod project_dependency_graph;
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
mod rag_store;
mod scene_profile;
mod service_health;
mod settings;
mod shooting_script;
mod source_sync;
mod task_queue;
mod text_anchor;
mod unavailable_providers;
mod video;
mod video_studio_transfer;
mod workflow;

use admin::AdminApplication;
use asset::AssetApplication;
use asset_lineage_sync::AssetLineageSyncApplication;
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
use project_dependency_graph::ProjectDependencyGraphApplication;
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
use unavailable_providers::{BootstrapUnavailableEmbedding, BootstrapUnavailableLlm};
use video::VideoApplication;
use workflow::WorkflowApplication;

/// 宿主业务编排上下文：领域 application owner + 基础设施能力。
#[derive(Clone)]
pub struct OrchestratorContext {
    /// 仅供尚未迁入 application owner 的遗留领域路径；runtime 不得直连。
    pub(crate) database: DbHandle,
    admin: AdminApplication,
    asset: AssetApplication,
    asset_lineage_sync: AssetLineageSyncApplication,
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
    llm_context: LlmContextApplication,
    manga: MangaApplication,
    optimize: OptimizeApplication,
    profile_material: ProfileMaterialApplication,
    project_blueprint: ProjectBlueprintApplication,
    project_chapter_context: ProjectChapterContextApplication,
    project_chapter_records: ProjectChapterRecordsApplication,
    project_dependency_graph: ProjectDependencyGraphApplication,
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
    capabilities: A4nCapabilities,
    service_manager: Arc<ServiceManager>,
    service_pid_dir: Option<PathBuf>,

    /// 应用配置（含 data_dir）；仅 context 子树与用途方法可访问。
    config: a4n_core::config::app_config::AppConfig,
    event_bus: Arc<a4n_core::event::bridge::BroadcastEventBus>,
    task_manager: Arc<a4n_core::event::channel::TaskManager>,
    /// 章节生成服务（进程内 generation lock，必须共享实例）。
    chapter: ChapterGenerationService,
    rag_store: Arc<dyn VectorStore>,
    task_queue: Arc<PersistentTaskQueue>,
    llm: Arc<dyn a4n_llm::provider::LlmProvider>,
    engine: Arc<a4n_prompt::tera_engine::TeraPromptEngine>,
    embedding: Arc<dyn a4n_embedding::provider::EmbeddingProvider>,
    snapshot_store: Arc<a4n_db::character::snapshot_store::Sha256SnapshotStore>,
}
