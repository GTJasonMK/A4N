//! 媒体资产 runtime 窄 facade。

use a4n_api_contract::{
    generation_tasks::GenerationTaskHandle, image_generation::StartImageGenerationInput,
};
use a4n_core::{error::AppError, models::ai::generated_image};
use a4n_orchestrator::{
    GenerationTaskAccessError, GenerationTaskStart, OrchestratorContext,
    media::{
        BatchImageGenerationInput, GenerateCanvasImageInput, GenerateGalgameImageInput,
        GeneratedImageSummary, ImageGenerationHostPort, ImageGenerationProgress,
        ImportedImageUploadInput, ObjectProfileImageInput, ObjectProfileImagePromptOutput,
        PageImageGenerationInput, PreparedImageGenerationRequest, PreparedImportedImageUpload,
        SceneImageGenerationInput, SceneProfileImageInput, SceneProfileImagePromptOutput,
    },
    video::UpdateGeneratedImageRecordInput,
};

pub(crate) struct MediaRuntimeFacade<'a> {
    orchestrator: &'a OrchestratorContext,
}

impl<'a> MediaRuntimeFacade<'a> {
    pub(super) fn new(orchestrator: &'a OrchestratorContext) -> Self {
        Self { orchestrator }
    }

    pub(crate) async fn start_image_generation_task(
        &self,
        user_id: i64,
        input: StartImageGenerationInput,
    ) -> Result<GenerationTaskStart, GenerationTaskAccessError> {
        self.orchestrator
            .start_image_generation_task(user_id, input)
            .await
    }

    pub(crate) async fn run_image_generation_task(
        &self,
        user_id: i64,
        input: StartImageGenerationInput,
        handle: GenerationTaskHandle,
        host_port: &dyn ImageGenerationHostPort,
    ) -> Result<(), String> {
        self.orchestrator
            .run_image_generation_task(user_id, input, handle, host_port)
            .await
    }

    pub(crate) async fn list_generated_images_for_user_validated(
        &self,
        user_id: i64,
        project_id: i64,
    ) -> Result<Vec<generated_image::Model>, AppError> {
        self.orchestrator
            .list_generated_images_for_user_validated(user_id, project_id)
            .await
    }

    pub(crate) async fn list_chapter_images_for_user_validated(
        &self,
        user_id: i64,
        project_id: i64,
        chapter_id: i64,
    ) -> Result<Vec<generated_image::Model>, AppError> {
        self.orchestrator
            .list_chapter_images_for_user_validated(user_id, project_id, chapter_id)
            .await
    }

    pub(crate) async fn list_video_reference_images_for_user_validated(
        &self,
        user_id: i64,
        project_id: i64,
        chapter_id: i64,
        requested_project_ids: Vec<i64>,
    ) -> Result<Vec<generated_image::Model>, AppError> {
        self.orchestrator
            .list_video_reference_images_for_user_validated(
                user_id,
                project_id,
                chapter_id,
                requested_project_ids,
            )
            .await
    }

    pub(crate) async fn prepare_imported_image_upload_for_user_validated(
        &self,
        user_id: i64,
        project_id: i64,
        input: ImportedImageUploadInput,
    ) -> Result<PreparedImportedImageUpload, AppError> {
        self.orchestrator
            .prepare_imported_image_upload_for_user_validated(user_id, project_id, input)
            .await
    }

    pub(crate) async fn create_imported_image_record_for_user_validated(
        &self,
        user_id: i64,
        prepared: PreparedImportedImageUpload,
        file_path: String,
        file_size: i64,
    ) -> Result<generated_image::Model, AppError> {
        self.orchestrator
            .create_imported_image_record_for_user_validated(
                user_id, prepared, file_path, file_size,
            )
            .await
    }

