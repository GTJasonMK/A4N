use a4n_api_contract::{image::MAX_UPLOAD_IMAGE_BYTES, portrait::PortraitReferenceImageSource};
use a4n_config::{ensure_image_size_supported, image_sizes_from_json};
use a4n_domain::EXTERNAL_IMPORT_MODEL;
use a4n_domain::cost::{
    ModelCapability, ModelUsageMeter, ModelUsageRecordCompletion, ModelUsageRecordStart,
};
use a4n_domain::image::validate_reference_image_ids;
use a4n_domain::media::{GeneratedMediaArtifactFile, image_mime_from_path};
use a4n_error::AppError;
use a4n_image_contract::ImageGenerationRequest;
use a4n_image_host_contract::{
    PersistGeneratedImageFileInput, PersistedGeneratedImageFile, PrepareGeneratedImageFileInput,
    PreparedGeneratedImageFile,
};
use a4n_media::{decode_uploaded_image_data, image_dimensions_from_bytes};
use a4n_portrait_host_contract::{
    DecodedPortraitImage, GeneratedPortraitImage, PortraitGenerationHostPort,
    PortraitImageProviderInput,
};
use a4n_provider_contract::ImageProviderConfig;
use a4n_queue::priority::{TaskKind as QueueTaskKind, TaskPriority};
use async_trait::async_trait;
use tokio_util::sync::CancellationToken;

use crate::{
    AppState,
    actions::image_generation::{
        persist_reserved_generated_image_file, prepare_generated_image_file, read_reference_image,
        resolve_image_config,
    },
    cleanup_outbox::enqueue_file_artifact_cleanup_by_project,
    file_utils::inspect_generated_media_file,
};
use a4n_storage::canonicalize_data_file_path;

pub(super) struct RuntimePortraitGenerationHostPort<'a> {
    state: &'a AppState,
}

pub(super) struct PortraitImageRequestPreviewContext {
    pub(super) request: ImageGenerationRequest,
    pub(super) config: ImageProviderConfig,
}

pub(super) struct PortraitImageRequestPreviewInput<'a> {
    pub(super) user_id: i64,
    pub(super) project_id: i64,
    pub(super) prompt: &'a str,
    pub(super) width: i32,
    pub(super) height: i32,
    pub(super) reference_image_ids: &'a [i64],
    pub(super) config: &'a ImageProviderConfig,
}

impl<'a> RuntimePortraitGenerationHostPort<'a> {
    pub(super) fn new(state: &'a AppState) -> Self {
        Self { state }
    }
}

pub(super) async fn build_portrait_image_request_preview(
    state: &AppState,
    user_id: i64,
    project_id: i64,
    prompt: &str,
    width: i32,
    height: i32,
    reference_image_ids: &[i64],
) -> Result<PortraitImageRequestPreviewContext, String> {
    let config = resolve_image_config(state, None).await?;
    build_portrait_image_request_preview_with_config(
        state,
        PortraitImageRequestPreviewInput {
            user_id,
            project_id,
            prompt,
            width,
            height,
            reference_image_ids,
            config: &config,
        },
    )
    .await
}

