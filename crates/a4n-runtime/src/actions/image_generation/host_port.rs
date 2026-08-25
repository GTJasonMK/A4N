use a4n_core::{error::AppError, media::image_mime_from_path};
use a4n_image::ImageReference;
use a4n_orchestrator::generation::GeneratedMediaArtifactFile;
use a4n_orchestrator::media::{
    ImageGenerationHostPort, ImageGenerationProgress, ImageGenerationProviderInput,
    ImageGenerationProviderOutput, PersistGeneratedImageFileInput, PersistedGeneratedImageFile,
};
use a4n_queue::priority::{TaskKind as QueueTaskKind, TaskPriority};
use async_trait::async_trait;
use base64::Engine as _;

use super::{
    generation_io::persist_generated_image_file, run_queued_request_with_cancel_and_workflow,
};
use crate::{
    AppState,
    cleanup_outbox::enqueue_file_artifact_cleanup_by_project,
    file_utils::{canonicalize_data_file_path, inspect_generated_media_file},
};

pub(in crate::actions) struct RuntimeImageGenerationHostPort<'state> {
    state: &'state AppState,
}

impl<'state> RuntimeImageGenerationHostPort<'state> {
    pub(in crate::actions) fn new(state: &'state AppState) -> Self {
        Self { state }
    }
}

#[async_trait]
impl ImageGenerationHostPort for RuntimeImageGenerationHostPort<'_> {
    async fn resolve_image_config(
        &self,
        config_id: Option<i64>,
    ) -> Result<a4n_core::models::ai::image_gen_config::Model, AppError> {
        super::resolve_image_config(self.state, config_id)
            .await
            .map_err(host_error)
    }

    fn runtime_config_snapshot(&self) -> Option<serde_json::Value> {
        crate::runtime_config::redacted_current_snapshot_value()
    }

    fn read_reference_image(
        &self,
        label: &str,
        relative_path: &str,
    ) -> Result<ImageReference, AppError> {
        let path = canonicalize_data_file_path(
            self.state.data_dir(),
            relative_path,
            &format!("参考图 {label}"),
        )
        .map_err(host_error)?;
        let bytes = std::fs::read(&path)
            .map_err(|error| host_error(format!("读取参考图 {} 失败：{error}", path.display())))?;
        let mime = image_mime_from_path(&path).map_err(host_error)?;
        Ok(ImageReference {
            label: label.to_string(),
            data_uri: format!(
                "data:{mime};base64,{}",
                base64::engine::general_purpose::STANDARD.encode(bytes)
            ),
        })
    }

    async fn generate_image(
        &self,
        input: ImageGenerationProviderInput,
        progress: &ImageGenerationProgress<'_>,
    ) -> Result<ImageGenerationProviderOutput, AppError> {
        let root_project_id = crate::actions::sidecar_root_project_id(self.state, input.project_id)
            .await
            .map_err(host_error)?;
        let result = run_queued_request_with_cancel_and_workflow(
            self.state,
            QueueTaskKind::ImageGeneration,
            TaskPriority::Medium,
            Some(input.project_id),
            Some(input.workflow_run_id),
            progress.cancel_token(),
            || async move {
                let sidecar_progress = |message: &str| {
                    progress.progress(3, 4, message);
                };
                crate::sidecar_clients::generate_image(
                    self.state.runtime_profile(),
                    self.state.capabilities(),
                    &input.config,
                    input.request,
                    Some(input.project_id),
                    root_project_id,
                    Some(input.workflow_run_id),
                    Some(&sidecar_progress),
                )
                .await
            },
        )
        .await
        .map_err(host_error)?;
        Ok(ImageGenerationProviderOutput {
            bytes: result.bytes,
            format: result.format,
            model: result.model,
        })
    }

    fn persist_generated_image_file(
        &self,
        input: PersistGeneratedImageFileInput<'_>,
    ) -> Result<PersistedGeneratedImageFile, AppError> {
        let (_, relative_path, file_size) = persist_generated_image_file(
            self.state.data_dir(),
            input.project_id,
            input.chapter_id,
            input.image_type,
            input.file_extra,
            input.bytes,
            input.format,
            input.file_context,
        )
        .map_err(host_error)?;
        Ok(PersistedGeneratedImageFile {
            relative_path,
            file_size,
        })
    }

    fn validate_generated_image_file(
        &self,
        relative_path: &str,
        label: &str,
    ) -> Result<(), AppError> {
        canonicalize_data_file_path(self.state.data_dir(), relative_path, label)
            .map(|_| ())
            .map_err(host_error)
    }

    fn inspect_generated_image_file(
        &self,
        relative_path: &str,
        label: &str,
    ) -> Result<GeneratedMediaArtifactFile, AppError> {
        inspect_generated_media_file(
            self.state.data_dir(),
            relative_path,
            label,
            image_mime_from_path,
        )
        .map_err(host_error)
    }

    async fn remove_generated_image_file(
        &self,
        relative_path: &str,
        label: &str,
    ) -> Result<(), AppError> {
        enqueue_file_artifact_cleanup_by_project(
            self.state,
            &format!("generated-image-cleanup:{relative_path}"),
            &[relative_path.to_string()],
        )
        .await
        .map_err(|error| host_error(format!("{label}：{error}")))
    }
}

fn host_error(error: String) -> AppError {
    AppError::InvalidState {
        current: "image_generation_host".to_string(),
        operation: error,
    }
}
