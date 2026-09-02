use std::{io::Write, path::Path};

use a4n_core::{
    error::AppError,
    files::{create_unique_timestamped_file, paths::ProjectPaths},
    media::image_mime_from_path,
    models::ai::image_gen_config,
};
use a4n_domain::EXTERNAL_IMPORT_MODEL;
use a4n_image::{ImageGenerationRequest, ImageReference};
use a4n_orchestrator::{
    generation::GeneratedMediaArtifactFile,
    portrait::{
        DecodedPortraitImage, GeneratedPortraitImage, PersistPortraitFileInput,
        PersistedPortraitFile, PortraitGenerationHostPort, PortraitImageProviderInput,
    },
};
use a4n_queue::priority::{TaskKind as QueueTaskKind, TaskPriority};
use async_trait::async_trait;
use base64::Engine as _;
use tokio_util::sync::CancellationToken;

use crate::{
    AppState,
    actions::image_generation::resolve_image_config,
    cleanup_outbox::enqueue_file_artifact_cleanup_by_project,
    file_utils::{
        canonicalize_data_file_path, inspect_generated_media_file, persist_created_media_file,
    },
};

pub(super) struct RuntimePortraitGenerationHostPort<'a> {
    state: &'a AppState,
}

pub(super) struct PortraitImageRequestPreviewContext {
    pub(super) request: ImageGenerationRequest,
    pub(super) config: image_gen_config::Model,
}

impl<'a> RuntimePortraitGenerationHostPort<'a> {
    pub(super) fn new(state: &'a AppState) -> Self {
        Self { state }
    }
}

pub(super) async fn build_portrait_image_request_preview(
    state: &AppState,
    project_id: i64,
    prompt: &str,
    width: i32,
    height: i32,
    reference_image_ids: &[i64],
) -> Result<PortraitImageRequestPreviewContext, String> {
    let config = resolve_image_config(state, None).await?;
    build_portrait_image_request_preview_with_config(
        state,
        project_id,
        prompt,
        width,
        height,
        reference_image_ids,
        &config,
    )
    .await
}

pub(super) async fn build_portrait_image_request_preview_with_config(
    state: &AppState,
    project_id: i64,
    prompt: &str,
    width: i32,
    height: i32,
    reference_image_ids: &[i64],
    config: &image_gen_config::Model,
) -> Result<PortraitImageRequestPreviewContext, String> {
    let image_ids = a4n_orchestrator::portrait::validate_reference_image_ids(reference_image_ids)?;
    let supported_sizes = a4n_core::config::image_sizes_from_json(&config.supported_sizes_json)
        .map_err(|error| error.to_string())?;
    a4n_core::config::ensure_image_size_supported(&supported_sizes, width, height)
        .map_err(|error| error.to_string())?;
    let records = state
        .orchestrator()
        .portrait_reference_images_by_ids(project_id, &image_ids)
        .await
        .map_err(|error| error.to_string())?;
    let sources = records
        .into_iter()
        .enumerate()
        .map(
            |(index, image)| a4n_orchestrator::portrait::PortraitReferenceImageSource {
                label: format!("参考图 {}", index + 1),
                file_path: image.file_path,
            },
        )
        .collect::<Vec<_>>();
    let reference_images = sources
        .iter()
        .map(|source| {
            read_reference_image(state, &source.label, &source.file_path)
                .map_err(|error| error.to_string())
        })
        .collect::<Result<Vec<_>, _>>()?;
    if !reference_images.is_empty()
        && !a4n_core::config::image_backend_supports_reference_images(&config.backend)
    {
        return Err(format!(
            "当前图片后端 {} 不支持真实参考图输入，请切换到 openai-compatible-image-edit、openai-chat-image 或 cliproxy-image，或移除参考图",
            config.backend
        ));
    }
    Ok(PortraitImageRequestPreviewContext {
        request: ImageGenerationRequest {
            prompt: prompt.to_string(),
            width,
            height,
            model: config.model.clone(),
            reference_images,
        },
        config: config.clone(),
    })
}