pub(super) async fn build_portrait_image_request_preview_with_config(
    state: &AppState,
    input: PortraitImageRequestPreviewInput<'_>,
) -> Result<PortraitImageRequestPreviewContext, String> {
    let PortraitImageRequestPreviewInput {
        user_id,
        project_id,
        prompt,
        width,
        height,
        reference_image_ids,
        config,
    } = input;
    let image_ids = validate_reference_image_ids(reference_image_ids)?;
    let supported_sizes =
        image_sizes_from_json(&config.supported_sizes_json).map_err(|error| error.to_string())?;
    ensure_image_size_supported(&supported_sizes, width, height)
        .map_err(|error| error.to_string())?;
    let records = state
        .image_reference_read_application()
        .reference_by_ids(user_id, project_id, &image_ids)
        .await
        .map_err(|error| error.to_string())?;
    let sources = records
        .into_iter()
        .enumerate()
        .map(|(index, image)| PortraitReferenceImageSource {
            label: format!("参考图 {}", index + 1),
            file_path: image.file_path,
        })
        .collect::<Vec<_>>();
    let reference_images = sources
        .iter()
        .map(|source| {
            read_reference_image(state.data_dir(), &source.label, &source.file_path)
                .map_err(|error| error.to_string())
        })
        .collect::<Result<Vec<_>, _>>()?;
    if !reference_images.is_empty()
        && !a4n_domain::image::image_backend_supports_reference_images(&config.backend)
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
        let supported_sizes = image_sizes_from_json(&config.supported_sizes_json)?;
        ensure_image_size_supported(&supported_sizes, width, height)?;
        let reference_images = reference_images
            .iter()
            .map(|source| {
                read_reference_image(self.state.data_dir(), &source.label, &source.file_path)
            })
            .collect::<Result<Vec<_>, _>>()?;
        if !reference_images.is_empty()
            && !a4n_domain::image::image_backend_supports_reference_images(&config.backend)
        {
            return Err(AppError::InvalidArgument(format!(
                "当前图片后端 {} 不支持真实参考图输入，请切换到 openai-compatible-image-edit、openai-chat-image 或 cliproxy-image，或移除参考图",
                config.backend
            )));
        }
        let mut request = ImageGenerationRequest {
            prompt,
            width,
            height,
            model: config.model.clone(),
            reference_images,
        };
        crate::actions::image_generation::assemble_image_generation_request(
            self.state,
            project_id,
            &mut request,
        )
        .await
        .map_err(host_error)?;
        let provider_backend = config.backend.clone();
        let request_json = serde_json::to_value(&request)?;
        let request_body_snapshot =
            match crate::actions::image_generation::image_generation_preview_step(
                crate::actions::image_generation::ImageGenerationPreviewInput {
                    runtime_profile: self.state.runtime_profile(),
                    client: self.state.capabilities().image.clone(),
                    collaboration: self.state.project_access_application(),
                    project_id,
                    key: "portrait_generation".to_string(),
                    title: "Portrait image provider request".to_string(),
                    prompt_name: Some("image_generation_request".to_string()),
                    note: None,
                    config: &config,
                    request: request.clone(),
                },
            )
            .await
            {
                Ok(preview) => preview.request,
                Err(error) => {
                    tracing::warn!(backend = %config.backend, %error, "Could not capture portrait image request snapshot");
                    None
                }
            };
        let root_project_id = crate::actions::sidecar_root_project_id(self.state, project_id)
            .await
            .map_err(host_error)?;
        let cost_record = self
            .state
            .provider_runtime()
            .start_model_usage(ModelUsageRecordStart {
                request_id: uuid::Uuid::new_v4().to_string(),
                user_id: None,
                project_id: Some(project_id),
                root_project_id,
                workflow_run_id,
                generation_run_id: None,
                workflow: "portrait_generation".to_string(),
                step_key: "provider_submit".to_string(),
                call_key: None,
                source: Some("portrait_generation".to_string()),
                capability: ModelCapability::Image,
                provider_config_id: Some(config.id),
                provider: Some(config.backend.clone()),
                model: Some(config.model.clone()),
                pricing: config.pricing.clone(),
                request_json: request_body_snapshot,
            })
            .await
            .map_err(|error| host_error(error.to_string()))?;
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
                    self.state.capabilities().image.clone(),
                    &config,
                    request,
                    crate::sidecar_clients::SidecarWorkflowScope {
                        project_id: Some(project_id),
                        root_project_id,
                        workflow_run_id,
                    },
                    None,
                )
                .await
            },
        )
        .await
        .map_err(host_error);
        let status = if result.is_ok() {
            "completed"
        } else {
            "failed"
        };
        self.state
            .provider_runtime()
            .finish_model_usage(
                cost_record,
                ModelUsageRecordCompletion {
                    status: status.to_string(),
                    meter: ModelUsageMeter {
                        image_count: Some(1),
                        ..Default::default()
                    },
                    error_kind: result.as_ref().err().map(|_| "provider".to_string()),
                    error_message: result.as_ref().err().map(ToString::to_string),
                },
            )
            .await
            .map_err(|error| host_error(error.to_string()))?;
        let result = result?;
        let (width, height) =
            image_dimensions_from_bytes(&result.bytes).map_err(|error| match error {
                a4n_media::ImageDimensionsError::Format(source) => AppError::InvalidState {
                    current: "provider_response".to_string(),
                    operation: format!("图片 provider 返回的媒体格式无法识别：{source}"),
                },
                a4n_media::ImageDimensionsError::Dimensions(source) => AppError::InvalidState {
                    current: "provider_response".to_string(),
                    operation: format!("图片 provider 返回的媒体尺寸无法读取：{source}"),
                },
                a4n_media::ImageDimensionsError::WidthOverflow
                | a4n_media::ImageDimensionsError::HeightOverflow => {
                    AppError::InvalidArgument(error.to_string())
                }
            })?;
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
        let decoded =
            decode_uploaded_image_data(file_name, mime_type, data_base64, MAX_UPLOAD_IMAGE_BYTES)
                .map_err(host_error)?;
        Ok(DecodedPortraitImage {
            bytes: decoded.bytes,
            format: decoded.extension.to_string(),
            model: EXTERNAL_IMPORT_MODEL.to_string(),
            width: decoded.width,
            height: decoded.height,
        })
    }

    fn prepare_portrait_file(
        &self,
        input: PrepareGeneratedImageFileInput<'_>,
    ) -> Result<PreparedGeneratedImageFile, AppError> {
        prepare_generated_image_file(self.state.data_dir(), input).map_err(host_error)
    }

    fn persist_portrait_file(
        &self,
        input: PersistGeneratedImageFileInput<'_>,
        prepared: &PreparedGeneratedImageFile,
    ) -> Result<PersistedGeneratedImageFile, AppError> {
        persist_reserved_generated_image_file(self.state.data_dir(), input, prepared)
            .map_err(host_error)
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

fn host_error(message: String) -> AppError {
    AppError::InvalidArgument(message)
}
