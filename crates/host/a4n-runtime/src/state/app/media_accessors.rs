use super::*;

impl AppState {
    pub(crate) fn chapter_analysis_resume_preview_application(
        &self,
    ) -> &dyn a4n_chapter_analysis_host_contract::ChapterAnalysisResumePreviewApplicationPort {
        self.chapter_analysis_resume_preview_application.as_ref()
    }

    pub(crate) fn import_review_ingestion_application(
        &self,
    ) -> &dyn a4n_novel_host_contract::ImportReviewIngestionApplicationPort {
        self.import_review_ingestion_application.as_ref()
    }

    pub(crate) fn import_review_acceptance_application(
        &self,
    ) -> &dyn a4n_api_contract::import_review::ImportReviewAcceptanceApplicationPort {
        self.import_review_acceptance_application.as_ref()
    }

    pub(crate) fn import_review_mutation_application(
        &self,
    ) -> &dyn a4n_api_contract::import_review::ImportReviewMutationApplicationPort {
        self.import_review_mutation_application.as_ref()
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

    pub(crate) fn video_chapter_import_application(
        &self,
    ) -> &dyn a4n_api_contract::video::ChapterVideoImportApplicationPort {
        self.video_chapter_import_application.as_ref()
    }

    pub(crate) fn video_reference_media_import_application(
        &self,
    ) -> &dyn a4n_api_contract::video::ReferenceMediaVideoImportApplicationPort {
        self.video_reference_media_import_application.as_ref()
    }

    pub(crate) fn video_assembly_import_application(
        &self,
    ) -> &dyn a4n_api_contract::video::AssemblyVideoImportApplicationPort {
        self.video_assembly_import_application.as_ref()
    }

    pub(crate) fn video_canvas_import_application(
        &self,
    ) -> &dyn a4n_api_contract::video::CanvasVideoImportApplicationPort {
        self.video_canvas_import_application.as_ref()
    }

    pub(crate) fn video_trim_application(
        &self,
    ) -> &dyn a4n_api_contract::video::VideoTrimApplicationPort {
        self.video_trim_application.as_ref()
    }

    pub(crate) fn video_assembly_planning_application(
        &self,
    ) -> &dyn a4n_api_contract::video::VideoAssemblyPlanningApplicationPort {
        self.video_assembly_planning_application.as_ref()
    }

    pub(crate) fn video_assembly_push_application(
        &self,
    ) -> &dyn a4n_api_contract::video::VideoAssemblyPushApplicationPort {
        self.video_assembly_push_application.as_ref()
    }

    pub(crate) fn video_assembly_postprocess_application(
        &self,
    ) -> &dyn a4n_api_contract::video::VideoAssemblyPostprocessApplicationPort {
        self.video_assembly_postprocess_application.as_ref()
    }

    pub(crate) fn video_record_lifecycle_application(
        &self,
    ) -> &dyn a4n_api_contract::video::VideoRecordLifecycleApplicationPort {
        self.video_record_lifecycle_application.as_ref()
    }

    pub(crate) fn video_record_completion_application(
        &self,
    ) -> &dyn a4n_api_contract::video::VideoRecordCompletionApplicationPort {
        self.video_record_completion_application.as_ref()
    }

    pub(crate) fn video_tail_frame_application(
        &self,
    ) -> &dyn a4n_api_contract::video::VideoTailFrameApplicationPort {
        self.video_tail_frame_application.as_ref()
    }

    pub(crate) fn video_record_failure_application(
        &self,
    ) -> &dyn a4n_api_contract::video::VideoRecordFailureApplicationPort {
        self.video_record_failure_application.as_ref()
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

    pub(crate) fn chapter_visual_plan_record_read_application(
        &self,
    ) -> &dyn a4n_api_contract::chapter_visual_plan::ChapterVisualPlanRecordReadApplicationPort
    {
        self.chapter_visual_plan_record_read_application.as_ref()
    }

    pub(crate) fn chapter_visual_plan_mutation_application(
        &self,
    ) -> &dyn a4n_api_contract::chapter_visual_plan::ChapterVisualPlanMutationApplicationPort {
        self.chapter_visual_plan_mutation_application.as_ref()
    }

    pub(crate) fn chapter_visual_plan_record_persistence_application(
        &self,
    ) -> &dyn a4n_api_contract::chapter_visual_plan::ChapterVisualPlanRecordPersistenceApplicationPort
    {
        self.chapter_visual_plan_record_persistence_application
            .as_ref()
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

    pub fn image_record_read_application(
        &self,
    ) -> &dyn a4n_api_contract::image::ImageRecordReadApplicationPort {
        self.image_record_read_application.as_ref()
    }

    pub(crate) fn image_reference_read_application(
        &self,
    ) -> &dyn a4n_api_contract::image::ImageReferenceReadApplicationPort {
        self.image_reference_read_application.as_ref()
    }

    pub(crate) fn image_library_read_application(
        &self,
    ) -> &dyn a4n_api_contract::image::ImageLibraryReadApplicationPort {
        self.image_library_read_application.as_ref()
    }

    pub(crate) fn image_record_mutation_application(
        &self,
    ) -> &dyn a4n_api_contract::image::ImageRecordMutationApplicationPort {
        self.image_record_mutation_application.as_ref()
    }

    pub(crate) fn image_record_cascade_mutation_application(
        &self,
    ) -> &dyn a4n_api_contract::image::ImageRecordCascadeMutationApplicationPort {
        self.image_record_cascade_mutation_application.as_ref()
    }

    pub(crate) fn image_import_preparation_application(
        &self,
    ) -> &dyn a4n_image_host_contract::ImageImportPreparationApplicationPort {
        self.image_import_preparation_application.as_ref()
    }

    pub fn image_import_publication_application(
        &self,
    ) -> &dyn a4n_image_host_contract::ImageImportPublicationApplicationPort {
        self.image_import_publication_application.as_ref()
    }

    pub(crate) fn artifact_media_publication_application(
        &self,
    ) -> Option<&dyn a4n_artifact_media_contract::ArtifactMediaPublicationPort> {
        self.artifact_media_publication_application.as_deref()
    }

    pub(crate) fn artifact_media_reference_application(
        &self,
    ) -> Option<&dyn a4n_artifact_media_contract::ArtifactMediaReferencePort> {
        self.artifact_media_reference_application.as_deref()
    }

    pub(crate) async fn persisted_media_path_prefix_has_reference(
        &self,
        prefix: &str,
    ) -> Result<bool, String> {
        if let Some(worker) = self.artifact_media_reference_application() {
            return worker
                .path_prefix_has_reference(a4n_artifact_media_contract::MediaPathPrefixRequest {
                    prefix: prefix.to_string(),
                })
                .await;
        }
        self.image_media_reference_application()
            .media_path_prefix_has_persisted_reference(prefix)
            .await
            .map_err(|error| error.to_string())
    }

    pub(crate) fn image_publication_recovery_application(
        &self,
    ) -> &dyn a4n_image_host_contract::ImagePublicationRecoveryPort {
        self.image_publication_recovery_application.as_ref()
    }

    pub fn image_publication_recovery_read_application(
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

    pub(crate) fn galgame_image_generation_application(
        &self,
    ) -> &dyn a4n_image_host_contract::GalgameImageGenerationApplicationPort {
        self.galgame_image_generation_application.as_ref()
    }

    pub(crate) fn image_canvas_request_preview_application(
        &self,
    ) -> &dyn a4n_image_host_contract::ImageCanvasRequestPreviewApplicationPort {
        self.image_canvas_request_preview_application.as_ref()
    }

    pub(crate) fn image_batch_request_preview_application(
        &self,
    ) -> &dyn a4n_image_host_contract::ImageBatchRequestPreviewApplicationPort {
        self.image_batch_request_preview_application.as_ref()
    }

    pub(crate) fn image_object_profile_request_preview_application(
        &self,
    ) -> &dyn a4n_image_host_contract::ImageObjectProfileRequestPreviewApplicationPort {
        self.image_object_profile_request_preview_application
            .as_ref()
    }

    pub(crate) fn image_page_request_preview_application(
        &self,
    ) -> &dyn a4n_image_host_contract::ImagePageRequestPreviewApplicationPort {
        self.image_page_request_preview_application.as_ref()
    }

    pub(crate) fn image_scene_request_preview_application(
        &self,
    ) -> &dyn a4n_image_host_contract::ImageSceneRequestPreviewApplicationPort {
        self.image_scene_request_preview_application.as_ref()
    }

    pub(crate) fn image_generation_task_application(
        &self,
    ) -> &dyn a4n_image_host_contract::ImageGenerationTaskApplicationPort {
        self.image_generation_task_application.as_ref()
    }

    pub(crate) fn image_profile_prompt_application(
        &self,
    ) -> &dyn a4n_api_contract::image_generation::ImageProfilePromptApplicationPort {
        self.image_profile_prompt_application.as_ref()
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
    ) -> &dyn a4n_image_host_contract::ProfileMaterialUploadApplicationPort {
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

    pub(crate) fn portrait_prompt_request_preview_application(
        &self,
    ) -> &dyn a4n_portrait_host_contract::PortraitPromptRequestPreviewApplicationPort {
        self.portrait_prompt_request_preview_application.as_ref()
    }

    pub(crate) fn portrait_image_request_preview_application(
        &self,
    ) -> &dyn a4n_portrait_host_contract::PortraitImageRequestPreviewApplicationPort {
        self.portrait_image_request_preview_application.as_ref()
    }

    pub(crate) fn portrait_prompt_application(
        &self,
    ) -> &dyn a4n_portrait_host_contract::PortraitPromptApplicationPort {
        self.portrait_prompt_application.as_ref()
    }

    pub(crate) fn portrait_upload_application(
        &self,
    ) -> &dyn a4n_portrait_host_contract::PortraitUploadApplicationPort {
        self.portrait_upload_application.as_ref()
    }

    pub(crate) fn portrait_generation_application(
        &self,
    ) -> &dyn a4n_portrait_host_contract::PortraitGenerationApplicationPort {
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
    ) -> &dyn a4n_galgame_host_contract::GalgameAssetGenerationApplicationPort {
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

    pub(crate) fn media_authoring_application(
        &self,
    ) -> &dyn a4n_api_contract::media_authoring::MediaAuthoringApplicationPort {
        self.media_authoring_application.as_ref()
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

    pub(crate) fn manga_pipeline_artifact_application(
        &self,
    ) -> &dyn a4n_api_contract::manga::MangaPipelineArtifactApplicationPort {
        self.manga_pipeline_artifact_application.as_ref()
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
    ) -> &dyn a4n_canvas_host_contract::CanvasLlmTextNodeApplicationPort {
        self.canvas_llm_text_application.as_ref()
    }

    pub(crate) fn canvas_image_application(
        &self,
    ) -> &dyn a4n_canvas_host_contract::CanvasImageNodeApplicationPort {
        self.canvas_image_application.as_ref()
    }

    pub(crate) fn canvas_video_application(
        &self,
    ) -> &dyn a4n_canvas_host_contract::CanvasVideoNodeApplicationPort {
        self.canvas_video_application.as_ref()
    }

    pub(crate) fn canvas_preview_application(
        &self,
    ) -> &dyn a4n_canvas_host_contract::CanvasPreviewApplicationPort {
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

    pub(crate) fn canvas_image_import_application(
        &self,
    ) -> &dyn a4n_canvas_contract::CanvasImageImportApplicationPort {
        self.canvas_image_import_application.as_ref()
    }

    pub(crate) fn canvas_video_import_application(
        &self,
    ) -> &dyn a4n_canvas_contract::CanvasVideoImportApplicationPort {
        self.canvas_video_import_application.as_ref()
    }

    pub(crate) fn canvas_audio_import_application(
        &self,
    ) -> &dyn a4n_canvas_contract::CanvasAudioImportApplicationPort {
        self.canvas_audio_import_application.as_ref()
    }

    pub(crate) fn canvas_graph_preparation_application(
        &self,
    ) -> &dyn a4n_canvas_contract::CanvasGraphPreparationApplicationPort {
        self.canvas_graph_preparation_application.as_ref()
    }

    pub(crate) fn canvas_graph_execution_application(
        &self,
    ) -> &dyn a4n_canvas_contract::CanvasGraphExecutionApplicationPort {
        self.canvas_graph_execution_application.as_ref()
    }

    pub(crate) fn canvas_graph_failure_application(
        &self,
    ) -> &dyn a4n_canvas_contract::CanvasGraphFailureApplicationPort {
        self.canvas_graph_failure_application.as_ref()
    }
}
