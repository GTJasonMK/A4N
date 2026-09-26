//! Host-side composition root for Runtime and Orchestrator.
//!
//! This crate owns construction only. HTTP routing, Tauri commands, and domain
//! behavior remain in their respective owners.

#[cfg(test)]
mod media_lifecycle_tests;
mod task_queue;

use std::collections::HashSet;
use std::sync::Arc;

use a4n_error::AppError;
use a4n_orchestrator::OrchestratorContext;
use a4n_runtime::{AppState, ApplicationPorts, RuntimeBootstrap, RuntimeProfile, RuntimeRole};
use a4n_task_runtime::persistent::PersistentTaskQueue;

pub async fn initialize_runtime(
    config: a4n_config::app_config::AppConfig,
    runtime_profile: RuntimeProfile,
    runtime_role: RuntimeRole,
) -> Result<AppState, AppError> {
    initialize_runtime_with_ports(config, runtime_profile, runtime_role, |_| {}).await
}

/// Binds Runtime application adapters before startup uses them.
async fn initialize_runtime_with_ports(
    config: a4n_config::app_config::AppConfig,
    runtime_profile: RuntimeProfile,
    runtime_role: RuntimeRole,
    configure_ports: impl FnOnce(&mut ApplicationPorts),
) -> Result<AppState, AppError> {
    let bootstrap = RuntimeBootstrap::prepare(config, runtime_profile, runtime_role).await?;
    let database = bootstrap.database();
    let task_queue = Arc::new(PersistentTaskQueue::new(database.clone()).await?);
    let task_queue_application =
        Arc::new(task_queue::TaskQueueApplication::new(task_queue.clone()));
    let novel_planning_worker =
        Arc::new(a4n_novel_planning_worker_client::NovelPlanningWorkerClient::from_env());
    let mut provider_runtime = None;
    let orchestrator = Arc::new(OrchestratorContext::bootstrap(
        database.clone(),
        bootstrap.prompt_client(),
        bootstrap.service_manager(),
        bootstrap.config().clone(),
        bootstrap.event_bus(),
        bootstrap.task_manager(),
        bootstrap.rag_store(),
        task_queue,
        bootstrap.prompt_engine(),
        bootstrap.document_export_renderer()?,
        novel_planning_worker.clone(),
        |provider_port| {
            let (provider_facade, llm, embedding) = bootstrap.create_provider(provider_port)?;
            provider_runtime = Some(provider_facade);
            Ok((llm, embedding))
        },
    )?);
    let auth_application = Arc::new(orchestrator.auth_application());
    let agent_resource_access_application = orchestrator.agent_resource_access_application();
    let agent_worker_application = orchestrator.agent_worker_application();

    let prompt_read_application = orchestrator.clone();
    let prompt_mutation_application = orchestrator.clone();
    let package_read_application = orchestrator.clone();
    let package_mutation_application = orchestrator.clone();
    let rag_search = orchestrator.clone();
    let rag_ingest = orchestrator.clone();
    let artifact_media_worker = artifact_media_worker_for_profile(runtime_profile)?;
    let agent_worker = Arc::new(a4n_agent_worker_client::AgentWorkerClient::from_env());
    let blueprint_worker = Arc::new(a4n_blueprint_worker_client::BlueprintWorkerClient::from_env());
    let outline_worker = Arc::new(a4n_outline_worker_client::OutlineWorkerClient::from_env());
    let asset_graph_worker =
        Arc::new(a4n_asset_graph_worker_client::AssetGraphWorkerClient::from_env());
    let canvas_record_worker =
        Arc::new(a4n_canvas_record_worker_client::CanvasRecordWorkerClient::from_env());
    let foreshadowing_worker =
        Arc::new(a4n_foreshadowing_worker_client::ForeshadowingWorkerClient::from_env());
    let object_profile_worker =
        Arc::new(a4n_object_profile_worker_client::ObjectProfileWorkerClient::from_env());
    let scene_profile_worker =
        Arc::new(a4n_scene_profile_worker_client::SceneProfileWorkerClient::from_env());
    let portrait_worker = Arc::new(a4n_portrait_worker_client::PortraitWorkerClient::from_env());
    let source_snapshot_worker =
        Arc::new(a4n_source_snapshot_worker_client::SourceSnapshotWorkerClient::from_env());
    let profile_material_worker =
        Arc::new(a4n_profile_material_worker_client::ProfileMaterialWorkerClient::from_env());
    let project_status_worker =
        Arc::new(a4n_project_status_worker_client::ProjectStatusWorkerClient::from_env());
    let project_read_worker =
        Arc::new(a4n_project_read_worker_client::ProjectReadWorkerClient::from_env());
    let import_review_worker =
        Arc::new(a4n_import_review_worker_client::ImportReviewWorkerClient::from_env());
    let inspiration_worker =
        Arc::new(a4n_inspiration_worker_client::InspirationWorkerClient::from_env());
    let project_access_worker =
        Arc::new(a4n_project_access_worker_client::ProjectAccessWorkerClient::from_env());
    let project_collaboration_worker = Arc::new(
        a4n_project_collaboration_worker_client::ProjectCollaborationWorkerClient::from_env(),
    );
    let admin_worker = Arc::new(a4n_admin_worker_client::AdminWorkerClient::from_env());
    let project_mutation_worker =
        Arc::new(a4n_project_mutation_worker_client::ProjectMutationWorkerClient::from_env());
    let chapter_scene_worker =
        Arc::new(a4n_chapter_scene_worker_client::ChapterSceneWorkerClient::from_env());
    let chapter_content_worker =
        Arc::new(a4n_chapter_content_worker_client::ChapterContentWorkerClient::from_env());
    let chapter_content_sync_application: Arc<
        dyn a4n_api_contract::chapter::ChapterContentSyncApplicationPort,
    > = orchestrator.clone();
    let chapter_content_mutation_application: Arc<
        dyn a4n_api_contract::chapter::ChapterContentMutationApplicationPort,
    > = Arc::new(ChapterContentMutationAdapter {
        worker: chapter_content_worker.clone(),
        sync: chapter_content_sync_application.clone(),
    });
    let workflow_read_worker =
        Arc::new(a4n_workflow_read_worker_client::WorkflowReadWorkerClient::from_env());
    let workflow_lifecycle_worker =
        Arc::new(a4n_workflow_lifecycle_worker_client::WorkflowLifecycleWorkerClient::from_env());
    let workflow_cancellation_application: Arc<
        dyn a4n_api_contract::workflow::WorkflowCancellationPort,
    > = Arc::new(WorkflowCancellationAdapter {
        lifecycle: workflow_lifecycle_worker,
        side_effects: orchestrator.clone(),
    });
    let config_read_worker =
        Arc::new(a4n_config_read_worker_client::ConfigReadWorkerClient::from_env());
    let character_mutation_worker =
        Arc::new(a4n_character_mutation_worker_client::CharacterMutationWorkerClient::from_env());
    let video_record_worker =
        Arc::new(a4n_video_record_worker_client::VideoRecordWorkerClient::from_env());
    let video_record_mutation_application: Arc<
        dyn a4n_api_contract::video::VideoRecordMutationApplicationPort,
    > = Arc::new(VideoRecordMutationAdapter {
        worker: video_record_worker.clone(),
        orchestrator: orchestrator.clone(),
    });
    let video_prompt_draft_application: Arc<
        dyn a4n_api_contract::video::VideoPromptDraftApplicationPort,
    > = Arc::new(VideoPromptDraftAdapter {
        worker: video_record_worker.clone(),
    });
    let manga_record_worker =
        Arc::new(a4n_manga_record_worker_client::MangaRecordWorkerClient::from_env());
    let theme_read_worker =
        Arc::new(a4n_theme_read_worker_client::ThemeReadWorkerClient::from_env());
    let artifact_media_publication_application = artifact_media_worker
        .clone()
        .map(|client| client as Arc<dyn a4n_artifact_media_contract::ArtifactMediaPublicationPort>);
    let image_media_reference_application: Arc<
        dyn a4n_api_contract::image::ImageMediaReferencePort,
    > = if let Some(client) = artifact_media_worker.clone() {
        Arc::new(ArtifactMediaImageReferenceAdapter { client })
    } else {
        orchestrator.clone()
    };
    let image_library_read_application: Arc<
        dyn a4n_api_contract::image::ImageLibraryReadApplicationPort,
    > = if let Some(client) = artifact_media_worker.clone() {
        client
    } else {
        orchestrator.clone()
    };
    let image_record_mutation_application: Arc<
        dyn a4n_api_contract::image::ImageRecordMutationApplicationPort,
    > = if let Some(client) = artifact_media_worker.clone() {
        client
    } else {
        orchestrator.clone()
    };
    let image_reference_read_application: Arc<
        dyn a4n_api_contract::image::ImageReferenceReadApplicationPort,
    > = if let Some(client) = artifact_media_worker.clone() {
        client
    } else {
        orchestrator.clone()
    };
    let artifact_media_reference_application = artifact_media_worker
        .map(|client| client as Arc<dyn a4n_artifact_media_contract::ArtifactMediaReferencePort>);
    let mut application_ports = ApplicationPorts {
        generation_task_read_application: orchestrator.clone(),
        generation_task_mutation_application: orchestrator.clone(),
        generation_lifecycle_application: orchestrator.clone(),
        generation_cleanup_application: orchestrator.clone(),
        source_sync_read_application: orchestrator.clone(),
        source_sync_mutation_application: orchestrator.clone(),
        shooting_script_current_application: novel_planning_worker.clone(),
        shooting_script_unit_progress_application: novel_planning_worker.clone(),
        shooting_script_record_mutation_application: novel_planning_worker.clone(),
        shooting_script_generation_application: orchestrator.clone(),
        auth_read_application: auth_application.clone(),
        auth_mutation_application: auth_application,
        admin_read_application: admin_worker.clone(),
        model_usage_read_application: orchestrator.clone(),
        admin_mutation_application: admin_worker,
        project_status_read_application: project_status_worker.clone(),
        project_status_mutation_application: project_status_worker,
        project_collaboration_application: project_collaboration_worker,
        project_access_application: project_access_worker,
        project_read_application: project_read_worker,
        project_source_snapshot_read_application: source_snapshot_worker,
        project_mutation_application: project_mutation_worker,
        project_asset_graph_application: asset_graph_worker,
        project_duplication_application: orchestrator.clone(),
        project_family_transfer_application: orchestrator.clone(),
        project_bundle_transfer_application: orchestrator.clone(),
        video_studio_transfer_application: orchestrator.clone(),
        chapter_scene_read_application: chapter_scene_worker.clone(),
        chapter_scene_mutation_application: chapter_scene_worker,
        chapter_scene_generation_application: orchestrator.clone(),
        optimize_session_read_application: orchestrator.clone(),
        optimize_agent_tool_application: orchestrator.clone(),
        optimize_session_mutation_application: orchestrator.clone(),
        optimize_patch_application: orchestrator.clone(),
        optimize_patch_preview_application: orchestrator.clone(),
        optimize_analysis_preview_application: orchestrator.clone(),
        optimize_chat_preview_application: orchestrator.clone(),
        optimize_chat_generation_application: orchestrator.clone(),
        optimize_analysis_generation_application: orchestrator.clone(),
        optimize_patch_generation_application: orchestrator.clone(),
        scene_profile_sync_application: orchestrator.clone(),
        cleanup_outbox_application: orchestrator.clone(),
        character_discovery_application: orchestrator.clone(),
        character_profile_sync_application: orchestrator.clone(),
        workflow_read_application: workflow_read_worker.clone(),
        workflow_task_lookup_application: workflow_read_worker.clone(),
        task_read_application: workflow_read_worker,
        task_cancellation_application: task_queue_application.clone(),
        task_queue_stats_application: task_queue_application,
        workflow_cancellation_application,
        object_profile_read_application: object_profile_worker.clone(),
        object_profile_mutation_application: object_profile_worker,
        scene_profile_read_application: scene_profile_worker.clone(),
        scene_profile_mutation_application: scene_profile_worker.clone(),
        scene_profile_record_sync_application: scene_profile_worker,
        character_profile_overview_read_application: character_mutation_worker.clone(),
        character_timeline_read_application: character_mutation_worker.clone(),
        character_profile_mutation_application: character_mutation_worker.clone(),
        character_profile_timeline_mutation_application: character_mutation_worker.clone(),
        character_profile_attribute_mutation_application: character_mutation_worker.clone(),
        character_profile_attribute_deletion_application: character_mutation_worker.clone(),
        character_implicit_review_check_application: orchestrator.clone(),
        character_implicit_review_preview_application: orchestrator.clone(),
        character_implicit_review_apply_application: orchestrator.clone(),
        character_implicit_review_read_application: orchestrator.clone(),
        character_profile_deletion_application: character_mutation_worker,
        character_conflict_application: orchestrator.clone(),
        foreshadowing_read_application: foreshadowing_worker.clone(),
        foreshadowing_mutation_application: foreshadowing_worker,
        blueprint_read_application: blueprint_worker.clone(),
        blueprint_generation_preview_application: orchestrator.clone(),
        blueprint_refinement_preview_application: orchestrator.clone(),
        blueprint_generation_application: orchestrator.clone(),
        blueprint_refinement_application: orchestrator.clone(),
        blueprint_mutation_application: orchestrator.clone(),
        chapter_content_read_application: chapter_content_worker,
        chapter_content_mutation_application,
        chapter_content_sync_application,
        chapter_generation_request_preview_application: orchestrator.clone(),
        chapter_generation_rag_preparation_application: orchestrator.clone(),
        chapter_generation_application: orchestrator.clone(),
        outline_read_application: outline_worker.clone(),
        outline_mutation_application: outline_worker.clone(),
        outline_generation_application: orchestrator.clone(),
        outline_part_generation_preview_application: orchestrator.clone(),
        outline_part_mutation_preview_application: orchestrator.clone(),
        outline_part_chapter_generation_preview_application: orchestrator.clone(),
        outline_batch_part_generation_preview_application: orchestrator.clone(),
        outline_chapter_count_generation_preview_application: orchestrator.clone(),
        outline_chapter_regeneration_preview_application: orchestrator.clone(),
        outline_beat_mapping_preview_application: orchestrator.clone(),
        theme_read_application: theme_read_worker.clone(),
        theme_mutation_application: theme_read_worker,
        config_read_application: config_read_worker.clone(),
        config_mutation_application: config_read_worker.clone(),
        settings_export_application: config_read_worker.clone(),
        settings_import_application: config_read_worker,
        prompt_read_application,
        prompt_mutation_application,
        agent_thread_mutation_application: agent_worker.clone(),
        agent_thread_read_application: agent_worker.clone(),
        agent_run_mutation_application: agent_worker.clone(),
        agent_run_read_application: agent_worker.clone(),
        agent_delegation_application: agent_worker.clone(),
        agent_tool_read_application: agent_worker.clone(),
        agent_skill_application: agent_worker.clone(),
        agent_event_read_application: agent_worker.clone(),
        agent_memory_read_application: agent_worker.clone(),
        agent_memory_mutation_application: agent_worker.clone(),
        agent_approval_application: agent_worker.clone(),
        agent_resource_access_application,
        agent_worker_application,
        canvas_read_application: canvas_record_worker.clone(),
        canvas_mutation_application: canvas_record_worker,
        canvas_render_text_application: orchestrator.clone(),
        canvas_llm_text_application: orchestrator.clone(),
        canvas_image_application: orchestrator.clone(),
        canvas_video_application: orchestrator.clone(),
        canvas_preview_application: orchestrator.clone(),
        canvas_editing_application: orchestrator.clone(),
        canvas_probe_application: orchestrator.clone(),
        canvas_output_application: orchestrator.clone(),
        canvas_image_import_application: orchestrator.clone(),
        canvas_video_import_application: orchestrator.clone(),
        canvas_audio_import_application: orchestrator.clone(),
        canvas_graph_preparation_application: orchestrator.clone(),
        canvas_graph_execution_application: orchestrator.clone(),
        canvas_graph_failure_application: orchestrator.clone(),
        package_read_application,
        package_mutation_application,
        import_analysis_progress_application: orchestrator.clone(),
        import_analysis_preview_application: orchestrator.clone(),
        import_analysis_generation_application: orchestrator.clone(),
        import_analysis_cancellation_application: orchestrator.clone(),
        import_reverse_preview_application: orchestrator.clone(),
        import_reverse_generation_application: orchestrator.clone(),
        import_reverse_confirmation_application: orchestrator.clone(),
        import_reverse_read_application: orchestrator.clone(),
        chapter_analysis_generation_application: orchestrator.clone(),
        project_fact_feedback_application: orchestrator.clone(),
        chapter_analysis_resume_application: orchestrator.clone(),
        chapter_analysis_generation_preview_application: orchestrator.clone(),
        chapter_analysis_resume_preview_application: orchestrator.clone(),
        import_review_ingestion_application: orchestrator.clone(),
        import_review_acceptance_application: orchestrator.clone(),
        import_review_mutation_application: import_review_worker.clone(),
        import_review_read_application: import_review_worker,
        project_export_application: orchestrator.clone(),
        galgame_export_application: orchestrator.clone(),
        video_record_read_application: video_record_worker.clone(),
        video_record_mutation_application,
        video_chapter_import_application: orchestrator.clone(),
        video_reference_media_import_application: orchestrator.clone(),
        video_assembly_import_application: orchestrator.clone(),
        video_canvas_import_application: orchestrator.clone(),
        video_trim_application: orchestrator.clone(),
        video_assembly_planning_application: orchestrator.clone(),
        video_assembly_push_application: orchestrator.clone(),
        video_assembly_postprocess_application: orchestrator.clone(),
        video_record_completion_application: orchestrator.clone(),
        video_tail_frame_application: orchestrator.clone(),
        video_record_failure_application: orchestrator.clone(),
        video_record_lifecycle_application: orchestrator.clone(),
        video_canvas_media_application: orchestrator.clone(),
        chapter_visual_plan_read_application: orchestrator.clone(),
        chapter_visual_plan_record_read_application: novel_planning_worker.clone(),
        chapter_visual_plan_mutation_application: orchestrator.clone(),
        chapter_visual_plan_record_persistence_application: novel_planning_worker.clone(),
        video_prompt_read_application: video_record_worker.clone(),
        video_prompt_mutation_application: orchestrator.clone(),
        video_prompt_timeline_application: orchestrator.clone(),
        video_prompt_draft_application,
        image_record_read_application: orchestrator.clone(),
        image_reference_read_application,
        image_library_read_application,
        image_record_mutation_application,
        image_import_preparation_application: orchestrator.clone(),
        image_import_publication_application: orchestrator.clone(),
        artifact_media_publication_application,
        artifact_media_reference_application,
        image_publication_recovery_application: orchestrator.clone(),
        image_publication_recovery_read_application: orchestrator.clone(),
        image_media_reference_application,
        image_provider_config: orchestrator.clone(),
        galgame_image_generation_application: orchestrator.clone(),
        image_canvas_request_preview_application: orchestrator.clone(),
        image_batch_request_preview_application: orchestrator.clone(),
        image_object_profile_request_preview_application: orchestrator.clone(),
        image_page_request_preview_application: orchestrator.clone(),
        image_scene_request_preview_application: orchestrator.clone(),
        image_generation_task_application: orchestrator.clone(),
        image_profile_prompt_application: orchestrator.clone(),
        profile_material_read_application: profile_material_worker.clone(),
        profile_material_mutation_application: profile_material_worker,
        profile_material_upload_application: orchestrator.clone(),
        portrait_record_read_application: portrait_worker.clone(),
        portrait_record_mutation_application: portrait_worker,
        portrait_prompt_request_preview_application: orchestrator.clone(),
        portrait_image_request_preview_application: orchestrator.clone(),
        portrait_prompt_application: orchestrator.clone(),
        portrait_upload_application: orchestrator.clone(),
        portrait_generation_application: orchestrator.clone(),
        video_provider_config: orchestrator.clone(),
        video_provider_execution: orchestrator.clone(),
        video_generation_application: orchestrator.clone(),
        canvas_video_request_preview_application: orchestrator.clone(),
        chapter_video_request_preview_application: orchestrator.clone(),
        reference_media_video_request_preview_application: orchestrator.clone(),
        galgame_pipeline_execution_application: orchestrator.clone(),
        galgame_pipeline_preview_application: orchestrator.clone(),
        galgame_pipeline_read_application: orchestrator.clone(),
        galgame_pipeline_cancellation_application: orchestrator.clone(),
        galgame_asset_plan_application: orchestrator.clone(),
        galgame_asset_generation_application: orchestrator.clone(),
        galgame_record_mutation_application: orchestrator.clone(),
        galgame_media_mutation_application: orchestrator.clone(),
        manga_pipeline_execution_application: orchestrator.clone(),
        manga_pipeline_preview_application: orchestrator.clone(),
        manga_pipeline_read_application: manga_record_worker.clone(),
        manga_pipeline_cancellation_application: manga_record_worker.clone(),
        manga_record_read_application: manga_record_worker,
        manga_record_mutation_application: orchestrator.clone(),
        galgame_read_application: orchestrator.clone(),
        inspiration_history_read_application: inspiration_worker,
        inspiration_generation_application: orchestrator.clone(),
        inspiration_request_application: orchestrator.clone(),
    };
    configure_ports(&mut application_ports);
    let provider_runtime = provider_runtime.ok_or_else(|| AppError::InvalidState {
        current: "provider runtime port 未初始化".to_string(),
        operation: "构造 Runtime provider".to_string(),
    })?;
    bootstrap
        .finish(application_ports, provider_runtime, rag_search, rag_ingest)
        .await
}

