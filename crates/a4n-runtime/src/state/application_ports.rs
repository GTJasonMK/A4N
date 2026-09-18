use std::sync::Arc;

#[derive(Clone)]
/// Application behavior injected by a Host assembly layer.
pub struct ApplicationPorts {
    pub generation_task_read_application:
        Arc<dyn a4n_generation_host_contract::GenerationTaskReadApplicationPort>,
    pub generation_task_mutation_application:
        Arc<dyn a4n_generation_host_contract::GenerationTaskMutationApplicationPort>,
    pub generation_lifecycle_application:
        Arc<dyn a4n_generation_host_contract::GenerationLifecycleApplicationPort>,
    pub generation_cleanup_application:
        Arc<dyn a4n_generation_host_contract::GenerationCleanupApplicationPort>,
    /// Source-sync preview authorization and projection stay in the
    /// source-sync owner.
    pub source_sync_read_application:
        Arc<dyn a4n_api_contract::source_sync::SourceSyncReadApplicationPort>,
    /// Source-sync decisions and detach state transitions stay in the
    /// source-sync owner.
    pub source_sync_mutation_application:
        Arc<dyn a4n_api_contract::source_sync::SourceSyncMutationApplicationPort>,
    pub shooting_script_current_application:
        Arc<dyn a4n_api_contract::shooting_script::ShootingScriptCurrentReadApplicationPort>,
    /// Workflow checkpoint progress remains on the generation owner until its
    /// worker contract carries the reconstruction state explicitly.
    pub shooting_script_unit_progress_application:
        Arc<dyn a4n_api_contract::shooting_script::ShootingScriptUnitProgressReadApplicationPort>,
    pub shooting_script_record_mutation_application:
        Arc<dyn a4n_api_contract::shooting_script::ShootingScriptRecordMutationApplicationPort>,
    pub shooting_script_full_generation_application:
        Arc<dyn a4n_novel_host_contract::ShootingScriptFullGenerationApplicationPort>,
    pub shooting_script_next_unit_generation_application:
        Arc<dyn a4n_novel_host_contract::ShootingScriptNextUnitGenerationApplicationPort>,
    /// Host-facing authentication read boundary. Passwords, sessions and ORM
    /// models remain owned by the authentication application.
    pub auth_read_application: Arc<dyn a4n_api_contract::auth::AuthReadApplicationPort>,
    pub auth_mutation_application: Arc<dyn a4n_api_contract::auth::AuthMutationApplicationPort>,
    /// Host-facing administrative read and authorization boundary. Admin
    /// projections stay in the orchestrator owner.
    pub admin_read_application: Arc<dyn a4n_api_contract::admin::AdminReadApplicationPort>,
    pub model_usage_read_application:
        Arc<dyn a4n_api_contract::cost::ModelUsageReadApplicationPort>,
    pub admin_mutation_application: Arc<dyn a4n_api_contract::admin::AdminMutationApplicationPort>,
    /// Host-facing project status boundary. Status transitions and family
    /// projections stay in the project owner and return API DTOs only.
    pub project_status_application:
        Arc<dyn a4n_api_contract::project::ProjectStatusApplicationPort>,
    /// Host-facing project collaboration boundary. Membership and invitation
    /// state transitions stay in the project collaboration owner.
    pub project_collaboration_application:
        Arc<dyn a4n_api_contract::collaboration::ProjectCollaborationApplicationPort>,
    /// Host-facing project access boundary. Permission checks and root
    /// identity stay with the project owner.
    pub project_access_application:
        Arc<dyn a4n_api_contract::collaboration::ProjectAccessApplicationPort>,
    /// Host-facing project read boundary. Runtime receives complete API
    /// summaries and does not assemble them from ORM projections.
    pub project_read_application: Arc<dyn a4n_api_contract::project::ProjectReadApplicationPort>,
    pub project_source_snapshot_read_application:
        Arc<dyn a4n_api_contract::project::ProjectSourceSnapshotReadApplicationPort>,
    /// Host-facing project mutation boundary. Project validation and summary
    /// projection remain in the project owner.
    pub project_mutation_application:
        Arc<dyn a4n_api_contract::project::ProjectMutationApplicationPort>,
    /// Host-facing project asset graph boundary.
    pub project_asset_graph_application:
        Arc<dyn a4n_api_contract::project::ProjectAssetGraphApplicationPort>,
    /// Project duplication owner. Runtime supplies only media copy/rollback
    /// adapters and receives the final API summary.
    pub project_duplication_application:
        Arc<dyn a4n_api_contract::project::ProjectDuplicationApplicationPort>,
    /// Project-family package transfer owner. Runtime supplies archive/file
    /// adapters and receives summaries rather than ORM records.
    pub project_family_transfer_application:
        Arc<dyn a4n_api_contract::project_transfer::ProjectFamilyTransferApplicationPort>,
    pub project_bundle_transfer_application:
        Arc<dyn a4n_api_contract::project_transfer::ProjectBundleTransferApplicationPort>,
    /// Video-studio package transfer owner. Runtime supplies archive/file
    /// adapters and receives summaries rather than ORM records.
    pub video_studio_transfer_application:
        Arc<dyn a4n_api_contract::project_transfer::VideoStudioProjectTransferApplicationPort>,
    /// Host-facing chapter scene boundary.
    pub chapter_scene_read_application:
        Arc<dyn a4n_api_contract::chapter::ChapterSceneReadApplicationPort>,
    pub chapter_scene_mutation_application:
        Arc<dyn a4n_api_contract::chapter::ChapterSceneMutationApplicationPort>,
    /// Chapter-scene generation and request construction remain in the scene
    /// owner; Runtime only supplies prompt and progress adapters.
    pub chapter_scene_generation_application:
        Arc<dyn a4n_novel_host_contract::ChapterSceneGenerationApplicationPort>,
    /// Content-optimization session reads stay behind the API contract.
    pub optimize_session_read_application:
        Arc<dyn a4n_api_contract::optimize::OptimizeSessionReadApplicationPort>,
    pub optimize_agent_tool_application:
        Arc<dyn a4n_optimize_host_contract::OptimizeAgentToolApplicationPort>,
    /// Content-optimization session mutations stay in the optimize owner.
    pub optimize_session_mutation_application:
        Arc<dyn a4n_optimize_host_contract::OptimizeSessionMutationApplicationPort>,
    /// Content-optimization patch application and chapter writeback stay in
    /// the optimize owner.
    pub optimize_patch_application:
        Arc<dyn a4n_optimize_host_contract::OptimizePatchApplicationPort>,
    /// Content-optimization patch request previews stay behind the patch
    /// boundary.
    pub optimize_patch_preview_application:
        Arc<dyn a4n_optimize_host_contract::OptimizePatchPreviewApplicationPort>,
    /// Content-optimization analysis request previews stay behind the
    /// analysis-specific boundary.
    pub optimize_analysis_preview_application:
        Arc<dyn a4n_optimize_host_contract::OptimizeAnalysisPreviewApplicationPort>,
    /// Content-optimization chat request previews stay behind the chat
    /// boundary.
    pub optimize_chat_preview_application:
        Arc<dyn a4n_optimize_host_contract::OptimizeChatPreviewApplicationPort>,
    /// Each content-optimization worker has its own runtime boundary so a
    /// worker-specific change does not widen the injected contract.
    pub optimize_chat_generation_application:
        Arc<dyn a4n_optimize_host_contract::OptimizeChatGenerationApplicationPort>,
    pub optimize_analysis_generation_application:
        Arc<dyn a4n_optimize_host_contract::OptimizeAnalysisGenerationApplicationPort>,
    pub optimize_patch_generation_application:
        Arc<dyn a4n_optimize_host_contract::OptimizePatchGenerationApplicationPort>,
    /// Scene-profile synchronization remains owned by the scene-profile
    /// application; Runtime supplies only the RAG host adapter.
    pub scene_profile_sync_application:
        Arc<dyn a4n_scene_host_contract::SceneProfileSyncApplicationPort>,
    /// Cleanup claiming, leasing and audit transitions stay with the cleanup owner.
    pub cleanup_outbox_application: Arc<dyn a4n_cleanup_contract::CleanupOutboxApplicationPort>,
    /// Character discovery and profile synchronization use separate worker
    /// boundaries; Runtime only adapts progress and RAG tracing.
    pub character_discovery_application:
        Arc<dyn a4n_character_host_contract::CharacterDiscoveryApplicationPort>,
    pub character_profile_sync_application:
        Arc<dyn a4n_character_host_contract::CharacterProfileSyncApplicationPort>,
    /// Host-facing workflow read boundary. Runtime keeps only live handles
    /// and cancellation coordination outside this port.
    pub workflow_read_application: Arc<dyn a4n_api_contract::workflow::WorkflowReadApplicationPort>,
    /// Runtime task/workflow lookup boundary. ORM records remain owned by the
    /// workflow application and are projected to narrow records here.
    pub workflow_task_lookup_application:
        Arc<dyn a4n_api_contract::workflow::WorkflowTaskLookupApplicationPort>,
    /// Host-facing task history read boundary. Task access checks remain in
    /// Runtime; persistence and summary projection stay with the task owner.
    pub task_read_application: Arc<dyn a4n_api_contract::task::TaskReadApplicationPort>,
    /// Workflow cancellation remains owned by Orchestrator; workers receive
    /// only the narrow side-effecting operation.
    pub workflow_cancellation_application:
        Arc<dyn a4n_api_contract::workflow::WorkflowCancellationPort>,
    /// Host-facing object profile read boundary.
    pub object_profile_read_application:
        Arc<dyn a4n_api_contract::profile::ObjectProfileReadApplicationPort>,
    /// Host-facing scene profile read boundary.
    pub scene_profile_read_application:
        Arc<dyn a4n_api_contract::profile::SceneProfileReadApplicationPort>,
    /// Host-facing character profile read boundary.
    pub character_profile_read_application:
        Arc<dyn a4n_api_contract::character::CharacterProfileReadApplicationPort>,
    /// Host-facing character profile mutation boundary. Runtime keeps only
    /// post-commit RAG warning handling outside the character owner.
    pub character_profile_mutation_application:
        Arc<dyn a4n_api_contract::character::CharacterProfileMutationApplicationPort>,
    /// Host-facing character timeline mutation boundary. Runtime only scopes
    /// the RAG trace and forwards rollback commands.
    pub character_profile_timeline_mutation_application:
        Arc<dyn a4n_api_contract::character::CharacterProfileTimelineMutationApplicationPort>,
    /// Host-facing explicit character attribute mutation boundary.
    pub character_profile_attribute_mutation_application:
        Arc<dyn a4n_api_contract::character::CharacterProfileAttributeMutationApplicationPort>,
    /// Host-facing attribute deletion-mark mutation boundary.
    pub character_profile_attribute_deletion_application:
        Arc<dyn a4n_api_contract::character::CharacterProfileAttributeDeletionApplicationPort>,
    /// Character implicit-review checks, previews, and applies use separate
    /// worker boundaries; Runtime only supplies prompt rendering.
    pub character_implicit_review_check_application:
        Arc<dyn a4n_character_host_contract::CharacterImplicitReviewCheckApplicationPort>,
    pub character_implicit_review_preview_application:
        Arc<dyn a4n_character_host_contract::CharacterImplicitReviewPreviewApplicationPort>,
    pub character_implicit_review_apply_application:
        Arc<dyn a4n_character_host_contract::CharacterImplicitReviewApplyApplicationPort>,
    /// Host-facing implicit-review read boundary. Summary queries stay in the
    /// API contract rather than the LLM decision workflow boundary.
    pub character_implicit_review_read_application:
        Arc<dyn a4n_api_contract::character::CharacterImplicitReviewReadApplicationPort>,
    /// Host-facing character profile deletion boundary.
    pub character_profile_deletion_application:
        Arc<dyn a4n_api_contract::character::CharacterProfileDeletionApplicationPort>,
    /// Character-consistency workflow owner. Runtime supplies only prompt,
    /// progress, and task lifecycle adapters.
    pub character_conflict_application:
        Arc<dyn a4n_character_host_contract::CharacterConflictApplicationPort>,
    /// Host-facing scene profile mutation boundary.
    pub scene_profile_mutation_application:
        Arc<dyn a4n_api_contract::profile::SceneProfileMutationApplicationPort>,
    /// Host-facing foreshadowing read boundary. Authorization and DTO
    /// projection stay in the owner.
    pub foreshadowing_read_application:
        Arc<dyn a4n_api_contract::profile::ForeshadowingReadApplicationPort>,
    pub foreshadowing_mutation_application:
        Arc<dyn a4n_api_contract::profile::ForeshadowingMutationApplicationPort>,
    /// Host-facing blueprint read boundary.
    pub blueprint_read_application:
        Arc<dyn a4n_api_contract::blueprint::BlueprintReadApplicationPort>,
    /// Blueprint generation request preview owner.
    pub blueprint_generation_preview_application:
        Arc<dyn a4n_blueprint_contract::BlueprintGenerationPreviewApplicationPort>,
    /// Blueprint refinement request preview owner.
    pub blueprint_refinement_preview_application:
        Arc<dyn a4n_blueprint_contract::BlueprintRefinementPreviewApplicationPort>,
    /// Blueprint generation owner. Runtime supplies task workers and prompt
    /// rendering; workflow persistence stays with the blueprint owner.
    pub blueprint_generation_application:
        Arc<dyn a4n_blueprint_contract::BlueprintGenerationApplicationPort>,
    /// Blueprint refinement worker boundary.
    pub blueprint_refinement_application:
        Arc<dyn a4n_blueprint_contract::BlueprintRefinementApplicationPort>,
    /// Host-facing blueprint mutation boundary. Durable version writes remain
    /// owned by the blueprint application.
    pub blueprint_mutation_application:
        Arc<dyn a4n_api_contract::blueprint::BlueprintMutationApplicationPort>,
    /// Host-facing chapter content read boundary.
    pub chapter_content_read_application:
        Arc<dyn a4n_api_contract::chapter::ChapterContentReadApplicationPort>,
    /// Host-facing chapter content mutation boundary. Workflow side effects
    /// are supplied through the explicit cleanup host port at call time.
    pub chapter_content_mutation_application:
        Arc<dyn a4n_api_contract::chapter::ChapterContentMutationApplicationPort>,
    /// Host-facing chapter-content derived-data synchronization and recovery
    /// boundary.
    pub chapter_content_sync_application:
        Arc<dyn a4n_api_contract::chapter::ChapterContentSyncApplicationPort>,
    /// Chapter request preview owner. Runtime supplies prompt and embedding
    /// inputs.
    pub chapter_generation_request_preview_application:
        Arc<dyn a4n_chapter_generation_contract::ChapterGenerationRequestPreviewApplicationPort>,
    /// Chapter-generation RAG preparation owner.
    pub chapter_generation_rag_preparation_application:
        Arc<dyn a4n_chapter_generation_contract::ChapterGenerationRagPreparationApplicationPort>,
    /// Chapter generation task owner. Runtime supplies prompt, embedding and
    /// cleanup adapters when a worker is started.
    pub chapter_generation_application:
        Arc<dyn a4n_chapter_generation_contract::ChapterGenerationApplicationPort>,
    /// Host-facing outline read boundary.
    pub outline_read_application: Arc<dyn a4n_api_contract::outline::OutlineReadApplicationPort>,
    /// Host-facing outline mutation boundary.
    pub outline_mutation_application:
        Arc<dyn a4n_api_contract::outline::OutlineMutationApplicationPort>,
    /// Outline generation task owner. Runtime only schedules the worker.
    pub outline_generation_application:
        Arc<dyn a4n_outline_contract::OutlineGenerationApplicationPort>,
    /// Part outline generation preview owner.
    pub outline_part_generation_preview_application:
        Arc<dyn a4n_outline_contract::OutlinePartGenerationPreviewApplicationPort>,
    /// Part outline mutation preview owner.
    pub outline_part_mutation_preview_application:
        Arc<dyn a4n_outline_contract::OutlinePartMutationPreviewApplicationPort>,
    /// Part chapter generation preview owner.
    pub outline_part_chapter_generation_preview_application:
        Arc<dyn a4n_outline_contract::OutlinePartChapterGenerationPreviewApplicationPort>,
    /// Batch part/chapter outline generation preview owner.
    pub outline_batch_part_generation_preview_application:
        Arc<dyn a4n_outline_contract::OutlineBatchPartGenerationPreviewApplicationPort>,
    /// Chapter-count generation preview owner.
    pub outline_chapter_count_generation_preview_application:
        Arc<dyn a4n_outline_contract::OutlineChapterCountGenerationPreviewApplicationPort>,
    /// Chapter outline regeneration preview owner.
    pub outline_chapter_regeneration_preview_application:
        Arc<dyn a4n_outline_contract::OutlineChapterRegenerationPreviewApplicationPort>,
    /// Outline beat-mapping preview owner.
    pub outline_beat_mapping_preview_application:
        Arc<dyn a4n_outline_contract::OutlineBeatMappingPreviewApplicationPort>,
    /// Host-facing theme/settings read boundary. Theme ownership and metadata
    /// stay in the settings application; Runtime handles only protected files.
    pub theme_read_application: Arc<dyn a4n_api_contract::theme::ThemeReadApplicationPort>,
    pub theme_mutation_application: Arc<dyn a4n_api_contract::theme::ThemeMutationApplicationPort>,
    /// Host-facing AI configuration read boundary. Validation, authorization
    /// and DTO projection stay in the configuration owner.
    pub config_read_application: Arc<dyn a4n_api_contract::config::ConfigReadApplicationPort>,
    pub config_mutation_application:
        Arc<dyn a4n_api_contract::config::ConfigMutationApplicationPort>,
    /// Host-facing settings export boundary. Authorization and projection
    /// stay in the settings owner.
    pub settings_export_application:
        Arc<dyn a4n_api_contract::config::SettingsExportApplicationPort>,
    /// Host-facing settings import boundary. Authorization and rollback
    /// semantics stay in the settings owner.
    pub settings_import_application:
        Arc<dyn a4n_api_contract::config::SettingsImportApplicationPort>,
    /// Host-facing prompt record read boundary. Prompt service selection stays
    /// with the prompt owner.
    pub prompt_read_application: Arc<dyn a4n_api_contract::prompt::PromptReadApplicationPort>,
    pub prompt_mutation_application:
        Arc<dyn a4n_api_contract::prompt::PromptMutationApplicationPort>,
    /// Host-facing Agent operations are injected through focused contracts;
    /// worker-only lease operations stay on the worker aggregate below.
    pub agent_thread_mutation_application:
        Arc<dyn a4n_api_contract::agent::AgentThreadMutationApplicationPort>,
    pub agent_thread_read_application:
        Arc<dyn a4n_api_contract::agent::AgentThreadReadApplicationPort>,
    pub agent_run_mutation_application:
        Arc<dyn a4n_api_contract::agent::AgentRunMutationApplicationPort>,
    pub agent_run_read_application: Arc<dyn a4n_api_contract::agent::AgentRunReadApplicationPort>,
    pub agent_delegation_application:
        Arc<dyn a4n_api_contract::agent::AgentDelegationApplicationPort>,
    pub agent_tool_read_application: Arc<dyn a4n_api_contract::agent::AgentToolReadApplicationPort>,
    pub agent_skill_application: Arc<dyn a4n_api_contract::agent::AgentSkillApplicationPort>,
    pub agent_event_read_application:
        Arc<dyn a4n_api_contract::agent::AgentEventReadApplicationPort>,
    pub agent_memory_read_application:
        Arc<dyn a4n_api_contract::agent::AgentMemoryReadApplicationPort>,
    pub agent_memory_mutation_application:
        Arc<dyn a4n_api_contract::agent::AgentMemoryMutationApplicationPort>,
    pub agent_approval_application: Arc<dyn a4n_api_contract::agent::AgentApprovalApplicationPort>,
    pub agent_resource_access_application:
        Arc<dyn a4n_api_contract::agent::AgentResourceAccessApplicationPort>,
    /// Agent worker ports used only by the durable Runtime worker. Keeping the
    /// aggregate behind its contract avoids coupling Runtime to the owner.
    pub agent_worker_application: Arc<dyn a4n_agent_runtime_contract::AgentWorkerApplicationPort>,
    /// Host-facing Canvas read port. Graph permissions, versioning and
    /// persistence stay with the Canvas owner; Runtime only routes actions.
    pub canvas_read_application: Arc<dyn a4n_api_contract::canvas::CanvasReadApplicationPort>,
    pub canvas_mutation_application:
        Arc<dyn a4n_api_contract::canvas::CanvasMutationApplicationPort>,
    /// Canvas RenderText owner. Runtime supplies only node identity and the
    /// shared workflow snapshot.
    pub canvas_render_text_application:
        Arc<dyn a4n_canvas_contract::CanvasRenderTextNodeApplicationPort>,
    /// Canvas LLM text owner. Runtime supplies the LLM HostPort and only node
    /// identity plus the shared workflow snapshot.
    pub canvas_llm_text_application:
        Arc<dyn a4n_canvas_host_contract::CanvasLlmTextNodeApplicationPort>,
    /// Canvas image-generation owner. Runtime supplies only the image HostPort
    /// and a shared workflow snapshot.
    pub canvas_image_application: Arc<dyn a4n_canvas_host_contract::CanvasImageNodeApplicationPort>,
    /// Canvas video-generation owner. Runtime supplies only the video HostPort
    /// and a shared workflow snapshot.
    pub canvas_video_application: Arc<dyn a4n_canvas_host_contract::CanvasVideoNodeApplicationPort>,
    /// Canvas request-preview owner. Runtime receives prepared pure requests
    /// and renders provider-specific preview steps.
    pub canvas_preview_application: Arc<dyn a4n_canvas_host_contract::CanvasPreviewApplicationPort>,
    /// Canvas editing node owner. Runtime supplies only media editing HostPort
    /// and a shared workflow snapshot.
    pub canvas_editing_application: Arc<dyn a4n_canvas_contract::CanvasEditingNodeApplicationPort>,
    /// Canvas probe node owner. Runtime supplies only node identity and the
    /// shared workflow snapshot.
    pub canvas_probe_application: Arc<dyn a4n_canvas_contract::CanvasProbeNodeApplicationPort>,
    /// Canvas output node owner. Runtime supplies only node identity and the
    /// shared workflow snapshot.
    pub canvas_output_application: Arc<dyn a4n_canvas_contract::CanvasOutputNodeApplicationPort>,
    /// Canvas media import workers own one media kind each. Runtime supplies
    /// upload/file adapters and receives API summaries only.
    pub canvas_image_import_application:
        Arc<dyn a4n_canvas_contract::CanvasImageImportApplicationPort>,
    pub canvas_video_import_application:
        Arc<dyn a4n_canvas_contract::CanvasVideoImportApplicationPort>,
    pub canvas_audio_import_application:
        Arc<dyn a4n_canvas_contract::CanvasAudioImportApplicationPort>,
    /// Canvas graph preparation, execution and failure compensation use
    /// separate worker boundaries.
    pub canvas_graph_preparation_application:
        Arc<dyn a4n_canvas_contract::CanvasGraphPreparationApplicationPort>,
    pub canvas_graph_execution_application:
        Arc<dyn a4n_canvas_contract::CanvasGraphExecutionApplicationPort>,
    pub canvas_graph_failure_application:
        Arc<dyn a4n_canvas_contract::CanvasGraphFailureApplicationPort>,
    /// Host-facing package state read port. Lifecycle workers keep package
    /// ownership behind the narrow read/mutation contracts.
    pub package_read_application: Arc<dyn a4n_api_contract::package::PackageReadApplicationPort>,
    pub package_mutation_application:
        Arc<dyn a4n_api_contract::package::PackageMutationApplicationPort>,
    /// Host-facing import-analysis progress boundary.
    pub import_analysis_progress_application:
        Arc<dyn a4n_api_contract::import::ImportAnalysisProgressApplicationPort>,
    /// Host-facing import-analysis request-preview boundary.
    pub import_analysis_preview_application:
        Arc<dyn a4n_import_analysis_host_contract::ImportAnalysisPreviewApplicationPort>,
    /// Host-facing import-analysis execution boundary. Runtime supplies only
    /// prompt and cleanup adapters for the long-running analysis workflow.
    pub import_analysis_generation_application:
        Arc<dyn a4n_import_analysis_host_contract::ImportAnalysisGenerationApplicationPort>,
    /// Host-facing import-analysis cancellation boundary.
    pub import_analysis_cancellation_application:
        Arc<dyn a4n_api_contract::import::ImportAnalysisCancellationApplicationPort>,
    /// Host-facing reverse-engineering request-preview boundary.
    pub import_reverse_preview_application:
        Arc<dyn a4n_novel_host_contract::ImportReversePreviewApplicationPort>,
    /// Host-facing reverse-engineering execution boundary.
    pub import_reverse_generation_application:
        Arc<dyn a4n_novel_host_contract::ImportReverseGenerationApplicationPort>,
    /// Host-facing reverse-engineering confirmation boundary.
    pub import_reverse_confirmation_application:
        Arc<dyn a4n_api_contract::import_reverse::ImportReverseConfirmationApplicationPort>,
    /// Reverse-engineering candidate reads stay behind the API contract;
    /// generation and confirmation remain in the owner boundary.
    pub import_reverse_read_application:
        Arc<dyn a4n_api_contract::import_reverse::ImportReverseReadApplicationPort>,
    /// Host-facing chapter-analysis generation boundary. Analysis
    /// authorization, workflow state and summaries stay in the chapter owner.
    pub chapter_analysis_generation_application:
        Arc<dyn a4n_chapter_analysis_host_contract::ChapterAnalysisGenerationApplicationPort>,
    /// Host-facing chapter-analysis synchronization and workflow-recovery
    /// boundary.
    pub chapter_analysis_resume_application:
        Arc<dyn a4n_chapter_analysis_host_contract::ChapterAnalysisResumeApplicationPort>,
    /// Host-facing new chapter-analysis request-preview boundary.
    pub chapter_analysis_generation_preview_application: Arc<
        dyn a4n_chapter_analysis_host_contract::ChapterAnalysisGenerationPreviewApplicationPort,
    >,
    /// Host-facing chapter-analysis resume request-preview boundary.
    pub chapter_analysis_resume_preview_application:
        Arc<dyn a4n_chapter_analysis_host_contract::ChapterAnalysisResumePreviewApplicationPort>,
    /// Host-facing import-review ingestion boundary. Parsing stays in Runtime;
    /// queue persistence and projection stay in the import-review owner.
    pub import_review_ingestion_application:
        Arc<dyn a4n_novel_host_contract::ImportReviewIngestionApplicationPort>,
    /// Host-facing import-review acceptance boundary.
    pub import_review_acceptance_application:
        Arc<dyn a4n_api_contract::import_review::ImportReviewAcceptanceApplicationPort>,
    /// Host-facing import-review delete/reject boundary.
    pub import_review_mutation_application:
        Arc<dyn a4n_api_contract::import_review::ImportReviewMutationApplicationPort>,
    /// Host-facing import-review read boundary. Review reads and deduplication
    /// stay in the API contract rather than the mixed mutation boundary.
    pub import_review_read_application:
        Arc<dyn a4n_api_contract::import_review::ImportReviewReadApplicationPort>,
    /// Host-facing project export boundary. Authorization and format
    /// rendering remain in the export owner; Runtime only adapts progress.
    pub project_export_application: Arc<dyn a4n_api_contract::export::ProjectExportApplicationPort>,
    pub galgame_export_application: Arc<dyn a4n_api_contract::export::GalgameExportApplicationPort>,
    /// Host-facing video-record read boundary.
    pub video_record_read_application:
        Arc<dyn a4n_api_contract::video::VideoRecordReadApplicationPort>,
    /// Host-facing ordinary video-record mutation boundary.
    pub video_record_mutation_application:
        Arc<dyn a4n_api_contract::video::VideoRecordMutationApplicationPort>,
    /// Focused video import and trim worker boundaries.
    pub video_chapter_import_application:
        Arc<dyn a4n_api_contract::video::ChapterVideoImportApplicationPort>,
    pub video_reference_media_import_application:
        Arc<dyn a4n_api_contract::video::ReferenceMediaVideoImportApplicationPort>,
    pub video_assembly_import_application:
        Arc<dyn a4n_api_contract::video::AssemblyVideoImportApplicationPort>,
    pub video_canvas_import_application:
        Arc<dyn a4n_api_contract::video::CanvasVideoImportApplicationPort>,
    pub video_trim_application: Arc<dyn a4n_api_contract::video::VideoTrimApplicationPort>,
    /// Host-facing Assembly clip planning boundary.
    pub video_assembly_planning_application:
        Arc<dyn a4n_api_contract::video::VideoAssemblyPlanningApplicationPort>,
    /// Host-facing Assembly push boundary.
    pub video_assembly_push_application:
        Arc<dyn a4n_api_contract::video::VideoAssemblyPushApplicationPort>,
    pub video_assembly_postprocess_application:
        Arc<dyn a4n_api_contract::video::VideoAssemblyPostprocessApplicationPort>,
    /// Host-facing video completion boundary.
    pub video_record_completion_application:
        Arc<dyn a4n_api_contract::video::VideoRecordCompletionApplicationPort>,
    pub video_tail_frame_application:
        Arc<dyn a4n_api_contract::video::VideoTailFrameApplicationPort>,
    pub video_record_failure_application:
        Arc<dyn a4n_api_contract::video::VideoRecordFailureApplicationPort>,
    /// Host-facing media-cleanup lifecycle boundary.
    pub video_record_lifecycle_application:
        Arc<dyn a4n_api_contract::video::VideoRecordLifecycleApplicationPort>,
    /// Host-facing canvas-media preparation and persistence boundary.
    pub video_canvas_media_application:
        Arc<dyn a4n_api_contract::video::VideoCanvasMediaApplicationPort>,
    /// Host-facing chapter visual-plan read boundary. Visual-plan ownership
    /// and DTO projection stay outside Runtime.
    pub chapter_visual_plan_read_application:
        Arc<dyn a4n_api_contract::chapter_visual_plan::ChapterVisualPlanReadApplicationPort>,
    pub chapter_visual_plan_record_read_application: Arc<
        dyn a4n_api_contract::chapter_visual_plan::ChapterVisualPlanRecordReadApplicationPort,
    >,
    pub chapter_visual_plan_mutation_application:
        Arc<dyn a4n_api_contract::chapter_visual_plan::ChapterVisualPlanMutationApplicationPort>,
    pub chapter_visual_plan_record_persistence_application: Arc<
        dyn a4n_api_contract::chapter_visual_plan::ChapterVisualPlanRecordPersistenceApplicationPort,
    >,
    /// Host-facing video-prompt read boundary. Prompt authorization and DTO
    /// projection stay in the video owner.
    pub video_prompt_read_application:
        Arc<dyn a4n_api_contract::video::VideoPromptReadApplicationPort>,
    /// Host-facing video-prompt mutation boundary. Downstream cleanup and
    /// prompt rendering are supplied by Runtime through explicit ports.
    pub video_prompt_mutation_application:
        Arc<dyn a4n_api_contract::video::VideoPromptMutationApplicationPort>,
    /// Host-facing video timeline boundary. Timeline revision and ordering
    /// rules stay in the video owner.
    pub video_prompt_timeline_application:
        Arc<dyn a4n_api_contract::video::VideoPromptTimelineApplicationPort>,
    /// Host-facing segment-plan draft boundary. Workflow/checkpoint
    /// validation stays in the video owner.
    pub video_prompt_draft_application:
        Arc<dyn a4n_api_contract::video::VideoPromptDraftApplicationPort>,
    /// Host-facing generated-image read boundary. Authorization and DTO
    /// projection stay in the media owner.
    pub image_record_read_application:
        Arc<dyn a4n_api_contract::image::ImageRecordReadApplicationPort>,
    /// Host-facing generated-image mutation boundary.
    pub image_record_mutation_application:
        Arc<dyn a4n_api_contract::image::ImageRecordMutationApplicationPort>,
    /// Imported-image file and publication transaction owner.
    pub image_import_preparation_application:
        Arc<dyn a4n_image_host_contract::ImageImportPreparationApplicationPort>,
    pub image_import_publication_application:
        Arc<dyn a4n_image_host_contract::ImageImportPublicationApplicationPort>,
    /// Final imported-image publication is delegated to the Artifact/Media
    /// worker when the HTTP host supplies its endpoint.
    pub artifact_media_publication_application:
        Option<Arc<dyn a4n_artifact_media_contract::ArtifactMediaPublicationPort>>,
    pub artifact_media_reference_application:
        Option<Arc<dyn a4n_artifact_media_contract::ArtifactMediaReferencePort>>,
    /// Image publication recovery and state-transition owner.
    pub image_publication_recovery_application:
        Arc<dyn a4n_image_host_contract::ImagePublicationRecoveryPort>,
    /// Image publication recovery reads stay behind the API contract; Runtime
    /// supplies file facts and cleanup side effects separately.
    pub image_publication_recovery_read_application:
        Arc<dyn a4n_api_contract::image::ImagePublicationRecoveryReadPort>,
    /// Persisted media-reference queries used by cleanup and orphan scans.
    pub image_media_reference_application:
        Arc<dyn a4n_api_contract::image::ImageMediaReferencePort>,
    /// Image provider config lookup stays in the settings owner.
    pub image_provider_config: Arc<dyn a4n_api_contract::image::ImageProviderConfigPort>,
    /// Galgame image generation owner. Runtime supplies only provider and
    /// file adapters when a worker is started.
    pub galgame_image_generation_application:
        Arc<dyn a4n_image_host_contract::GalgameImageGenerationApplicationPort>,
    /// Canvas image request preview owner.
    pub image_canvas_request_preview_application:
        Arc<dyn a4n_image_host_contract::ImageCanvasRequestPreviewApplicationPort>,
    /// Batch image request preview owner.
    pub image_batch_request_preview_application:
        Arc<dyn a4n_image_host_contract::ImageBatchRequestPreviewApplicationPort>,
    /// Object profile image request preview owner.
    pub image_object_profile_request_preview_application:
        Arc<dyn a4n_image_host_contract::ImageObjectProfileRequestPreviewApplicationPort>,
    /// Manga page image request preview owner.
    pub image_page_request_preview_application:
        Arc<dyn a4n_image_host_contract::ImagePageRequestPreviewApplicationPort>,
    /// Scene image request preview owner.
    pub image_scene_request_preview_application:
        Arc<dyn a4n_image_host_contract::ImageSceneRequestPreviewApplicationPort>,
    /// Queued image generation lifecycle owner.
    pub image_generation_task_application:
        Arc<dyn a4n_image_host_contract::ImageGenerationTaskApplicationPort>,
    /// Profile image prompt generation has no provider/file task dependency.
    pub image_profile_prompt_application:
        Arc<dyn a4n_api_contract::image_generation::ImageProfilePromptApplicationPort>,
    /// Host-facing profile-material read boundary. Authorization and group
    /// projection stay in the media owner.
    pub profile_material_read_application:
        Arc<dyn a4n_api_contract::profile::ProfileMaterialReadApplicationPort>,
    pub profile_material_mutation_application:
        Arc<dyn a4n_api_contract::profile::ProfileMaterialMutationApplicationPort>,
    /// Upload publication uses a separate media/file adapter boundary.
    pub profile_material_upload_application:
        Arc<dyn a4n_image_host_contract::ProfileMaterialUploadApplicationPort>,
    /// Host-facing portrait-record read boundary. Portrait authorization and
    /// DTO projection stay in the character/portrait owner.
    pub portrait_record_read_application:
        Arc<dyn a4n_api_contract::portrait::PortraitRecordReadApplicationPort>,
    pub portrait_record_mutation_application:
        Arc<dyn a4n_api_contract::portrait::PortraitRecordMutationApplicationPort>,
    /// Portrait prompt request preview owner.
    pub portrait_prompt_request_preview_application:
        Arc<dyn a4n_portrait_host_contract::PortraitPromptRequestPreviewApplicationPort>,
    /// Portrait image request preview owner.
    pub portrait_image_request_preview_application:
        Arc<dyn a4n_portrait_host_contract::PortraitImageRequestPreviewApplicationPort>,
    /// Portrait prompt generation owner. Runtime supplies prompt rendering.
    pub portrait_prompt_application:
        Arc<dyn a4n_portrait_host_contract::PortraitPromptApplicationPort>,
    /// Portrait upload owner. Runtime supplies image/file adapters.
    pub portrait_upload_application:
        Arc<dyn a4n_portrait_host_contract::PortraitUploadApplicationPort>,
    /// Portrait generation task owner. Runtime supplies prompt and media
    /// adapters when a worker is started.
    pub portrait_generation_application:
        Arc<dyn a4n_portrait_host_contract::PortraitGenerationApplicationPort>,
    /// Persisted video provider configuration lookup. Runtime snapshot and
    /// user-scope policy stay outside this port.
    pub video_provider_config: Arc<dyn a4n_api_contract::video::VideoProviderConfigPort>,
    /// Video owner boundary for provider-job recovery and status transitions.
    pub video_provider_execution:
        Arc<dyn a4n_api_contract::video::VideoProviderExecutionApplicationPort>,
    /// Video generation task owner. Runtime supplies only provider adapters
    /// when a worker is started.
    pub video_generation_application:
        Arc<dyn a4n_api_contract::video::VideoGenerationApplicationPort>,
    /// Video request-preview workers are separated by request source.
    pub canvas_video_request_preview_application:
        Arc<dyn a4n_api_contract::video::CanvasVideoRequestPreviewApplicationPort>,
    pub chapter_video_request_preview_application:
        Arc<dyn a4n_api_contract::video::ChapterVideoRequestPreviewApplicationPort>,
    pub reference_media_video_request_preview_application:
        Arc<dyn a4n_api_contract::video::ReferenceMediaVideoRequestPreviewApplicationPort>,
    /// Host-facing Galgame pipeline execution boundary. Workflow
    /// authorization and persistence stay in the Galgame owner.
    pub galgame_pipeline_execution_application:
        Arc<dyn a4n_api_contract::galgame::GalgamePipelineExecutionApplicationPort>,
    /// Host-facing Galgame pipeline request-preview boundary.
    pub galgame_pipeline_preview_application:
        Arc<dyn a4n_api_contract::galgame::GalgamePipelinePreviewApplicationPort>,
    /// Host-facing Galgame pipeline status-read boundary.
    pub galgame_pipeline_read_application:
        Arc<dyn a4n_api_contract::galgame::GalgamePipelineReadApplicationPort>,
    /// Host-facing Galgame pipeline cancellation boundary.
    pub galgame_pipeline_cancellation_application:
        Arc<dyn a4n_api_contract::galgame::GalgamePipelineCancellationApplicationPort>,
    /// Galgame asset planning owner. Runtime receives only the stable plan.
    pub galgame_asset_plan_application:
        Arc<dyn a4n_api_contract::galgame::GalgameAssetPlanApplicationPort>,
    /// Galgame asset-generation owner. Runtime supplies visual/file adapters.
    pub galgame_asset_generation_application:
        Arc<dyn a4n_galgame_host_contract::GalgameAssetGenerationApplicationPort>,
    /// Host-facing Galgame script-record mutation boundary. Persistence and
    /// DTO projection remain in the Galgame owner.
    pub galgame_record_mutation_application:
        Arc<dyn a4n_api_contract::galgame::GalgameRecordMutationApplicationPort>,
    /// Host-facing Galgame media-binding mutation boundary.
    pub galgame_media_mutation_application:
        Arc<dyn a4n_api_contract::galgame::GalgameMediaMutationApplicationPort>,
    /// Host-facing Manga pipeline execution boundary. Workflow persistence
    /// remains in the Manga owner; Runtime only schedules and routes results.
    pub manga_pipeline_execution_application:
        Arc<dyn a4n_api_contract::manga::MangaPipelineExecutionApplicationPort>,
    /// Host-facing Manga pipeline request-preview boundary.
    pub manga_pipeline_preview_application:
        Arc<dyn a4n_api_contract::manga::MangaPipelinePreviewApplicationPort>,
    /// Host-facing Manga pipeline status-read boundary.
    pub manga_pipeline_read_application:
        Arc<dyn a4n_api_contract::manga::MangaPipelineReadApplicationPort>,
    /// Host-facing Manga pipeline cancellation boundary.
    pub manga_pipeline_cancellation_application:
        Arc<dyn a4n_api_contract::manga::MangaPipelineCancellationApplicationPort>,
    /// Host-facing Manga prompt read boundary. Prompt projection remains in
    /// the Manga owner.
    pub manga_record_read_application:
        Arc<dyn a4n_api_contract::manga::MangaRecordReadApplicationPort>,
    /// Host-facing Manga prompt mutation boundary. Persistence and cleanup
    /// decisions remain in the Manga owner.
    pub manga_record_mutation_application:
        Arc<dyn a4n_api_contract::manga::MangaRecordMutationApplicationPort>,
    /// Host-facing Galgame read boundary. Script authorization and projection
    /// stay in the Galgame owner.
    pub galgame_read_application: Arc<dyn a4n_api_contract::galgame::GalgameReadApplicationPort>,
    /// Host-facing inspiration history read boundary. Authorization and
    /// message projection stay in the inspiration owner.
    pub inspiration_history_read_application:
        Arc<dyn a4n_api_contract::inspiration::InspirationHistoryReadApplicationPort>,
    /// Inspiration generation owner. Runtime only schedules the worker and
    /// supplies the prompt-rendering adapter.
    pub inspiration_generation_application:
        Arc<dyn a4n_inspiration_host_contract::InspirationGenerationApplicationPort>,
    /// Inspiration request construction boundary. Runtime supplies prompt
    /// rendering and provider preview budgeting after this call returns.
    pub inspiration_request_application:
        Arc<dyn a4n_inspiration_host_contract::InspirationRequestPreviewApplicationPort>,
}
