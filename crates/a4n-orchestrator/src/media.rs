//! 媒体资产应用编排：文件边界校验、文件事实计算与 provenance 原子落库。

mod batch_image_application;
mod batch_image_rules;
mod contracts;
mod image_generation;
mod image_record_lifecycle;
mod image_workflow_rules;
mod rules;
mod uploaded_image_rules;

pub use batch_image_application::{
    BatchImageExecutionItem, BatchImageGenerationApplicationInput, BatchImageGenerationItem,
    BatchImageGenerationOutcome,
};
pub use contracts::{CreateMediaAssetInput, CreateMediaProvenanceInput};
pub use image_generation::{
    BatchImageGenerationInput, GenerateCanvasImageInput, GenerateGalgameImageInput,
    GeneratedImageArtifactFile, GeneratedImageSummary, ImageGenerationExecutionInput,
    ImageGenerationExecutionOutcome, ImageGenerationHostPort, ImageGenerationPostCommitAction,
    ImageGenerationProgress, ImageGenerationProviderInput, ImageGenerationProviderOutput,
    ImageGenerationWorkflowBinding, ObjectProfileImageInput, ObjectProfileImagePromptOutput,
    PageImageGenerationInput, PersistGeneratedImageFileInput, PersistedGeneratedImageFile,
    PreparedImageGenerationRequest, SceneImageGenerationInput, SceneProfileImageInput,
    SceneProfileImagePromptOutput, generated_image_summary, image_generation_request_trace_json,
    normalize_image_metadata,
};
pub use image_record_lifecycle::{
    IMPORTED_IMAGE_MODEL, ImportedImageUploadInput, PreparedImportedImageUpload,
};
pub use image_workflow_rules::{
    batch_image_item_step_key, batch_image_workflow_business_key, batch_image_workflow_fingerprint,
    batch_image_workflow_run_input, image_generation_step_key,
    image_generation_workflow_business_key, image_generation_workflow_fingerprint,
    image_generation_workflow_run_input,
};
pub use uploaded_image_rules::{
    DecodedUploadedImage, decode_uploaded_image_data, strip_data_url_prefix,
};

pub use rules::{
    MEDIA_BINDING_ROLE_BACKGROUND, MEDIA_BINDING_ROLE_BGM, MEDIA_BINDING_ROLE_CG,
    MEDIA_BINDING_ROLE_SFX, MEDIA_BINDING_ROLES, MEDIA_BINDING_TARGET_GALGAME_LINE,
    MEDIA_BINDING_TARGET_GALGAME_SCENE, MEDIA_BINDING_TARGET_KINDS, MEDIA_KIND_AUDIO,
    MEDIA_KIND_IMAGE, MEDIA_KIND_VIDEO, MEDIA_ORIGIN_ASSEMBLED_PREVIEW,
    MEDIA_ORIGIN_EXTRACTED_FRAME, MEDIA_ORIGIN_GENERATED, MEDIA_ORIGIN_IMPORTED,
    MEDIA_STATUS_CANCELLED, MEDIA_STATUS_COMPLETED, MEDIA_STATUS_FAILED, MEDIA_STATUS_PENDING,
    MEDIA_STATUS_RUNNING, galgame_media_binding_required_media_kind,
    is_valid_galgame_media_binding_pair, is_valid_media_binding_role,
    is_valid_media_binding_target_kind, is_valid_media_kind, is_valid_media_origin,
    is_valid_media_status,
};

use std::fs;
use std::io::Cursor;
use std::path::{Path, PathBuf};

use a4n_core::error::AppError;
use a4n_core::models::media::media_asset;
use a4n_db::media::{CreateMediaAssetRecordInput, MediaDbError, MediaProvenanceRecordInput};
use sha2::{Digest, Sha256};

use crate::OrchestratorContext;