struct ArtifactMediaImageReferenceAdapter {
    client: Arc<a4n_artifact_media_client::ArtifactMediaWorkerClient>,
}

struct WorkflowCancellationAdapter {
    lifecycle: Arc<dyn a4n_api_contract::workflow::WorkflowLifecycleApplicationPort>,
    side_effects: Arc<dyn a4n_api_contract::workflow::WorkflowCancellationSideEffectsPort>,
}

#[async_trait::async_trait]
impl a4n_api_contract::workflow::WorkflowCancellationPort for WorkflowCancellationAdapter {
    async fn mark_running(&self, workflow_run_id: i64) -> Result<(), AppError> {
        self.lifecycle.mark_running(workflow_run_id).await
    }

    async fn cancel(&self, workflow_run_id: i64, reason: &str) -> Result<(), AppError> {
        let cancellation = self.lifecycle.cancel(workflow_run_id, reason).await?;
        self.side_effects
            .apply_cancellation_side_effects(cancellation, reason)
            .await
    }

    async fn resume_with_active_task(
        &self,
        workflow_run_id: i64,
        active_task_id: &str,
    ) -> Result<(), AppError> {
        self.lifecycle
            .resume_with_active_task(workflow_run_id, active_task_id)
            .await
    }
}

struct VideoRecordMutationAdapter {
    worker: Arc<a4n_video_record_worker_client::VideoRecordWorkerClient>,
    orchestrator: Arc<dyn a4n_api_contract::video::VideoRecordMutationApplicationPort>,
}