    pub(crate) async fn update_generated_image_for_user_validated(
        &self,
        user_id: i64,
        image_id: i64,
        input: UpdateGeneratedImageRecordInput,
    ) -> Result<generated_image::Model, AppError> {
        self.orchestrator
            .update_generated_image_for_user_validated(user_id, image_id, input)
            .await
    }

    pub(crate) async fn toggle_generated_image_favorite_for_user_validated(
        &self,
        user_id: i64,
        image_id: i64,
    ) -> Result<generated_image::Model, AppError> {
        self.orchestrator
            .toggle_generated_image_favorite_for_user_validated(user_id, image_id)
            .await
    }

    pub(crate) async fn delete_generated_image_for_user_validated(
        &self,
        user_id: i64,
        image_id: i64,
    ) -> Result<(), AppError> {
        self.orchestrator
            .delete_generated_image_for_user_validated(user_id, image_id)
            .await
    }

    pub(crate) async fn prepare_scene_image_request_validated(
        &self,
        host: &dyn ImageGenerationHostPort,
        user_id: i64,
        input: SceneImageGenerationInput,
    ) -> Result<PreparedImageGenerationRequest, AppError> {
        self.orchestrator
            .prepare_scene_image_request_validated(host, user_id, input)
            .await
    }

    pub(crate) async fn prepare_canvas_image_request_validated(
        &self,
        host: &dyn ImageGenerationHostPort,
        user_id: i64,
        input: GenerateCanvasImageInput,
    ) -> Result<PreparedImageGenerationRequest, AppError> {
        self.orchestrator
            .prepare_canvas_image_request_validated(host, user_id, input)
            .await
    }

    pub(crate) async fn generate_galgame_image_validated(
        &self,
        host: &dyn ImageGenerationHostPort,
        user_id: i64,
        input: GenerateGalgameImageInput,
        progress: &ImageGenerationProgress<'_>,
    ) -> Result<GeneratedImageSummary, AppError> {
        self.orchestrator
            .generate_galgame_image_validated(host, user_id, input, progress)
            .await
    }

    pub(crate) async fn generate_object_profile_image_prompt_validated(
        &self,
        user_id: i64,
        input: &ObjectProfileImageInput,
    ) -> Result<ObjectProfileImagePromptOutput, AppError> {
        self.orchestrator
            .generate_object_profile_image_prompt_validated(user_id, input)
            .await
    }

    pub(crate) async fn generate_scene_profile_image_prompt_validated(
        &self,
        user_id: i64,
        input: &SceneProfileImageInput,
    ) -> Result<SceneProfileImagePromptOutput, AppError> {
        self.orchestrator
            .generate_scene_profile_image_prompt_validated(user_id, input)
            .await
    }

    pub(crate) async fn prepare_object_profile_image_request_validated(
        &self,
        host: &dyn ImageGenerationHostPort,
        user_id: i64,
        input: ObjectProfileImageInput,
    ) -> Result<PreparedImageGenerationRequest, AppError> {
        self.orchestrator
            .prepare_object_profile_image_request_validated(host, user_id, input)
            .await
    }

    pub(crate) async fn prepare_page_image_request_validated(
        &self,
        host: &dyn ImageGenerationHostPort,
        user_id: i64,
        input: PageImageGenerationInput,
    ) -> Result<PreparedImageGenerationRequest, AppError> {
        self.orchestrator
            .prepare_page_image_request_validated(host, user_id, input)
            .await
    }

    pub(crate) async fn prepare_batch_images_request_validated(
        &self,
        host: &dyn ImageGenerationHostPort,
        user_id: i64,
        input: BatchImageGenerationInput,
    ) -> Result<Vec<PreparedImageGenerationRequest>, AppError> {
        self.orchestrator
            .prepare_batch_images_request_validated(host, user_id, input)
            .await
    }

    pub(crate) async fn delete_generated_image_validated(
        &self,
        image: a4n_core::models::ai::generated_image::Model,
    ) -> Result<(), AppError> {
        self.orchestrator
            .delete_generated_image_validated(image)
            .await
    }
}