impl OrchestratorContext {
    pub async fn create_media_asset(
        &self,
        input: CreateMediaAssetInput,
    ) -> Result<media_asset::Model, AppError> {
        let media_kind =
            normalize_media_value(&input.media_kind, "media_kind", is_valid_media_kind)?;
        let origin = normalize_media_value(&input.origin, "origin", is_valid_media_origin)?;
        let status = normalize_media_value(&input.status, "status", is_valid_media_status)?;
        let duration_secs = validate_media_duration(media_kind, status, input.duration_secs)?;
        let checked_file = match input.file_path.as_deref() {
            Some(path) => Some(resolve_data_file(self.data_dir(), path, "媒体文件")?),
            None => None,
        };
        if let Some(path) = input.thumbnail_path.as_deref() {
            resolve_data_file(self.data_dir(), path, "媒体缩略图")?;
        }
        if status == MEDIA_STATUS_COMPLETED && checked_file.is_none() {
            return Err(AppError::InvalidArgument(
                "completed 媒体资产必须提供 file_path".to_string(),
            ));
        }
        let (file_size, checksum) = if let Some(path) = checked_file.as_ref() {
            (
                Some(media_file_size(path)?),
                Some(media_file_checksum(path)?),
            )
        } else {
            (None, None)
        };
        a4n_db::media::create_media_asset_with_provenance_record(
            self.database.connection(),
            CreateMediaAssetRecordInput {
                project_id: input.project_id,
                root_project_id: input.root_project_id,
                source_snapshot_id: input.source_snapshot_id,
                source_chapter_id: input.source_chapter_id,
                source_snapshot_chapter_id: input.source_snapshot_chapter_id,
                media_kind: media_kind.to_string(),
                origin: origin.to_string(),
                file_path: input.file_path,
                thumbnail_path: input.thumbnail_path,
                width: input.width,
                height: input.height,
                duration_secs,
                fps: input.fps,
                file_size,
                checksum,
                status: status.to_string(),
                error_message: input.error_message,
                provenance: media_provenance_record_input(input.provenance),
            },
        )
        .await
        .map_err(media_db_error)
    }

    /// 单张图片生成：校验后创建/恢复 workflow_run，并返回 step_key 与指纹。
    pub async fn begin_image_generation_workflow_validated(
        &self,
        project_id: i64,
        chapter_id: Option<i64>,
        command: &str,
        image_type: &str,
        panel_id: Option<&str>,
        request_json: &serde_json::Value,
        metadata_json: Option<&serde_json::Value>,
    ) -> Result<ImageGenerationWorkflowStart, AppError> {
        let (input, step_key, fingerprint_json) = image_generation_workflow_run_input(
            project_id,
            chapter_id,
            command,
            image_type,
            panel_id,
            request_json,
            metadata_json,
        )
        .map_err(AppError::InvalidArgument)?;
        let workflow_run = self.begin_workflow_run_validated(input).await?;
        Ok(ImageGenerationWorkflowStart {
            workflow_run,
            step_key,
            fingerprint_json,
        })
    }
}

/// 单张图片生成 begin 的返回值（workflow + Host 后续 checkpoint 所需字段）。
#[derive(Debug, Clone)]
pub struct ImageGenerationWorkflowStart {
    pub workflow_run: a4n_core::models::generation::workflow_run::Model,
    pub step_key: String,
    pub fingerprint_json: serde_json::Value,
}

fn resolve_data_file(data_dir: &Path, path: &str, label: &str) -> Result<PathBuf, AppError> {
    let path = required_text(path, label)?;
    let raw = Path::new(path);
    if raw.is_absolute() {
        return Err(AppError::InvalidArgument(format!(
            "{label}必须是应用数据目录内的相对路径"
        )));
    }
    let canonical_data_dir = data_dir.canonicalize()?;
    let canonical_file = data_dir.join(raw).canonicalize()?;
    if !canonical_file.starts_with(&canonical_data_dir) || !canonical_file.is_file() {
        return Err(AppError::InvalidArgument(format!(
            "{label}不在应用数据目录内或不是文件"
        )));
    }
    Ok(canonical_file)
}

fn media_file_size(path: &Path) -> Result<i64, AppError> {
    i64::try_from(fs::metadata(path)?.len())
        .map_err(|_| AppError::InvalidArgument("媒体文件过大".to_string()))
}

fn media_file_checksum(path: &Path) -> Result<String, AppError> {
    let bytes = fs::read(path)?;
    let mut hasher = Sha256::new();
    hasher.update(&bytes);
    Ok(format!("{:x}", hasher.finalize()))
}

