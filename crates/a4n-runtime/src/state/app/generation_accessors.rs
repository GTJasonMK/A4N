use super::*;

impl AppState {
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

    pub(crate) fn canvas_video_request_preview_application(
        &self,
    ) -> &dyn a4n_api_contract::video::CanvasVideoRequestPreviewApplicationPort {
        self.canvas_video_request_preview_application.as_ref()
    }

    pub(crate) fn chapter_video_request_preview_application(
        &self,
    ) -> &dyn a4n_api_contract::video::ChapterVideoRequestPreviewApplicationPort {
        self.chapter_video_request_preview_application.as_ref()
    }

    pub(crate) fn reference_media_video_request_preview_application(
        &self,
    ) -> &dyn a4n_api_contract::video::ReferenceMediaVideoRequestPreviewApplicationPort {
        self.reference_media_video_request_preview_application
            .as_ref()
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
    ) -> &dyn a4n_chapter_generation_contract::ChapterGenerationApplicationPort {
        self.chapter_generation_application.as_ref()
    }

    pub(crate) fn chapter_generation_request_preview_application(
        &self,
    ) -> &dyn a4n_chapter_generation_contract::ChapterGenerationRequestPreviewApplicationPort {
        self.chapter_generation_request_preview_application.as_ref()
    }

    pub(crate) fn chapter_generation_rag_preparation_application(
        &self,
    ) -> &dyn a4n_chapter_generation_contract::ChapterGenerationRagPreparationApplicationPort {
        self.chapter_generation_rag_preparation_application.as_ref()
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
    ) -> &dyn a4n_outline_contract::OutlineGenerationApplicationPort {
        self.outline_generation_application.as_ref()
    }

    pub(crate) fn outline_part_generation_preview_application(
        &self,
    ) -> &dyn a4n_outline_contract::OutlinePartGenerationPreviewApplicationPort {
        self.outline_part_generation_preview_application.as_ref()
    }

    pub(crate) fn outline_part_mutation_preview_application(
        &self,
    ) -> &dyn a4n_outline_contract::OutlinePartMutationPreviewApplicationPort {
        self.outline_part_mutation_preview_application.as_ref()
    }

    pub(crate) fn outline_part_chapter_generation_preview_application(
        &self,
    ) -> &dyn a4n_outline_contract::OutlinePartChapterGenerationPreviewApplicationPort {
        self.outline_part_chapter_generation_preview_application
            .as_ref()
    }

    pub(crate) fn outline_batch_part_generation_preview_application(
        &self,
    ) -> &dyn a4n_outline_contract::OutlineBatchPartGenerationPreviewApplicationPort {
        self.outline_batch_part_generation_preview_application
            .as_ref()
    }

    pub(crate) fn outline_chapter_count_generation_preview_application(
        &self,
    ) -> &dyn a4n_outline_contract::OutlineChapterCountGenerationPreviewApplicationPort {
        self.outline_chapter_count_generation_preview_application
            .as_ref()
    }

    pub(crate) fn outline_chapter_regeneration_preview_application(
        &self,
    ) -> &dyn a4n_outline_contract::OutlineChapterRegenerationPreviewApplicationPort {
        self.outline_chapter_regeneration_preview_application
            .as_ref()
    }

    pub(crate) fn outline_beat_mapping_preview_application(
        &self,
    ) -> &dyn a4n_outline_contract::OutlineBeatMappingPreviewApplicationPort {
        self.outline_beat_mapping_preview_application.as_ref()
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

    pub(crate) fn settings_export_application(
        &self,
    ) -> &dyn a4n_api_contract::config::SettingsExportApplicationPort {
        self.settings_export_application.as_ref()
    }

    pub(crate) fn settings_import_application(
        &self,
    ) -> &dyn a4n_api_contract::config::SettingsImportApplicationPort {
        self.settings_import_application.as_ref()
    }

    pub(crate) fn prompt_read_application(
        &self,
    ) -> &dyn a4n_api_contract::prompt::PromptReadApplicationPort {
        self.prompt_read_application.as_ref()
    }

    pub(crate) fn prompt_mutation_application(
        &self,
    ) -> &dyn a4n_api_contract::prompt::PromptMutationApplicationPort {
        self.prompt_mutation_application.as_ref()
    }
}