#[async_trait]
impl PortraitGenerationHostPort for RuntimePortraitGenerationHostPort<'_> {
    async fn generate_image(
        &self,
        input: PortraitImageProviderInput,
        cancel_token: CancellationToken,
    ) -> Result<GeneratedPortraitImage, AppError> {
        let PortraitImageProviderInput {
            project_id,
            prompt,
            width,
            height,
            reference_images,
            workflow_run_id,
        } = input;
        let config = resolve_image_config(self.state, None)
            .await
            .map_err(host_error)?;
        let supported_sizes =
            a4n_core::config::image_sizes_from_json(&config.supported_sizes_json)?;
        a4n_core::config::ensure_image_size_supported(&supported_sizes, width, height)?;
        let reference_images = reference_images
            .iter()
            .map(|source| read_reference_image(self.state, &source.label, &source.file_path))
            .collect::<Result<Vec<_>, _>>()?;
        if !reference_images.is_empty()
            && !a4n_core::config::image_backend_supports_reference_images(&config.backend)
        {
            return Err(AppError::InvalidArgument(format!(
                "当前图片后端 {} 不支持真实参考图输入，请切换到 openai-compatible-image-edit、openai-chat-image 或 cliproxy-image，或移除参考图",
                config.backend
            )));
        }
        let request = ImageGenerationRequest {
            prompt,
            width,
            height,
            model: config.model.clone(),
            reference_images,
        };
        let provider_backend = config.backend.clone();
        let request_json = serde_json::to_value(&request)?;
        let root_project_id = crate::actions::sidecar_root_project_id(self.state, project_id)
            .await
            .map_err(host_error)?;
        let result = crate::actions::image_generation::run_queued_request_with_cancel_and_workflow(
            self.state,
            QueueTaskKind::ImageGeneration,
            TaskPriority::Medium,
            Some(project_id),
            workflow_run_id,
            cancel_token,
            || async move {
                crate::sidecar_clients::generate_image(
                    self.state.runtime_profile(),
                    self.state.capabilities(),
                    &config,
                    request,
                    Some(project_id),
                    root_project_id,
                    workflow_run_id,
                    None,
                )
                .await
            },
        )
        .await
        .map_err(host_error)?;
        let (width, height) = a4n_orchestrator::media::image_dimensions_from_bytes(&result.bytes)?;
        Ok(GeneratedPortraitImage {
            bytes: result.bytes,
            format: result.format,
            model: result.model,
            provider: provider_backend,
            request_json,
            width,
            height,
        })
    }

    fn decode_uploaded_image(
        &self,
        file_name: &str,
        mime_type: &str,
        data_base64: &str,
    ) -> Result<DecodedPortraitImage, AppError> {
        let decoded = a4n_orchestrator::media::decode_uploaded_image_data(
            file_name,
            mime_type,
            data_base64,
            50 * 1024 * 1024,
        )
        .map_err(host_error)?;
        Ok(DecodedPortraitImage {
            bytes: decoded.bytes,
            format: decoded.extension.to_string(),
            model: EXTERNAL_IMPORT_MODEL.to_string(),
            width: decoded.width,
            height: decoded.height,
        })
    }

    fn persist_portrait_file(
        &self,
        input: PersistPortraitFileInput<'_>,
    ) -> Result<PersistedPortraitFile, AppError> {
        persist_portrait_file_at(self.state.data_dir(), input).map(|(_, persisted)| persisted)
    }

    fn inspect_portrait_file(
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

    fn validate_portrait_file(&self, relative_path: &str, label: &str) -> Result<(), AppError> {
        canonicalize_data_file_path(self.state.data_dir(), relative_path, label)
            .map(|_| ())
            .map_err(host_error)
    }

    async fn remove_portrait_file(&self, relative_path: &str, label: &str) -> Result<(), AppError> {
        enqueue_file_artifact_cleanup_by_project(
            self.state,
            &format!("portrait-cleanup:{relative_path}"),
            &[relative_path.to_string()],
        )
        .await
        .map_err(|error| host_error(format!("{label}：{error}")))
    }
}

pub(super) fn persist_portrait_file_at(
    data_dir: &Path,
    input: PersistPortraitFileInput<'_>,
) -> Result<(std::path::PathBuf, PersistedPortraitFile), AppError> {
    let (path, mut output_file) =
        create_portrait_output(data_dir, input.project_id, input.profile_id, input.format)?;
    let (relative_path, file_size) = persist_created_media_file(
        data_dir,
        &path,
        move |_| {
            output_file
                .write_all(input.bytes)
                .map_err(|error| format!("无法写入立绘：{error}"))
        },
        input.path_context,
        input.path_context,
        input.path_context,
    )
    .map_err(host_error)?;
    Ok((
        path,
        PersistedPortraitFile {
            relative_path,
            file_size,
        },
    ))
}

fn read_reference_image(
    state: &AppState,
    label: &str,
    relative_path: &str,
) -> Result<ImageReference, AppError> {
    let path =
        canonicalize_data_file_path(state.data_dir(), relative_path, label).map_err(host_error)?;
    let bytes = std::fs::read(&path)?;
    let mime = image_mime_from_path(&path).map_err(AppError::InvalidArgument)?;
    let encoded = base64::engine::general_purpose::STANDARD.encode(bytes);
    Ok(ImageReference {
        label: label.to_string(),
        data_uri: format!("data:{mime};base64,{encoded}"),
    })
}

fn create_portrait_output(
    data_dir: &Path,
    project_id: i64,
    profile_id: i64,
    format: &str,
) -> Result<(std::path::PathBuf, std::fs::File), AppError> {
    let directory = ProjectPaths::new(data_dir, project_id).portraits_dir();
    std::fs::create_dir_all(&directory)?;
    create_unique_timestamped_file(&directory, &format!("portrait_{profile_id}"), format)
        .map_err(AppError::Io)
}

fn host_error(message: String) -> AppError {
    AppError::InvalidArgument(message)
}