#[async_trait::async_trait]
impl a4n_api_contract::video::VideoRecordMutationApplicationPort for VideoRecordMutationAdapter {
    async fn update(
        &self,
        user_id: i64,
        video_id: i64,
        input: a4n_api_contract::video::UpdateGeneratedVideoInput,
    ) -> Result<a4n_api_contract::video::GeneratedVideoSummary, a4n_error::AppError> {
        self.worker.update_record(user_id, video_id, input).await
    }

    async fn select_segment(
        &self,
        user_id: i64,
        project_id: i64,
        chapter_id: i64,
        video_id: i64,
        input: a4n_api_contract::video::SelectSegmentVideoInput,
    ) -> Result<a4n_api_contract::video::GeneratedVideoSummary, a4n_error::AppError> {
        self.worker
            .select_segment(user_id, project_id, chapter_id, video_id, input)
            .await
    }

    async fn delete(&self, user_id: i64, video_id: i64) -> Result<(), a4n_error::AppError> {
        self.orchestrator.delete(user_id, video_id).await
    }
}

struct VideoPromptDraftAdapter {
    worker: Arc<a4n_video_record_worker_client::VideoRecordWorkerClient>,
}

struct ChapterContentMutationAdapter {
    worker: Arc<a4n_chapter_content_worker_client::ChapterContentWorkerClient>,
    sync: Arc<dyn a4n_api_contract::chapter::ChapterContentSyncApplicationPort>,
}

