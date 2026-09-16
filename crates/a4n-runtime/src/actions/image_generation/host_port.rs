use a4n_domain::media::GeneratedMediaArtifactFile;
use a4n_domain::media::image_mime_from_path;
use a4n_error::AppError;
use a4n_image_contract::ImageReference;
use a4n_image_host_contract::{
    ImageGenerationHostPort, ImageGenerationProgress, ImageGenerationProviderInput,
    ImageGenerationProviderOutput, PersistGeneratedImageFileInput, PersistedGeneratedImageFile,
    PrepareGeneratedImageFileInput, PreparedGeneratedImageFile,
};
use a4n_provider_contract::ImageProviderConfig;
use a4n_queue::priority::{TaskKind as QueueTaskKind, TaskPriority};
use async_trait::async_trait;
use base64::Engine as _;

use super::{
    generation_io::{persist_reserved_generated_image_file, prepare_generated_image_file},
    run_queued_request_with_cancel_and_workflow,
};
use crate::{
    AppState, cleanup_outbox::enqueue_file_artifact_cleanup_by_project,
    file_utils::inspect_generated_media_file,
};
use a4n_storage::canonicalize_data_file_path;

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
    ) -> Result<ImageProviderConfig, AppError> {
        super::resolve_image_provider_config(self.state, config_id)
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
                    self.state.capabilities().image.clone(),
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

    fn prepare_generated_image_file(
        &self,
        input: PrepareGeneratedImageFileInput<'_>,
    ) -> Result<PreparedGeneratedImageFile, AppError> {
        prepare_generated_image_file(self.state.data_dir(), input).map_err(host_error)
    }

    fn persist_generated_image_file(
        &self,
        input: PersistGeneratedImageFileInput<'_>,
        prepared: &PreparedGeneratedImageFile,
    ) -> Result<PersistedGeneratedImageFile, AppError> {
        persist_reserved_generated_image_file(self.state.data_dir(), input, prepared)
            .map_err(host_error)
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
    let message = error
        .strip_prefix("内部错误：")
        .unwrap_or(&error)
        .to_string();
    AppError::Internal(anyhow::anyhow!(message))
}