pub fn image_dimensions_from_bytes(bytes: &[u8]) -> Result<(i32, i32), AppError> {
    let reader = image::ImageReader::new(Cursor::new(bytes))
        .with_guessed_format()
        .map_err(|error| AppError::InvalidState {
            current: "provider_response".to_string(),
            operation: format!("图片 provider 返回的媒体格式无法识别：{error}"),
        })?;
    let (width, height) = reader
        .into_dimensions()
        .map_err(|error| AppError::InvalidState {
            current: "provider_response".to_string(),
            operation: format!("图片 provider 返回的媒体尺寸无法读取：{error}"),
        })?;
    Ok((
        i32::try_from(width)
            .map_err(|_| AppError::InvalidArgument("图片宽度超出支持范围".to_string()))?,
        i32::try_from(height)
            .map_err(|_| AppError::InvalidArgument("图片高度超出支持范围".to_string()))?,
    ))
}

fn required_text<'a>(value: &'a str, label: &str) -> Result<&'a str, AppError> {
    let value = value.trim();
    if value.is_empty() {
        Err(AppError::InvalidArgument(format!("{label}不能为空")))
    } else {
        Ok(value)
    }
}

fn normalize_media_value<'a>(
    value: &'a str,
    label: &str,
    is_valid: impl FnOnce(&str) -> bool,
) -> Result<&'a str, AppError> {
    let value = value.trim();
    if is_valid(value) {
        Ok(value)
    } else {
        Err(AppError::InvalidArgument(format!("{label} 无效: {value}")))
    }
}

fn validate_media_duration(
    media_kind: &str,
    status: &str,
    duration_secs: Option<f32>,
) -> Result<Option<f32>, AppError> {
    match duration_secs {
        Some(value) if value.is_finite() && value > 0.0 => Ok(Some(value)),
        Some(_) => Err(AppError::InvalidArgument(
            "媒体 duration_secs 必须是正数".to_string(),
        )),
        None if media_kind == MEDIA_KIND_AUDIO && status == MEDIA_STATUS_COMPLETED => Err(
            AppError::InvalidArgument("completed 音频资产必须提供 duration_secs".to_string()),
        ),
        None => Ok(None),
    }
}

fn media_provenance_record_input(input: CreateMediaProvenanceInput) -> MediaProvenanceRecordInput {
    MediaProvenanceRecordInput {
        generation_spec_id: input.generation_spec_id,
        prompt_artifact_id: input.prompt_artifact_id,
        provider_job_id: input.provider_job_id,
        source_media_asset_id: input.source_media_asset_id,
        source_external_path: input.source_external_path,
        imported_from_user: input.imported_from_user,
        request_snapshot_json: input.request_snapshot_json,
    }
}

fn media_db_error(error: MediaDbError) -> AppError {
    match error {
        MediaDbError::NotFound(message) => AppError::NotFound(message),
        MediaDbError::InvalidArgument(message) => AppError::InvalidArgument(message),
        MediaDbError::Database(error) => AppError::Database(error),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use base64::Engine as _;

    const PNG_1X1: &str = "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mNkYAAAAAYAAjCB0C8AAAAASUVORK5CYII=";

    #[test]
    fn media_thumbnail_path_must_be_relative_to_data_dir() {
        let error = resolve_data_file(
            &std::env::temp_dir(),
            "/tmp/outside-thumb.png",
            "媒体缩略图",
        )
        .expect_err("absolute thumbnail path must be rejected before filesystem access");

        assert!(matches!(
            error,
            AppError::InvalidArgument(message)
                if message == "媒体缩略图必须是应用数据目录内的相对路径"
        ));
    }

    #[test]
    fn image_dimensions_use_actual_media_bytes() {
        let bytes = base64::engine::general_purpose::STANDARD
            .decode(PNG_1X1)
            .expect("测试 PNG 应有效");
        assert_eq!(image_dimensions_from_bytes(&bytes).unwrap(), (1, 1));
    }

    #[test]
    fn completed_audio_requires_positive_duration() {
        assert!(matches!(
            validate_media_duration(MEDIA_KIND_AUDIO, MEDIA_STATUS_COMPLETED, None),
            Err(AppError::InvalidArgument(message))
                if message == "completed 音频资产必须提供 duration_secs"
        ));
        assert_eq!(
            validate_media_duration(MEDIA_KIND_AUDIO, MEDIA_STATUS_COMPLETED, Some(1.25)).unwrap(),
            Some(1.25)
        );
    }
}