#[async_trait::async_trait]
impl a4n_api_contract::chapter::ChapterContentMutationApplicationPort
    for ChapterContentMutationAdapter
{
    async fn update(
        &self,
        user_id: i64,
        host_port: &dyn a4n_api_contract::chapter::ChapterDerivedCleanupHostPort,
        input: a4n_api_contract::chapter::UpdateChapterContentInput,
    ) -> Result<i64, a4n_error::AppError> {
        let result = self.worker.update_content(user_id, input).await?;
        if let Err(error) = self.sync.settle(host_port, result.workflow_run_id).await {
            tracing::warn!(
                chapter_id = result.chapter_id,
                workflow_run_id = result.workflow_run_id,
                error = %error,
                "章节正文已保存，派生资料同步待恢复"
            );
        }
        Ok(result.chapter_id)
    }

    async fn import(
        &self,
        user_id: i64,
        host_port: &dyn a4n_api_contract::chapter::ChapterDerivedCleanupHostPort,
        input: a4n_api_contract::chapter::ImportChapterInput,
    ) -> Result<i64, a4n_error::AppError> {
        let result = self.worker.import_content(user_id, input).await?;
        if let Err(error) = self.sync.settle(host_port, result.workflow_run_id).await {
            tracing::warn!(
                chapter_id = result.chapter_id,
                workflow_run_id = result.workflow_run_id,
                error = %error,
                "导入正文已保存，派生资料同步待恢复"
            );
        }
        Ok(result.chapter_id)
    }
}

