use a4n_api_contract::{
    generation_tasks::GenerationTaskHandle,
    image::GeneratedImageSummary,
    image_generation::{
        GenerateBatchImagesInput, GenerateGalgameImageInput, GeneratePageImageInput,
        GenerateSceneImageInput, ImageProfilePromptApplicationPort, ObjectProfileImageInput,
        ObjectProfileImagePromptOutput, SceneProfileImageInput, SceneProfileImagePromptOutput,
        StartImageGenerationInput,
    },
};
use a4n_canvas_contract::GenerateCanvasImageInput;
use a4n_error::AppError;

use crate::{
    OrchestratorContext,
    media::{
        GalgameImageGenerationApplicationPort, ImageBatchRequestPreviewApplicationPort,
        ImageCanvasRequestPreviewApplicationPort, ImageGenerationHostPort, ImageGenerationProgress,
        ImageGenerationTaskApplicationPort, ImageObjectProfileRequestPreviewApplicationPort,
        ImagePageRequestPreviewApplicationPort, ImageSceneRequestPreviewApplicationPort,
        PreparedImageGenerationRequest,
    },
};

#[async_trait::async_trait]
impl GalgameImageGenerationApplicationPort for OrchestratorContext {
    async fn generate_galgame_image(
        &self,
        host_port: &dyn ImageGenerationHostPort,
        user_id: i64,
        input: GenerateGalgameImageInput,
        progress: &ImageGenerationProgress<'_>,
    ) -> Result<GeneratedImageSummary, AppError> {
        self.generate_galgame_image_validated(host_port, user_id, input, progress)
            .await
    }
}

#[async_trait::async_trait]
impl ImageObjectProfileRequestPreviewApplicationPort for OrchestratorContext {
    async fn prepare_object_profile_request(
        &self,
        host_port: &dyn ImageGenerationHostPort,
        user_id: i64,
        input: ObjectProfileImageInput,
    ) -> Result<PreparedImageGenerationRequest, AppError> {
        self.prepare_object_profile_image_request_validated(host_port, user_id, input)
            .await
    }
}

#[async_trait::async_trait]
impl ImageBatchRequestPreviewApplicationPort for OrchestratorContext {
    async fn prepare_batch_request(
        &self,
        host_port: &dyn ImageGenerationHostPort,
        user_id: i64,
        input: GenerateBatchImagesInput,
    ) -> Result<Vec<PreparedImageGenerationRequest>, AppError> {
        self.prepare_batch_images_request_validated(host_port, user_id, input)
            .await
    }
}

#[async_trait::async_trait]
impl ImagePageRequestPreviewApplicationPort for OrchestratorContext {
    async fn prepare_page_request(
        &self,
        host_port: &dyn ImageGenerationHostPort,
        user_id: i64,
        input: GeneratePageImageInput,
    ) -> Result<PreparedImageGenerationRequest, AppError> {
        self.prepare_page_image_request_validated(host_port, user_id, input)
            .await
    }
}

#[async_trait::async_trait]
impl ImageCanvasRequestPreviewApplicationPort for OrchestratorContext {
    async fn prepare_canvas_request(
        &self,
        host_port: &dyn ImageGenerationHostPort,
        user_id: i64,
        input: GenerateCanvasImageInput,
    ) -> Result<PreparedImageGenerationRequest, AppError> {
        self.prepare_canvas_image_request_validated(host_port, user_id, input)
            .await
    }
}

#[async_trait::async_trait]
impl ImageSceneRequestPreviewApplicationPort for OrchestratorContext {
    async fn prepare_scene_request(
        &self,
        host_port: &dyn ImageGenerationHostPort,
        user_id: i64,
        input: GenerateSceneImageInput,
    ) -> Result<PreparedImageGenerationRequest, AppError> {
        self.prepare_scene_image_request_validated(host_port, user_id, input)
            .await
    }
}

#[async_trait::async_trait]
impl ImageGenerationTaskApplicationPort for OrchestratorContext {
    async fn start_task(
        &self,
        user_id: i64,
        input: &StartImageGenerationInput,
    ) -> Result<
        crate::GenerationTaskStart,
        a4n_api_contract::generation_tasks::GenerationTaskStartError,
    > {
        OrchestratorContext::start_image_generation_task(self, user_id, input)
            .await
            .map_err(crate::GenerationTaskAccessError::into_start_error)
    }

    async fn run_task(
        &self,
        user_id: i64,
        input: StartImageGenerationInput,
        handle: GenerationTaskHandle,
        host_port: &dyn ImageGenerationHostPort,
    ) -> Result<(), String> {
        OrchestratorContext::run_image_generation_task(self, user_id, input, handle, host_port)
            .await
    }

    async fn generate_attached_page_image(
        &self,
        user_id: i64,
        input: GeneratePageImageInput,
        workflow_run_id: i64,
        host_port: &dyn ImageGenerationHostPort,
        progress: &ImageGenerationProgress<'_>,
    ) -> Result<GeneratedImageSummary, AppError> {
        self.generate_page_image_attached_validated(
            host_port,
            user_id,
            input,
            workflow_run_id,
            progress,
        )
        .await
    }
}

#[async_trait::async_trait]
impl ImageProfilePromptApplicationPort for OrchestratorContext {
    async fn object_profile_prompt(
        &self,
        user_id: i64,
        input: &ObjectProfileImageInput,
    ) -> Result<ObjectProfileImagePromptOutput, AppError> {
        self.generate_object_profile_image_prompt_validated(user_id, input)
            .await
    }

    async fn scene_profile_prompt(
        &self,
        user_id: i64,
        input: &SceneProfileImageInput,
    ) -> Result<SceneProfileImagePromptOutput, AppError> {
        self.generate_scene_profile_image_prompt_validated(user_id, input)
            .await
    }
}
