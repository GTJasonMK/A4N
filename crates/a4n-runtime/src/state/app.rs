use std::{
    collections::{HashMap, HashSet},
    fs::{File, OpenOptions, TryLockError},
    future::Future,
    path::Path,
    pin::Pin,
    sync::Arc,
};

use a4n_api_contract::generation_tasks::GenerationTaskHandle;
use a4n_config::app_config::AppConfig;
use a4n_db::character::Sha256SnapshotStore;
use a4n_event::{BroadcastEventBus, InternalEvent};
use a4n_orchestrator::chapter_generation::ChapterGenerationService;
use a4n_orchestrator::task_queue::PersistentTaskQueue;
use a4n_worker_contract::{
    AGENT_HOST_COMMAND_OPERATION, AgentHostCommandPayload, AgentRunPayload, WorkerRequest,
};
use serde_json::Value;

use super::database::{
    acquire_generation_owner_lock, run_startup_migrations, validate_runtime_config,
};
use super::profile::{RuntimeProfile, RuntimeRole};
use super::prompt_engine::create_prompt_engine;
use super::rag::create_rag_store;
use crate::sidecar_clients::RuntimeCapabilities;

/// Runtime owns Host lifecycle state and receives business behavior through
/// explicit application ports.
#[derive(Clone)]
pub struct AppState {
    /// Runtime-owned configuration view used by Host paths and adapters.
    config: AppConfig,
    /// Process-local task handles and snapshots are owned by a4n-task-runtime.
    /// The same instance is injected into the orchestrator for workflow code.
    task_manager: Arc<a4n_task_runtime::channel::TaskManager>,
    /// Generation task queries and subscriptions stay behind a narrow read
    /// boundary.
    generation_task_read_application: Arc<dyn a4n_orchestrator::GenerationTaskReadApplicationPort>,
    generation_task_mutation_application:
        Arc<dyn a4n_orchestrator::GenerationTaskMutationApplicationPort>,
    generation_lifecycle_application: Arc<dyn a4n_orchestrator::GenerationLifecycleApplicationPort>,
    generation_cleanup_application: Arc<dyn a4n_orchestrator::GenerationCleanupApplicationPort>,
    /// Source-sync preview authorization and projection stay in the
    /// source-sync owner.
    source_sync_read_application:
        Arc<dyn a4n_api_contract::source_sync::SourceSyncReadApplicationPort>,
    /// Source-sync decisions and detach state transitions stay in the
    /// source-sync owner.
    source_sync_mutation_application:
        Arc<dyn a4n_api_contract::source_sync::SourceSyncMutationApplicationPort>,
    /// Persisted shooting-script reads stay behind the API contract;
    /// generation workers use a separate execution boundary.
    shooting_script_record_read_application:
        Arc<dyn a4n_api_contract::shooting_script::ShootingScriptRecordReadApplicationPort>,
    shooting_script_record_mutation_application:
        Arc<dyn a4n_api_contract::shooting_script::ShootingScriptRecordMutationApplicationPort>,
    shooting_script_generation_application:
        Arc<dyn a4n_orchestrator::ShootingScriptGenerationApplicationPort>,
    /// Host-facing authentication read boundary. Passwords, sessions and ORM
    /// models remain owned by the authentication application.
    auth_read_application: Arc<dyn a4n_api_contract::auth::AuthReadApplicationPort>,
    auth_mutation_application: Arc<dyn a4n_api_contract::auth::AuthMutationApplicationPort>,
    /// Host-facing administrative read and authorization boundary. Admin
    /// projections stay in the orchestrator owner.
    admin_read_application: Arc<dyn a4n_api_contract::admin::AdminReadApplicationPort>,
    admin_mutation_application: Arc<dyn a4n_api_contract::admin::AdminMutationApplicationPort>,
    /// Host-facing project status boundary. Status transitions and family
    /// projections stay in the project owner and return API DTOs only.
    project_status_application: Arc<dyn a4n_api_contract::project::ProjectStatusApplicationPort>,
    /// Host-facing project collaboration boundary. Membership and invitation
    /// state transitions stay in the project collaboration owner.
    project_collaboration_application:
        Arc<dyn a4n_api_contract::collaboration::ProjectCollaborationApplicationPort>,
    /// Host-facing project access boundary. Permission checks and root
    /// identity stay with the project owner.
    project_access_application:
        Arc<dyn a4n_api_contract::collaboration::ProjectAccessApplicationPort>,
    /// Host-facing project read boundary. Runtime receives complete API
    /// summaries and does not assemble them from ORM projections.
    project_read_application: Arc<dyn a4n_api_contract::project::ProjectReadApplicationPort>,
    /// Host-facing project mutation boundary. Project validation and summary
    /// projection remain in the project owner.
    project_mutation_application:
        Arc<dyn a4n_api_contract::project::ProjectMutationApplicationPort>,
    /// Host-facing project asset graph boundary.
    project_asset_graph_application:
        Arc<dyn a4n_api_contract::project::ProjectAssetGraphApplicationPort>,
    /// Project duplication owner. Runtime supplies only media copy/rollback
    /// adapters and receives the final API summary.
    project_duplication_application:
        Arc<dyn a4n_api_contract::project::ProjectDuplicationApplicationPort>,
    /// Project-family package transfer owner. Runtime supplies archive/file
    /// adapters and receives summaries rather than ORM records.
    project_family_transfer_application:
        Arc<dyn a4n_api_contract::project_transfer::ProjectFamilyTransferApplicationPort>,
    /// Video-studio package transfer owner. Runtime supplies archive/file
    /// adapters and receives summaries rather than ORM records.
    video_studio_transfer_application:
        Arc<dyn a4n_api_contract::project_transfer::VideoStudioProjectTransferApplicationPort>,
    /// Host-facing chapter scene boundary.
    chapter_scene_application: Arc<dyn a4n_api_contract::chapter::ChapterSceneApplicationPort>,
    /// Chapter-scene generation and request construction remain in the scene
    /// owner; Runtime only supplies prompt and progress adapters.
    chapter_scene_generation_application:
        Arc<dyn a4n_orchestrator::chapter_scene::ChapterSceneGenerationApplicationPort>,
    /// Content optimization sessions, projections and generation workers stay
    /// in the optimize owner; Runtime only routes Host actions.
    optimize_application: Arc<dyn a4n_orchestrator::optimize::OptimizeApplicationPort>,
    /// Scene-profile synchronization remains owned by the scene-profile
    /// application; Runtime supplies only the RAG host adapter.
    scene_profile_sync_application:
        Arc<dyn a4n_orchestrator::scene_profile::SceneProfileSyncApplicationPort>,
    /// Managed service health queries stay with the service manager owner.
    service_health_application:
        Arc<dyn a4n_api_contract::service_health::ServiceHealthApplicationPort>,
    /// Cleanup claiming, leasing and audit transitions stay with the cleanup owner.
    cleanup_outbox_application: Arc<dyn a4n_cleanup_contract::CleanupOutboxApplicationPort>,
    /// Character discovery and timeline synchronization remain owned by the
    /// character application; Runtime only adapts progress and RAG tracing.
    character_ai_application: Arc<dyn a4n_orchestrator::character::CharacterAiApplicationPort>,
    /// Host-facing workflow read boundary. Runtime keeps only live handles
    /// and cancellation coordination outside this port.
    workflow_read_application: Arc<dyn a4n_api_contract::workflow::WorkflowReadApplicationPort>,
    /// Runtime task/workflow lookup boundary. ORM records remain owned by the
    /// workflow application and are projected to narrow records here.
    workflow_task_lookup_application:
        Arc<dyn a4n_api_contract::workflow::WorkflowTaskLookupApplicationPort>,
    /// Host-facing task history read boundary. Task access checks remain in
    /// Runtime; persistence and summary projection stay with the task owner.
    task_read_application: Arc<dyn a4n_api_contract::task::TaskReadApplicationPort>,
    /// Standalone queue cancellation boundary for tasks without a workflow.
    task_cancellation_application: Arc<dyn a4n_api_contract::task::TaskCancellationPort>,
    /// Host-facing scheduler statistics boundary. Queue facts and API
    /// projection stay with the task queue owner.
    task_queue_stats_application: Arc<dyn a4n_api_contract::task::TaskQueueStatsApplicationPort>,
    /// Workflow cancellation remains owned by Orchestrator; workers receive
    /// only the narrow side-effecting operation.
    workflow_cancellation_application:
        Arc<dyn a4n_api_contract::workflow::WorkflowCancellationPort>,
    /// Host-facing object profile read boundary.
    object_profile_read_application:
        Arc<dyn a4n_api_contract::profile::ObjectProfileReadApplicationPort>,
    /// Host-facing object profile mutation boundary.
    object_profile_mutation_application:
        Arc<dyn a4n_api_contract::profile::ObjectProfileMutationApplicationPort>,
    /// Host-facing scene profile read boundary.
    scene_profile_read_application:
        Arc<dyn a4n_api_contract::profile::SceneProfileReadApplicationPort>,
    /// Host-facing character profile read boundary.
    character_profile_read_application:
        Arc<dyn a4n_api_contract::character::CharacterProfileReadApplicationPort>,
    /// Host-facing character profile mutation boundary. Runtime keeps only
    /// post-commit RAG warning handling outside the character owner.
    character_profile_mutation_application:
        Arc<dyn a4n_api_contract::character::CharacterProfileMutationApplicationPort>,
    /// Host-facing character timeline mutation boundary. Runtime only scopes
    /// the RAG trace and forwards rollback commands.
    character_profile_timeline_mutation_application:
        Arc<dyn a4n_api_contract::character::CharacterProfileTimelineMutationApplicationPort>,
    /// Host-facing explicit character attribute mutation boundary.
    character_profile_attribute_mutation_application:
        Arc<dyn a4n_api_contract::character::CharacterProfileAttributeMutationApplicationPort>,
    /// Host-facing attribute deletion-mark mutation boundary.
    character_profile_attribute_deletion_application:
        Arc<dyn a4n_api_contract::character::CharacterProfileAttributeDeletionApplicationPort>,
    /// Character implicit-review owner. Runtime supplies prompt rendering and
    /// receives already-projected API results.
    character_implicit_review_application:
        Arc<dyn a4n_orchestrator::character::CharacterImplicitReviewApplicationPort>,
    /// Host-facing implicit-review read boundary. Summary queries stay in the
    /// API contract rather than the LLM decision workflow boundary.
    character_implicit_review_read_application:
        Arc<dyn a4n_api_contract::character::CharacterImplicitReviewReadApplicationPort>,
    /// Host-facing character profile deletion boundary.
    character_profile_deletion_application:
        Arc<dyn a4n_api_contract::character::CharacterProfileDeletionApplicationPort>,
    /// Character-consistency workflow owner. Runtime supplies only prompt,
    /// progress, and task lifecycle adapters.
    character_conflict_application:
        Arc<dyn a4n_orchestrator::character::CharacterConflictApplicationPort>,
    /// Host-facing scene profile mutation boundary.
    scene_profile_mutation_application:
        Arc<dyn a4n_api_contract::profile::SceneProfileMutationApplicationPort>,
    /// Host-facing foreshadowing read boundary. Authorization and DTO
    /// projection stay in the owner.
    foreshadowing_read_application:
        Arc<dyn a4n_api_contract::profile::ForeshadowingReadApplicationPort>,
    foreshadowing_mutation_application:
        Arc<dyn a4n_api_contract::profile::ForeshadowingMutationApplicationPort>,
    /// Host-facing blueprint read boundary.
    blueprint_read_application: Arc<dyn a4n_api_contract::blueprint::BlueprintReadApplicationPort>,
    /// Blueprint request construction boundary. Runtime supplies only prompt
    /// rendering for the preview response.
    blueprint_request_preview_application:
        Arc<dyn a4n_orchestrator::BlueprintRequestPreviewApplicationPort>,
    /// Blueprint generation owner. Runtime supplies task workers and prompt
    /// rendering; workflow persistence stays with the blueprint owner.
    blueprint_generation_application: Arc<dyn a4n_orchestrator::BlueprintGenerationApplicationPort>,
    /// Host-facing blueprint mutation boundary. Durable version writes remain
    /// owned by the blueprint application.
    blueprint_mutation_application:
        Arc<dyn a4n_api_contract::blueprint::BlueprintMutationApplicationPort>,
    /// Host-facing chapter content read boundary.
    chapter_content_read_application:
        Arc<dyn a4n_api_contract::chapter::ChapterContentReadApplicationPort>,
    /// Host-facing chapter content mutation boundary. Workflow side effects
    /// are supplied through the explicit cleanup host port at call time.
    chapter_content_mutation_application:
        Arc<dyn a4n_api_contract::chapter::ChapterContentMutationApplicationPort>,
    /// Host-facing chapter-content derived-data synchronization and recovery
    /// boundary.
    chapter_content_sync_application:
        Arc<dyn a4n_api_contract::chapter::ChapterContentSyncApplicationPort>,
    /// Chapter request preview and RAG preparation remain in the chapter
    /// owner; Runtime only supplies prompt and embedding inputs.
    chapter_generation_preview_application:
        Arc<dyn a4n_orchestrator::chapter_generation::ChapterGenerationPreviewApplicationPort>,
    /// Chapter generation task owner. Runtime supplies prompt, embedding and
    /// cleanup adapters when a worker is started.
    chapter_generation_application:
        Arc<dyn a4n_orchestrator::chapter_generation::ChapterGenerationApplicationPort>,
    /// Host-facing outline read boundary.
    outline_read_application: Arc<dyn a4n_api_contract::outline::OutlineReadApplicationPort>,
    /// Host-facing outline mutation boundary.
    outline_mutation_application:
        Arc<dyn a4n_api_contract::outline::OutlineMutationApplicationPort>,
    /// Outline generation task owner. Runtime only schedules the worker.
    outline_generation_application:
        Arc<dyn a4n_orchestrator::outline::OutlineGenerationApplicationPort>,
    /// Outline request-preview owner. Runtime formats API preview responses
    /// but does not construct prompt requests.
    outline_preview_application: Arc<dyn a4n_orchestrator::OutlinePreviewApplicationPort>,
    /// Host-facing theme/settings read boundary. Theme ownership and metadata
    /// stay in the settings application; Runtime handles only protected files.
    theme_read_application: Arc<dyn a4n_api_contract::theme::ThemeReadApplicationPort>,
    theme_mutation_application: Arc<dyn a4n_api_contract::theme::ThemeMutationApplicationPort>,
    /// Host-facing AI configuration read boundary. Validation, authorization
    /// and DTO projection stay in the configuration owner.
    config_read_application: Arc<dyn a4n_api_contract::config::ConfigReadApplicationPort>,
    config_mutation_application: Arc<dyn a4n_api_contract::config::ConfigMutationApplicationPort>,
    /// Host-facing settings export/import boundary. Authorization and
    /// rollback semantics stay in the settings owner.
    settings_all_application: Arc<dyn a4n_api_contract::config::SettingsAllApplicationPort>,
    /// Host-facing prompt record read boundary. Prompt service selection stays
    /// with the prompt owner.
    prompt_read_application: Arc<dyn a4n_orchestrator::prompt_records::PromptReadApplicationPort>,
    prompt_mutation_application:
        Arc<dyn a4n_orchestrator::prompt_records::PromptMutationApplicationPort>,
    /// Host-facing Agent application port. Worker-only lease operations stay
    /// on the orchestrator implementation and are intentionally not exposed
    /// through this port.
    agent_application: Arc<dyn a4n_api_contract::agent::AgentApplicationPort>,
    /// Agent worker ports used only by the durable Runtime worker. Keeping the
    /// aggregate behind its contract avoids coupling Runtime to the owner.
    agent_worker_application: Arc<dyn a4n_agent_runtime_contract::AgentWorkerApplicationPort>,
    /// Host-facing Canvas read port. Graph permissions, versioning and
    /// persistence stay with the Canvas owner; Runtime only routes actions.
    canvas_read_application: Arc<dyn a4n_api_contract::canvas::CanvasReadApplicationPort>,
    canvas_mutation_application: Arc<dyn a4n_api_contract::canvas::CanvasMutationApplicationPort>,
    /// Canvas RenderText owner. Runtime supplies only node identity and the
    /// shared workflow snapshot.
    canvas_render_text_application:
        Arc<dyn a4n_canvas_contract::CanvasRenderTextNodeApplicationPort>,
    /// Canvas LLM text owner. Runtime supplies the LLM HostPort and only node
    /// identity plus the shared workflow snapshot.
    canvas_llm_text_application:
        Arc<dyn a4n_orchestrator::canvas::CanvasLlmTextNodeApplicationPort>,
    /// Canvas image-generation owner. Runtime supplies only the image HostPort
    /// and a shared workflow snapshot.
    canvas_image_application: Arc<dyn a4n_orchestrator::canvas::CanvasImageNodeApplicationPort>,
    /// Canvas video-generation owner. Runtime supplies only the video HostPort
    /// and a shared workflow snapshot.
    canvas_video_application: Arc<dyn a4n_orchestrator::canvas::CanvasVideoNodeApplicationPort>,
    /// Canvas request-preview owner. Runtime receives prepared pure requests
    /// and renders provider-specific preview steps.
    canvas_preview_application: Arc<dyn a4n_orchestrator::canvas::CanvasPreviewApplicationPort>,
    /// Canvas editing node owner. Runtime supplies only media editing HostPort
    /// and a shared workflow snapshot.
    canvas_editing_application: Arc<dyn a4n_canvas_contract::CanvasEditingNodeApplicationPort>,
    /// Canvas probe node owner. Runtime supplies only node identity and the
    /// shared workflow snapshot.
    canvas_probe_application: Arc<dyn a4n_canvas_contract::CanvasProbeNodeApplicationPort>,
    /// Canvas output node owner. Runtime supplies only node identity and the
    /// shared workflow snapshot.
    canvas_output_application: Arc<dyn a4n_canvas_contract::CanvasOutputNodeApplicationPort>,
    /// Canvas media import owner. Runtime supplies upload/file adapters and
    /// receives API summaries only.
    canvas_media_import_application: Arc<dyn a4n_canvas_contract::CanvasMediaImportApplicationPort>,
    /// Canvas graph workflow preparation and terminal transition owner.
    canvas_graph_execution_application:
        Arc<dyn a4n_canvas_contract::CanvasGraphExecutionApplicationPort>,
    /// Host-facing package state read port. Lifecycle workers keep package
    /// ownership behind the narrow read/mutation contracts.
    package_read_application: Arc<dyn a4n_api_contract::package::PackageReadApplicationPort>,
    package_mutation_application:
        Arc<dyn a4n_api_contract::package::PackageMutationApplicationPort>,
    /// Host-facing import-analysis progress boundary.
    import_analysis_progress_application:
        Arc<dyn a4n_api_contract::import::ImportAnalysisProgressApplicationPort>,
    /// Host-facing import-analysis request-preview boundary.
    import_analysis_preview_application:
        Arc<dyn a4n_orchestrator::import_analysis::ImportAnalysisPreviewApplicationPort>,
    /// Host-facing import-analysis execution boundary. Runtime supplies only
    /// prompt and cleanup adapters for the long-running analysis workflow.
    import_analysis_application:
        Arc<dyn a4n_orchestrator::import_analysis::ImportAnalysisApplicationPort>,
    /// Host-facing reverse-engineering boundary.
    import_reverse_application:
        Arc<dyn a4n_orchestrator::import_reverse::ImportReverseApplicationPort>,
    /// Reverse-engineering candidate reads stay behind the API contract;
    /// generation and confirmation remain in the owner boundary.
    import_reverse_read_application:
        Arc<dyn a4n_api_contract::import_reverse::ImportReverseReadApplicationPort>,
    /// Host-facing chapter-analysis generation boundary. Analysis
    /// authorization, workflow state and summaries stay in the chapter owner.
    chapter_analysis_generation_application:
        Arc<dyn a4n_orchestrator::chapter_analysis::ChapterAnalysisGenerationApplicationPort>,
    /// Host-facing chapter-analysis synchronization and workflow-recovery
    /// boundary.
    chapter_analysis_resume_application:
        Arc<dyn a4n_orchestrator::chapter_analysis::ChapterAnalysisResumeApplicationPort>,
    /// Host-facing chapter-analysis request-preview boundary.
    chapter_analysis_preview_application:
        Arc<dyn a4n_orchestrator::chapter_analysis::ChapterAnalysisPreviewApplicationPort>,
    /// Host-facing import-review boundary. Review authorization, persistence
    /// and API projection stay in the import-review owner.
    import_review_application:
        Arc<dyn a4n_orchestrator::import_review::ImportReviewApplicationPort>,
    /// Host-facing import-review read boundary. Review reads and deduplication
    /// stay in the API contract rather than the mixed mutation boundary.
    import_review_read_application:
        Arc<dyn a4n_api_contract::import_review::ImportReviewReadApplicationPort>,
    /// Host-facing project export boundary. Authorization and format
    /// rendering remain in the export owner; Runtime only adapts progress.
    project_export_application: Arc<dyn a4n_api_contract::export::ProjectExportApplicationPort>,
    galgame_export_application: Arc<dyn a4n_api_contract::export::GalgameExportApplicationPort>,
    /// Host-facing video-record read boundary.
    video_record_read_application: Arc<dyn a4n_api_contract::video::VideoRecordReadApplicationPort>,
    /// Host-facing ordinary video-record mutation boundary.
    video_record_mutation_application:
        Arc<dyn a4n_api_contract::video::VideoRecordMutationApplicationPort>,
    /// Host-facing imported-video and trim boundary.
    video_import_application: Arc<dyn a4n_api_contract::video::VideoImportApplicationPort>,
    /// Host-facing Assembly planning and push boundary.
    video_assembly_application: Arc<dyn a4n_api_contract::video::VideoAssemblyApplicationPort>,
    /// Host-facing video completion and cleanup lifecycle boundary.
    video_record_lifecycle_application:
        Arc<dyn a4n_api_contract::video::VideoRecordLifecycleApplicationPort>,
    /// Host-facing canvas-media preparation and persistence boundary.
    video_canvas_media_application:
        Arc<dyn a4n_api_contract::video::VideoCanvasMediaApplicationPort>,
    /// Host-facing chapter visual-plan read boundary. Visual-plan ownership
    /// and DTO projection stay outside Runtime.
    chapter_visual_plan_read_application:
        Arc<dyn a4n_api_contract::chapter_visual_plan::ChapterVisualPlanReadApplicationPort>,
    chapter_visual_plan_mutation_application:
        Arc<dyn a4n_api_contract::chapter_visual_plan::ChapterVisualPlanMutationApplicationPort>,
    /// Host-facing video-prompt read boundary. Prompt authorization and DTO
    /// projection stay in the video owner.
    video_prompt_read_application: Arc<dyn a4n_api_contract::video::VideoPromptReadApplicationPort>,
    /// Host-facing video-prompt mutation boundary. Downstream cleanup and
    /// prompt rendering are supplied by Runtime through explicit ports.
    video_prompt_mutation_application:
        Arc<dyn a4n_api_contract::video::VideoPromptMutationApplicationPort>,
    /// Host-facing video timeline boundary. Timeline revision and ordering
    /// rules stay in the video owner.
    video_prompt_timeline_application:
        Arc<dyn a4n_api_contract::video::VideoPromptTimelineApplicationPort>,
    /// Host-facing segment-plan draft boundary. Workflow/checkpoint
    /// validation stays in the video owner.
    video_prompt_draft_application:
        Arc<dyn a4n_api_contract::video::VideoPromptDraftApplicationPort>,
    /// Host-facing generated-image read boundary. Authorization and DTO
    /// projection stay in the media owner.
    image_record_read_application: Arc<dyn a4n_api_contract::image::ImageRecordReadApplicationPort>,
    /// Host-facing generated-image mutation boundary.
    image_record_mutation_application:
        Arc<dyn a4n_api_contract::image::ImageRecordMutationApplicationPort>,
    /// Imported-image file and publication transaction owner.
    image_import_application: Arc<dyn a4n_orchestrator::media::ImageImportApplicationPort>,
    /// Image publication recovery and state-transition owner.
    image_publication_recovery_application:
        Arc<dyn a4n_orchestrator::media::ImagePublicationRecoveryPort>,
    /// Image publication recovery reads stay behind the API contract; Runtime
    /// supplies file facts and cleanup side effects separately.
    image_publication_recovery_read_application:
        Arc<dyn a4n_api_contract::image::ImagePublicationRecoveryReadPort>,
    /// Persisted media-reference queries used by cleanup and orphan scans.
    image_media_reference_application: Arc<dyn a4n_api_contract::image::ImageMediaReferencePort>,
    /// Image provider config lookup stays in the settings owner.
    image_provider_config: Arc<dyn a4n_api_contract::image::ImageProviderConfigPort>,
    /// Image generation task owner. Runtime supplies only provider and file
    /// adapters when a worker is started.
    image_generation_application: Arc<dyn a4n_orchestrator::media::ImageGenerationApplicationPort>,
    /// Host-facing profile-material read boundary. Authorization and group
    /// projection stay in the media owner.
    profile_material_read_application:
        Arc<dyn a4n_api_contract::profile::ProfileMaterialReadApplicationPort>,
    profile_material_mutation_application:
        Arc<dyn a4n_api_contract::profile::ProfileMaterialMutationApplicationPort>,
    /// Upload publication uses a separate media/file adapter boundary.
    profile_material_upload_application:
        Arc<dyn a4n_orchestrator::ProfileMaterialUploadApplicationPort>,
    /// Host-facing portrait-record read boundary. Portrait authorization and
    /// DTO projection stay in the character/portrait owner.
    portrait_record_read_application:
        Arc<dyn a4n_api_contract::portrait::PortraitRecordReadApplicationPort>,
    portrait_record_mutation_application:
        Arc<dyn a4n_api_contract::portrait::PortraitRecordMutationApplicationPort>,
    /// Portrait request preview owner. Runtime supplies prompt rendering for
    /// preview construction.
    portrait_request_preview_application:
        Arc<dyn a4n_orchestrator::portrait::PortraitRequestPreviewApplicationPort>,
    /// Portrait prompt generation owner. Runtime supplies prompt rendering.
    portrait_prompt_application: Arc<dyn a4n_orchestrator::portrait::PortraitPromptApplicationPort>,
    /// Portrait upload owner. Runtime supplies image/file adapters.
    portrait_upload_application: Arc<dyn a4n_orchestrator::portrait::PortraitUploadApplicationPort>,
    /// Portrait generation task owner. Runtime supplies prompt and media
    /// adapters when a worker is started.
    portrait_generation_application:
        Arc<dyn a4n_orchestrator::portrait::PortraitGenerationApplicationPort>,
    /// Persisted video provider configuration lookup. Runtime snapshot and
    /// user-scope policy stay outside this port.
    video_provider_config: Arc<dyn a4n_api_contract::video::VideoProviderConfigPort>,
    /// Video owner boundary for provider-job recovery and status transitions.
    video_provider_execution:
        Arc<dyn a4n_api_contract::video::VideoProviderExecutionApplicationPort>,
    /// Video generation task owner. Runtime supplies only provider adapters
    /// when a worker is started.
    video_generation_application: Arc<dyn a4n_api_contract::video::VideoGenerationApplicationPort>,
    /// Video request-preview owner. Runtime supplies provider adapters while
    /// the video owner prepares and validates the request.
    video_request_preview_application:
        Arc<dyn a4n_api_contract::video::VideoRequestPreviewApplicationPort>,
    /// Host-facing Galgame pipeline execution boundary. Workflow
    /// authorization and persistence stay in the Galgame owner.
    galgame_pipeline_execution_application:
        Arc<dyn a4n_api_contract::galgame::GalgamePipelineExecutionApplicationPort>,
    /// Host-facing Galgame pipeline request-preview boundary.
    galgame_pipeline_preview_application:
        Arc<dyn a4n_api_contract::galgame::GalgamePipelinePreviewApplicationPort>,
    /// Host-facing Galgame pipeline status-read boundary.
    galgame_pipeline_read_application:
        Arc<dyn a4n_api_contract::galgame::GalgamePipelineReadApplicationPort>,
    /// Host-facing Galgame pipeline cancellation boundary.
    galgame_pipeline_cancellation_application:
        Arc<dyn a4n_api_contract::galgame::GalgamePipelineCancellationApplicationPort>,
    /// Galgame asset planning owner. Runtime receives only the stable plan.
    galgame_asset_plan_application:
        Arc<dyn a4n_api_contract::galgame::GalgameAssetPlanApplicationPort>,
    /// Galgame asset-generation owner. Runtime supplies visual/file adapters.
    galgame_asset_generation_application:
        Arc<dyn a4n_orchestrator::galgame::GalgameAssetGenerationApplicationPort>,
    /// Host-facing Galgame script-record mutation boundary. Persistence and
    /// DTO projection remain in the Galgame owner.
    galgame_record_mutation_application:
        Arc<dyn a4n_api_contract::galgame::GalgameRecordMutationApplicationPort>,
    /// Host-facing Galgame media-binding mutation boundary.
    galgame_media_mutation_application:
        Arc<dyn a4n_api_contract::galgame::GalgameMediaMutationApplicationPort>,
    /// Host-facing Manga pipeline execution boundary. Workflow persistence
    /// remains in the Manga owner; Runtime only schedules and routes results.
    manga_pipeline_execution_application:
        Arc<dyn a4n_api_contract::manga::MangaPipelineExecutionApplicationPort>,
    /// Host-facing Manga pipeline request-preview boundary.
    manga_pipeline_preview_application:
        Arc<dyn a4n_api_contract::manga::MangaPipelinePreviewApplicationPort>,
    /// Host-facing Manga pipeline status-read boundary.
    manga_pipeline_read_application:
        Arc<dyn a4n_api_contract::manga::MangaPipelineReadApplicationPort>,
    /// Host-facing Manga pipeline cancellation boundary.
    manga_pipeline_cancellation_application:
        Arc<dyn a4n_api_contract::manga::MangaPipelineCancellationApplicationPort>,
    /// Host-facing Manga prompt read boundary. Prompt projection remains in
    /// the Manga owner.
    manga_record_read_application: Arc<dyn a4n_api_contract::manga::MangaRecordReadApplicationPort>,
    /// Host-facing Manga prompt mutation boundary. Persistence and cleanup
    /// decisions remain in the Manga owner.
    manga_record_mutation_application:
        Arc<dyn a4n_api_contract::manga::MangaRecordMutationApplicationPort>,
    /// Host-facing Galgame read boundary. Script authorization and projection
    /// stay in the Galgame owner.
    galgame_read_application: Arc<dyn a4n_api_contract::galgame::GalgameReadApplicationPort>,
    /// Host-facing inspiration history read boundary. Authorization and
    /// message projection stay in the inspiration owner.
    inspiration_history_read_application:
        Arc<dyn a4n_api_contract::inspiration::InspirationHistoryReadApplicationPort>,
    /// Inspiration generation owner. Runtime only schedules the worker and
    /// supplies the prompt-rendering adapter.
    inspiration_generation_application:
        Arc<dyn a4n_orchestrator::inspiration::InspirationGenerationApplicationPort>,
    /// Inspiration request construction boundary. Runtime supplies prompt
    /// rendering and provider preview budgeting after this call returns.
    inspiration_request_application:
        Arc<dyn a4n_orchestrator::inspiration::InspirationRequestPreviewApplicationPort>,
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
        command: String,
        args: Value,
        worker_request: &WorkerRequest<AgentRunPayload>,
    ) -> Result<Value, a4n_error::AppError> {
        let run_id = worker_request.payload.run_id;
        let runtime_config_snapshot = self
            .agent_runtime_snapshot(run_id)
            .await
            .map(serde_json::to_value)
            .transpose()?;
        let payload = AgentHostCommandPayload {
            run_id,
            command: command.clone(),
            arguments: args,
            runtime_config_snapshot,
        };
        payload
            .validate()
            .map_err(|error| a4n_error::AppError::InvalidState {
                current: format!("Agent Host command payload 无效: {error}"),
                operation: "构造 Agent Host command request".into(),
            })?;
        let request_id = format!(
            "{}:host:{}",
            worker_request.request_id,
            uuid::Uuid::new_v4()
        );
        let request = WorkerRequest {
            schema_version: worker_request.schema_version,
            request_id: request_id.clone(),
            trace_id: worker_request.trace_id.clone(),
            principal: worker_request.principal.clone(),
            operation_id: AGENT_HOST_COMMAND_OPERATION.into(),
            idempotency_key: format!("{}:host:{request_id}", worker_request.idempotency_key),
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

    pub(crate) fn chapter_scene_application(
        &self,
    ) -> &dyn a4n_api_contract::chapter::ChapterSceneApplicationPort {
        self.chapter_scene_application.as_ref()
    }

    pub(crate) fn chapter_scene_generation_application(
        &self,
    ) -> &dyn a4n_orchestrator::chapter_scene::ChapterSceneGenerationApplicationPort {
        self.chapter_scene_generation_application.as_ref()
    }

    pub(crate) fn optimize_application(
        &self,
    ) -> &dyn a4n_orchestrator::optimize::OptimizeApplicationPort {
        self.optimize_application.as_ref()
    }

    pub(crate) fn scene_profile_sync_application(
        &self,
    ) -> &dyn a4n_orchestrator::scene_profile::SceneProfileSyncApplicationPort {
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

    pub(crate) fn character_ai_application(
        &self,
    ) -> &dyn a4n_orchestrator::character::CharacterAiApplicationPort {
        self.character_ai_application.as_ref()
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

    pub(crate) fn character_implicit_review_application(
        &self,
    ) -> &dyn a4n_orchestrator::character::CharacterImplicitReviewApplicationPort {
        self.character_implicit_review_application.as_ref()
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
    ) -> &dyn a4n_orchestrator::character::CharacterConflictApplicationPort {
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

    pub(crate) fn blueprint_request_preview_application(
        &self,
    ) -> &dyn a4n_orchestrator::BlueprintRequestPreviewApplicationPort {
        self.blueprint_request_preview_application.as_ref()
    }

    pub(crate) fn blueprint_generation_application(
        &self,
    ) -> &dyn a4n_orchestrator::BlueprintGenerationApplicationPort {
        self.blueprint_generation_application.as_ref()
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
    ) -> &dyn a4n_orchestrator::inspiration::InspirationGenerationApplicationPort {
        self.inspiration_generation_application.as_ref()
    }

    pub(crate) fn inspiration_request_application(
        &self,
    ) -> &dyn a4n_orchestrator::inspiration::InspirationRequestPreviewApplicationPort {
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

    pub(crate) fn video_request_preview_application(
        &self,
    ) -> &dyn a4n_api_contract::video::VideoRequestPreviewApplicationPort {
        self.video_request_preview_application.as_ref()
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
    ) -> &dyn a4n_orchestrator::chapter_generation::ChapterGenerationApplicationPort {
        self.chapter_generation_application.as_ref()
    }

    pub(crate) fn chapter_generation_preview_application(
        &self,
    ) -> &dyn a4n_orchestrator::chapter_generation::ChapterGenerationPreviewApplicationPort {
        self.chapter_generation_preview_application.as_ref()
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
    ) -> &dyn a4n_orchestrator::outline::OutlineGenerationApplicationPort {
        self.outline_generation_application.as_ref()
    }

    pub(crate) fn outline_preview_application(
        &self,
    ) -> &dyn a4n_orchestrator::OutlinePreviewApplicationPort {
        self.outline_preview_application.as_ref()
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

    pub(crate) fn settings_all_application(
        &self,
    ) -> &dyn a4n_api_contract::config::SettingsAllApplicationPort {
        self.settings_all_application.as_ref()
    }

    pub(crate) fn prompt_read_application(
        &self,
    ) -> &dyn a4n_orchestrator::prompt_records::PromptReadApplicationPort {
        self.prompt_read_application.as_ref()
    }

    pub(crate) fn prompt_mutation_application(
        &self,
    ) -> &dyn a4n_orchestrator::prompt_records::PromptMutationApplicationPort {
        self.prompt_mutation_application.as_ref()
    }

    pub(crate) fn agent_application(&self) -> &dyn a4n_api_contract::agent::AgentApplicationPort {
        self.agent_application.as_ref()
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
    ) -> &dyn a4n_orchestrator::canvas::CanvasLlmTextNodeApplicationPort {
        self.canvas_llm_text_application.as_ref()
    }

    pub(crate) fn canvas_image_application(
        &self,
    ) -> &dyn a4n_orchestrator::canvas::CanvasImageNodeApplicationPort {
        self.canvas_image_application.as_ref()
    }

    pub(crate) fn canvas_video_application(
        &self,
    ) -> &dyn a4n_orchestrator::canvas::CanvasVideoNodeApplicationPort {
        self.canvas_video_application.as_ref()
    }

    pub(crate) fn canvas_preview_application(
        &self,
    ) -> &dyn a4n_orchestrator::canvas::CanvasPreviewApplicationPort {
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

    pub(crate) fn canvas_media_import_application(
        &self,
    ) -> &dyn a4n_canvas_contract::CanvasMediaImportApplicationPort {
        self.canvas_media_import_application.as_ref()
    }

    pub(crate) fn canvas_graph_execution_application(
        &self,
    ) -> &dyn a4n_canvas_contract::CanvasGraphExecutionApplicationPort {
        self.canvas_graph_execution_application.as_ref()
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

    pub(crate) fn import_analysis_application(
        &self,
    ) -> &dyn a4n_orchestrator::import_analysis::ImportAnalysisApplicationPort {
        self.import_analysis_application.as_ref()
    }

    pub(crate) fn import_analysis_progress_application(
        &self,
    ) -> &dyn a4n_api_contract::import::ImportAnalysisProgressApplicationPort {
        self.import_analysis_progress_application.as_ref()
    }

    pub(crate) fn import_analysis_preview_application(
        &self,
    ) -> &dyn a4n_orchestrator::import_analysis::ImportAnalysisPreviewApplicationPort {
        self.import_analysis_preview_application.as_ref()
    }

    pub(crate) fn import_reverse_application(
        &self,
    ) -> &dyn a4n_orchestrator::import_reverse::ImportReverseApplicationPort {
        self.import_reverse_application.as_ref()
    }

    pub(crate) fn import_reverse_read_application(
        &self,
    ) -> &dyn a4n_api_contract::import_reverse::ImportReverseReadApplicationPort {
        self.import_reverse_read_application.as_ref()
    }

    pub(crate) fn chapter_analysis_generation_application(
        &self,
    ) -> &dyn a4n_orchestrator::chapter_analysis::ChapterAnalysisGenerationApplicationPort {
        self.chapter_analysis_generation_application.as_ref()
    }

    pub(crate) fn chapter_analysis_resume_application(
        &self,
    ) -> &dyn a4n_orchestrator::chapter_analysis::ChapterAnalysisResumeApplicationPort {
        self.chapter_analysis_resume_application.as_ref()
    }

    pub(crate) fn chapter_analysis_preview_application(
        &self,
    ) -> &dyn a4n_orchestrator::chapter_analysis::ChapterAnalysisPreviewApplicationPort {
        self.chapter_analysis_preview_application.as_ref()
    }

    pub(crate) fn import_review_application(
        &self,
    ) -> &dyn a4n_orchestrator::import_review::ImportReviewApplicationPort {
        self.import_review_application.as_ref()
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

    pub(crate) fn video_import_application(
        &self,
    ) -> &dyn a4n_api_contract::video::VideoImportApplicationPort {
        self.video_import_application.as_ref()
    }

    pub(crate) fn video_assembly_application(
        &self,
    ) -> &dyn a4n_api_contract::video::VideoAssemblyApplicationPort {
        self.video_assembly_application.as_ref()
    }

    pub(crate) fn video_record_lifecycle_application(
        &self,
    ) -> &dyn a4n_api_contract::video::VideoRecordLifecycleApplicationPort {
        self.video_record_lifecycle_application.as_ref()
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

    pub(crate) fn image_record_read_application(
        &self,
    ) -> &dyn a4n_api_contract::image::ImageRecordReadApplicationPort {
        self.image_record_read_application.as_ref()
    }

    pub(crate) fn image_record_mutation_application(
        &self,
    ) -> &dyn a4n_api_contract::image::ImageRecordMutationApplicationPort {
        self.image_record_mutation_application.as_ref()
    }

    pub(crate) fn image_import_application(
        &self,
    ) -> &dyn a4n_orchestrator::media::ImageImportApplicationPort {
        self.image_import_application.as_ref()
    }

    pub(crate) fn image_publication_recovery_application(
        &self,
    ) -> &dyn a4n_orchestrator::media::ImagePublicationRecoveryPort {
        self.image_publication_recovery_application.as_ref()
    }

    pub(crate) fn image_publication_recovery_read_application(
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

    pub(crate) fn image_generation_application(
        &self,
    ) -> &dyn a4n_orchestrator::media::ImageGenerationApplicationPort {
        self.image_generation_application.as_ref()
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
    ) -> &dyn a4n_orchestrator::ProfileMaterialUploadApplicationPort {
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

    pub(crate) fn portrait_request_preview_application(
        &self,
    ) -> &dyn a4n_orchestrator::portrait::PortraitRequestPreviewApplicationPort {
        self.portrait_request_preview_application.as_ref()
    }

    pub(crate) fn portrait_prompt_application(
        &self,
    ) -> &dyn a4n_orchestrator::portrait::PortraitPromptApplicationPort {
        self.portrait_prompt_application.as_ref()
    }

    pub(crate) fn portrait_upload_application(
        &self,
    ) -> &dyn a4n_orchestrator::portrait::PortraitUploadApplicationPort {
        self.portrait_upload_application.as_ref()
    }

    pub(crate) fn portrait_generation_application(
        &self,
    ) -> &dyn a4n_orchestrator::portrait::PortraitGenerationApplicationPort {
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
    ) -> &dyn a4n_orchestrator::galgame::GalgameAssetGenerationApplicationPort {
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
    ) -> &dyn a4n_orchestrator::GenerationTaskReadApplicationPort {
        self.generation_task_read_application.as_ref()
    }

    pub(crate) fn generation_task_mutation_application(
        &self,
    ) -> &dyn a4n_orchestrator::GenerationTaskMutationApplicationPort {
        self.generation_task_mutation_application.as_ref()
    }

    pub(crate) fn generation_lifecycle_application(
        &self,
    ) -> &dyn a4n_orchestrator::GenerationLifecycleApplicationPort {
        self.generation_lifecycle_application.as_ref()
    }

    pub(crate) fn generation_cleanup_application(
        &self,
    ) -> &dyn a4n_orchestrator::GenerationCleanupApplicationPort {
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

    pub(crate) fn shooting_script_generation_application(
        &self,
    ) -> &dyn a4n_orchestrator::ShootingScriptGenerationApplicationPort {
        self.shooting_script_generation_application.as_ref()
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

    pub async fn new_with_profile(
        config: AppConfig,
        runtime_profile: RuntimeProfile,
    ) -> Result<Self, a4n_error::AppError> {
        Self::new_with_profile_and_role(config, runtime_profile, RuntimeRole::Host).await
    }

    /// Builds the reduced Runtime instance used by the external Agent worker.
    /// It shares the database and configured capability endpoints with the
    /// Control host but does not claim Host-wide file/generation ownership or
    /// start background workers owned by Control.
    pub async fn new_agent_worker_with_profile(
        config: AppConfig,
        runtime_profile: RuntimeProfile,
    ) -> Result<Self, a4n_error::AppError> {
        Self::new_with_profile_and_role(config, runtime_profile, RuntimeRole::AgentWorker).await
    }

    async fn new_with_profile_and_role(
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
        let prompt_engine = prompt_engine_result?;
        let chapter = ChapterGenerationService::new(db.clone());
        let rag_store = rag_store_result?;
        let snapshot_store = Arc::new(Sha256SnapshotStore::new(db.clone()));
        let managed_sidecars = Arc::new(tokio::sync::Mutex::new(Vec::new()));
        let mut provider_runtime = None;
        let orchestrator = a4n_orchestrator::OrchestratorContext::bootstrap(
            a4n_db::DbHandle::new(db.clone()),
            capabilities.prompt.clone(),
            service_manager.clone(),
            config.clone(),
            event_bus.clone(),
            task_manager.clone(),
            chapter,
            rag_store,
            task_queue.clone(),
            Arc::new(prompt_engine),
            snapshot_store,
            |provider_port| {
                let provider_facade =
                    crate::provider_runtime::DynamicProviderFacade::new(provider_port);
                provider_runtime = Some(provider_facade.clone());
                let embedding = crate::sidecar_clients::create_embedding_provider(
                    provider_facade.clone(),
                    &config,
                    runtime_profile,
                    capabilities.embedding.clone(),
                )?;
                let llm = crate::sidecar_clients::create_llm_provider(
                    provider_facade,
                    runtime_profile,
                    &capabilities,
                )?;
                Ok((llm, embedding))
            },
        )?;
        let prompt_read_application: Arc<
            dyn a4n_orchestrator::prompt_records::PromptReadApplicationPort,
        > = Arc::new(orchestrator.clone());
        let prompt_mutation_application: Arc<
            dyn a4n_orchestrator::prompt_records::PromptMutationApplicationPort,
        > = Arc::new(orchestrator.clone());
        prompt_mutation_application.hydrate_overrides().await?;
        let provider_runtime =
            provider_runtime.ok_or_else(|| a4n_error::AppError::InvalidState {
                current: "provider runtime port 未初始化".to_string(),
                operation: "构造 Runtime provider".to_string(),
            })?;
        let task_queue_application = Arc::new(crate::task_queue::TaskQueueApplication::new(
            task_queue.clone(),
        ));
        let rag_search: Arc<dyn a4n_rag_contract::RagSearchPort> = Arc::new(orchestrator.clone());
        let rag_ingest: Arc<dyn a4n_rag_contract::RagIngestPort> = Arc::new(orchestrator.clone());
        let config_read_application: Arc<dyn a4n_api_contract::config::ConfigReadApplicationPort> =
            Arc::new(orchestrator.clone());
        let config_mutation_application: Arc<
            dyn a4n_api_contract::config::ConfigMutationApplicationPort,
        > = Arc::new(orchestrator.clone());
        let package_read_application: Arc<
            dyn a4n_api_contract::package::PackageReadApplicationPort,
        > = Arc::new(orchestrator.clone());
        let package_mutation_application: Arc<
            dyn a4n_api_contract::package::PackageMutationApplicationPort,
        > = Arc::new(orchestrator.clone());
        let settings_all_application: Arc<
            dyn a4n_api_contract::config::SettingsAllApplicationPort,
        > = Arc::new(orchestrator.clone());
        if runtime_role == RuntimeRole::Host {
            package_mutation_application.initialize_states().await?;
            crate::baseline_configs::ensure_baseline_model_configs(
                config_read_application.as_ref(),
                config_mutation_application.as_ref(),
                &config,
            )
            .await?;
        }

        let state = Self {
            config: config.clone(),
            task_manager: task_manager.clone(),
            generation_task_read_application: Arc::new(orchestrator.clone()),
            generation_task_mutation_application: Arc::new(orchestrator.clone()),
            generation_lifecycle_application: Arc::new(orchestrator.clone()),
            generation_cleanup_application: Arc::new(orchestrator.clone()),
            source_sync_read_application: Arc::new(orchestrator.clone()),
            source_sync_mutation_application: Arc::new(orchestrator.clone()),
            shooting_script_record_read_application: Arc::new(orchestrator.clone()),
            shooting_script_record_mutation_application: Arc::new(orchestrator.clone()),
            shooting_script_generation_application: Arc::new(orchestrator.clone()),
            auth_read_application: Arc::new(orchestrator.clone()),
            auth_mutation_application: Arc::new(orchestrator.clone()),
            admin_read_application: Arc::new(orchestrator.clone()),
            admin_mutation_application: Arc::new(orchestrator.clone()),
            project_status_application: Arc::new(orchestrator.clone()),
            project_collaboration_application: Arc::new(orchestrator.clone()),
            project_access_application: Arc::new(orchestrator.clone()),
            project_read_application: Arc::new(orchestrator.clone()),
            project_mutation_application: Arc::new(orchestrator.clone()),
            project_asset_graph_application: Arc::new(orchestrator.clone()),
            project_duplication_application: Arc::new(orchestrator.clone()),
            project_family_transfer_application: Arc::new(orchestrator.clone()),
            video_studio_transfer_application: Arc::new(orchestrator.clone()),
            chapter_scene_application: Arc::new(orchestrator.clone()),
            chapter_scene_generation_application: Arc::new(orchestrator.clone()),
            optimize_application: Arc::new(orchestrator.clone()),
            scene_profile_sync_application: Arc::new(orchestrator.clone()),
            service_health_application: Arc::new(crate::service_health::ServiceHealthApplication::new(
                service_manager.clone(),
            )),
            cleanup_outbox_application: Arc::new(orchestrator.clone()),
            character_ai_application: Arc::new(orchestrator.clone()),
            workflow_read_application: Arc::new(orchestrator.clone()),
            workflow_task_lookup_application: Arc::new(orchestrator.clone()),
            task_read_application: Arc::new(orchestrator.clone()),
            task_cancellation_application: task_queue_application.clone(),
            task_queue_stats_application: task_queue_application,
            workflow_cancellation_application: Arc::new(orchestrator.clone()),
            object_profile_read_application: Arc::new(orchestrator.clone()),
            object_profile_mutation_application: Arc::new(orchestrator.clone()),
            scene_profile_read_application: Arc::new(orchestrator.clone()),
            character_profile_read_application: Arc::new(orchestrator.clone()),
            character_profile_mutation_application: Arc::new(orchestrator.clone()),
            character_profile_timeline_mutation_application: Arc::new(orchestrator.clone()),
            character_profile_attribute_mutation_application: Arc::new(orchestrator.clone()),
            character_profile_attribute_deletion_application: Arc::new(orchestrator.clone()),
            character_implicit_review_application: Arc::new(orchestrator.clone()),
            character_implicit_review_read_application: Arc::new(orchestrator.clone()),
            character_profile_deletion_application: Arc::new(orchestrator.clone()),
            character_conflict_application: Arc::new(orchestrator.clone()),
            scene_profile_mutation_application: Arc::new(orchestrator.clone()),
            foreshadowing_read_application: Arc::new(orchestrator.clone()),
            foreshadowing_mutation_application: Arc::new(orchestrator.clone()),
            blueprint_read_application: Arc::new(orchestrator.clone()),
            blueprint_request_preview_application: Arc::new(orchestrator.clone()),
            blueprint_generation_application: Arc::new(orchestrator.clone()),
            blueprint_mutation_application: Arc::new(orchestrator.clone()),
            chapter_content_read_application: Arc::new(orchestrator.clone()),
            chapter_content_mutation_application: Arc::new(orchestrator.clone()),
            chapter_content_sync_application: Arc::new(orchestrator.clone()),
            chapter_generation_preview_application: Arc::new(orchestrator.clone()),
            chapter_generation_application: Arc::new(orchestrator.clone()),
            outline_read_application: Arc::new(orchestrator.clone()),
            outline_mutation_application: Arc::new(orchestrator.clone()),
            outline_generation_application: Arc::new(orchestrator.clone()),
            outline_preview_application: Arc::new(orchestrator.clone()),
            theme_read_application: Arc::new(orchestrator.clone()),
            theme_mutation_application: Arc::new(orchestrator.clone()),
            config_read_application,
            config_mutation_application,
            settings_all_application,
            prompt_read_application,
            prompt_mutation_application,
            agent_application: Arc::new(orchestrator.agent().clone()),
            agent_worker_application: Arc::new(orchestrator.agent().clone()),
            canvas_read_application: Arc::new(orchestrator.clone()),
            canvas_mutation_application: Arc::new(orchestrator.clone()),
            canvas_render_text_application: Arc::new(orchestrator.clone()),
            canvas_llm_text_application: Arc::new(orchestrator.clone()),
            canvas_image_application: Arc::new(orchestrator.clone()),
            canvas_video_application: Arc::new(orchestrator.clone()),
            canvas_preview_application: Arc::new(orchestrator.clone()),
            canvas_editing_application: Arc::new(orchestrator.clone()),
            canvas_probe_application: Arc::new(orchestrator.clone()),
            canvas_output_application: Arc::new(orchestrator.clone()),
            canvas_media_import_application: Arc::new(orchestrator.clone()),
            canvas_graph_execution_application: Arc::new(orchestrator.clone()),
            package_read_application,
            package_mutation_application,
            import_analysis_progress_application: Arc::new(orchestrator.clone()),
            import_analysis_preview_application: Arc::new(orchestrator.clone()),
            import_analysis_application: Arc::new(orchestrator.clone()),
            import_reverse_application: Arc::new(orchestrator.clone()),
            import_reverse_read_application: Arc::new(orchestrator.clone()),
            chapter_analysis_generation_application: Arc::new(orchestrator.clone()),
            chapter_analysis_resume_application: Arc::new(orchestrator.clone()),
            chapter_analysis_preview_application: Arc::new(orchestrator.clone()),
            import_review_application: Arc::new(orchestrator.clone()),
            import_review_read_application: Arc::new(orchestrator.clone()),
            project_export_application: Arc::new(orchestrator.clone()),
            galgame_export_application: Arc::new(orchestrator.clone()),
            video_record_read_application: Arc::new(orchestrator.clone()),
            video_record_mutation_application: Arc::new(orchestrator.clone()),
            video_import_application: Arc::new(orchestrator.clone()),
            video_assembly_application: Arc::new(orchestrator.clone()),
            video_record_lifecycle_application: Arc::new(orchestrator.clone()),
            video_canvas_media_application: Arc::new(orchestrator.clone()),
            chapter_visual_plan_read_application: Arc::new(orchestrator.clone()),
            chapter_visual_plan_mutation_application: Arc::new(orchestrator.clone()),
            video_prompt_read_application: Arc::new(orchestrator.clone()),
            video_prompt_mutation_application: Arc::new(orchestrator.clone()),
            video_prompt_timeline_application: Arc::new(orchestrator.clone()),
            video_prompt_draft_application: Arc::new(orchestrator.clone()),
            image_record_read_application: Arc::new(orchestrator.clone()),
            image_record_mutation_application: Arc::new(orchestrator.clone()),
            image_import_application: Arc::new(orchestrator.clone()),
            image_publication_recovery_application: Arc::new(orchestrator.clone()),
            image_publication_recovery_read_application: Arc::new(orchestrator.clone()),
            image_media_reference_application: Arc::new(orchestrator.clone()),
            image_provider_config: Arc::new(orchestrator.clone()),
            image_generation_application: Arc::new(orchestrator.clone()),
            profile_material_read_application: Arc::new(orchestrator.clone()),
            profile_material_mutation_application: Arc::new(orchestrator.clone()),
            profile_material_upload_application: Arc::new(orchestrator.clone()),
            portrait_record_read_application: Arc::new(orchestrator.clone()),
            portrait_record_mutation_application: Arc::new(orchestrator.clone()),
            portrait_request_preview_application: Arc::new(orchestrator.clone()),
            portrait_prompt_application: Arc::new(orchestrator.clone()),
            portrait_upload_application: Arc::new(orchestrator.clone()),
            portrait_generation_application: Arc::new(orchestrator.clone()),
            video_provider_config: Arc::new(orchestrator.clone()),
            video_provider_execution: Arc::new(orchestrator.clone()),
            video_generation_application: Arc::new(orchestrator.clone()),
            video_request_preview_application: Arc::new(orchestrator.clone()),
            galgame_pipeline_execution_application: Arc::new(orchestrator.clone()),
            galgame_pipeline_preview_application: Arc::new(orchestrator.clone()),
            galgame_pipeline_read_application: Arc::new(orchestrator.clone()),
            galgame_pipeline_cancellation_application: Arc::new(orchestrator.clone()),
            galgame_asset_plan_application: Arc::new(orchestrator.clone()),
            galgame_asset_generation_application: Arc::new(orchestrator.clone()),
            galgame_record_mutation_application: Arc::new(orchestrator.clone()),
            galgame_media_mutation_application: Arc::new(orchestrator.clone()),
            manga_pipeline_execution_application: Arc::new(orchestrator.clone()),
            manga_pipeline_preview_application: Arc::new(orchestrator.clone()),
            manga_pipeline_read_application: Arc::new(orchestrator.clone()),
            manga_pipeline_cancellation_application: Arc::new(orchestrator.clone()),
            manga_record_read_application: Arc::new(orchestrator.clone()),
            manga_record_mutation_application: Arc::new(orchestrator.clone()),
            galgame_read_application: Arc::new(orchestrator.clone()),
            inspiration_history_read_application: Arc::new(orchestrator.clone()),
            inspiration_generation_application: Arc::new(orchestrator.clone()),
            inspiration_request_application: Arc::new(orchestrator.clone()),
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