#[async_trait::async_trait]
impl a4n_api_contract::video::VideoPromptDraftApplicationPort for VideoPromptDraftAdapter {
    async fn load_video_segment_plan_draft(
        &self,
        user_id: i64,
        input: a4n_api_contract::video::LoadVideoSegmentPlanDraftInput,
    ) -> Result<a4n_api_contract::video::VideoSegmentPlanDraftSummary, a4n_error::AppError> {
        self.worker
            .load_video_segment_plan_draft(user_id, input)
            .await
    }

    async fn update_video_segment_plan_draft_recipe(
        &self,
        user_id: i64,
        input: a4n_api_contract::video::UpdateVideoSegmentPlanDraftRecipeInput,
    ) -> Result<(), a4n_error::AppError> {
        self.worker
            .update_video_segment_plan_draft_recipe(user_id, input)
            .await
    }
}

#[async_trait::async_trait]
impl a4n_api_contract::image::ImageMediaReferencePort for ArtifactMediaImageReferenceAdapter {
    async fn unreferenced_media_paths(
        &self,
        paths: &[String],
    ) -> Result<Vec<String>, a4n_error::AppError> {
        self.client
            .unreferenced_paths(a4n_artifact_media_contract::MediaPathsRequest {
                paths: paths.to_vec(),
            })
            .await
            .map_err(|error| a4n_error::AppError::ConfigError(error.to_string()))
    }

    async fn media_path_prefix_has_persisted_reference(
        &self,
        prefix: &str,
    ) -> Result<bool, a4n_error::AppError> {
        self.client
            .path_prefix_has_reference(a4n_artifact_media_contract::MediaPathPrefixRequest {
                prefix: prefix.to_string(),
            })
            .await
            .map_err(|error| a4n_error::AppError::ConfigError(error.to_string()))
    }

    async fn project_has_active_image_publication(
        &self,
        project_id: i64,
    ) -> Result<bool, a4n_error::AppError> {
        self.client
            .project_has_publication(a4n_artifact_media_contract::ProjectPublicationRequest {
                project_id,
            })
            .await
            .map_err(|error| a4n_error::AppError::ConfigError(error.to_string()))
    }

    async fn protected_image_publication_cleanup_paths(
        &self,
        project_id: i64,
        paths: &[String],
    ) -> Result<HashSet<String>, a4n_error::AppError> {
        let paths = self
            .client
            .protected_paths(a4n_artifact_media_contract::ProjectPathsRequest {
                project_id,
                paths: paths.to_vec(),
            })
            .await
            .map_err(|error| a4n_error::AppError::ConfigError(error.to_string()))?;
        Ok(paths.into_iter().collect())
    }
}

fn artifact_media_worker_for_profile(
    runtime_profile: RuntimeProfile,
) -> Result<Option<Arc<a4n_artifact_media_client::ArtifactMediaWorkerClient>>, AppError> {
    if !matches!(runtime_profile, RuntimeProfile::Sidecar) {
        return Ok(None);
    }

    let client = a4n_artifact_media_client::ArtifactMediaWorkerClient::from_env().ok_or_else(
        || {
            AppError::ConfigError(
                "sidecar runtime requires A4N_ARTIFACT_MEDIA_WORKER_URL and A4N_ARTIFACT_MEDIA_WORKER_TOKEN"
                    .to_string(),
            )
        },
    )?;
    Ok(Some(Arc::new(client)))
}
