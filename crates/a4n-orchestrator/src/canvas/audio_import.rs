use a4n_core::{error::AppError, files::paths::ProjectPaths, models::media::media_asset};
use base64::Engine as _;
use chrono::Utc;
use serde_json::json;
use std::path::{Path, PathBuf};

use crate::{
    OrchestratorContext,
    media::{
        CreateMediaAssetInput, CreateMediaProvenanceInput, MEDIA_KIND_AUDIO, MEDIA_ORIGIN_IMPORTED,
        MEDIA_STATUS_COMPLETED,
    },
    project::{PROJECT_KIND_VIDEO, SOURCE_MODE_FREEFORM},
};

use super::audio_rules::{
    canvas_audio_extension, required_canvas_audio_file_name, validate_canvas_audio_mime,
};

const MAX_IMPORT_AUDIO_BYTES: u64 = 256 * 1024 * 1024;

#[derive(Debug, Clone)]
pub struct CanvasAudioImportHostInput {
    pub user_id: i64,
    pub project_id: i64,
    pub file_path: Option<String>,
    pub file_name: Option<String>,
    pub mime_type: Option<String>,
    pub data_base64: Option<String>,
}

#[derive(Debug, Clone)]
pub struct PersistedCanvasAudioFile {
    pub relative_path: String,
    pub source_file_name: String,
    pub mime_type: Option<String>,
    pub duration_secs: f32,
}

enum CanvasAudioSourceData {
    Path(PathBuf),
    Bytes(Vec<u8>),
}

struct CanvasAudioImportSource {
    file_name: String,
    extension: &'static str,
    data: CanvasAudioSourceData,
}

impl CanvasAudioImportSource {
    fn from_input(input: &CanvasAudioImportHostInput) -> Result<Self, String> {
        if let Some(data_base64) = input.data_base64.as_deref() {
            return Self::from_upload(
                input
                    .file_name
                    .as_deref()
                    .ok_or_else(|| "导入画布音频缺少 file_name".to_string())?,
                input
                    .mime_type
                    .as_deref()
                    .unwrap_or("application/octet-stream"),
                data_base64,
            );
        }
        let file_path = input
            .file_path
            .as_deref()
            .ok_or_else(|| "导入画布音频缺少 file_path 或 data_base64".to_string())?;
        Self::from_path(file_path)
    }

    fn file_name(&self) -> &str {
        &self.file_name
    }

    fn extension(&self) -> &'static str {
        self.extension
    }

    fn write_to(&self, output: &Path) -> Result<(), String> {
        match &self.data {
            CanvasAudioSourceData::Bytes(bytes) => {
                std::fs::write(output, bytes).map_err(|err| format!("写入画布音频文件失败：{err}"))
            }
            CanvasAudioSourceData::Path(path) => std::fs::copy(path, output)
                .map(|_| ())
                .map_err(|err| format!("复制画布音频文件失败：{err}")),
        }
    }

    fn from_upload(file_name: &str, mime_type: &str, data_base64: &str) -> Result<Self, String> {
        let file_name = required_canvas_audio_file_name(file_name)?;
        let extension = canvas_audio_extension(Path::new(&file_name))?;
        validate_canvas_audio_mime(mime_type, extension)?;
        let raw = data_base64
            .split_once(";base64,")
            .map(|(_, body)| body)
            .unwrap_or(data_base64)
            .trim();
        if raw.is_empty() {
            return Err("导入画布音频数据不能为空".to_string());
        }
        let bytes = base64::engine::general_purpose::STANDARD
            .decode(raw)
            .map_err(|err| format!("导入画布音频 base64 解码失败：{err}"))?;
        if bytes.is_empty() {
            return Err("导入画布音频文件不能为空".to_string());
        }
        if bytes.len() as u64 > MAX_IMPORT_AUDIO_BYTES {
            return Err(format!(
                "导入画布音频不能超过 {}MB",
                MAX_IMPORT_AUDIO_BYTES / 1024 / 1024
            ));
        }
        Ok(Self {
            file_name,
            extension,
            data: CanvasAudioSourceData::Bytes(bytes),
        })
    }

    fn from_path(file_path: &str) -> Result<Self, String> {
        let trimmed = file_path.trim();
        if trimmed.is_empty() {
            return Err("请选择要导入的音频文件".to_string());
        }
        let path = PathBuf::from(trimmed);
        let metadata =
            std::fs::metadata(&path).map_err(|err| format!("无法读取导入音频文件：{err}"))?;
        if !metadata.is_file() {
            return Err("导入音频路径不是文件".to_string());
        }
        if metadata.len() > MAX_IMPORT_AUDIO_BYTES {
            return Err(format!(
                "导入画布音频不能超过 {}MB",
                MAX_IMPORT_AUDIO_BYTES / 1024 / 1024
            ));
        }
        let canonical = path
            .canonicalize()
            .map_err(|err| format!("无法解析导入音频路径：{err}"))?;
        let file_name = canonical
            .file_name()
            .and_then(|value| value.to_str())
            .map(str::to_string)
            .unwrap_or_else(|| "audio".to_string());
        let extension = canvas_audio_extension(&canonical)?;
        Ok(Self {
            file_name,
            extension,
            data: CanvasAudioSourceData::Path(canonical),
        })
    }
}

impl OrchestratorContext {
    pub async fn import_canvas_audio_validated(
        &self,
        input: CanvasAudioImportHostInput,
    ) -> Result<media_asset::Model, AppError> {
        self.verify_project_permission(
            input.project_id,
            input.user_id,
            crate::collaboration::ProjectAccessRole::Editor,
        )
        .await?;
        if input.project_id <= 0 {
            return Err(AppError::InvalidArgument(
                "项目 ID 必须是正整数".to_string(),
            ));
        }
        let project = self
            .project_by_id(input.project_id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("项目不存在: {}", input.project_id)))?;
        if project.project_kind != PROJECT_KIND_VIDEO || project.source_mode != SOURCE_MODE_FREEFORM
        {
            return Err(AppError::InvalidArgument(
                "画布音频导入只支持 source_mode=freeform 的视频项目".to_string(),
            ));
        }

        let project_id = input.project_id;
        let mime_type = input.mime_type.clone();
        let persisted = persist_canvas_audio(self.data_dir(), input)?;
        let create_result = self
            .create_media_asset(CreateMediaAssetInput {
                project_id,
                root_project_id: project_id,
                source_snapshot_id: None,
                source_chapter_id: None,
                source_snapshot_chapter_id: None,
                media_kind: MEDIA_KIND_AUDIO.to_string(),
                origin: MEDIA_ORIGIN_IMPORTED.to_string(),
                file_path: Some(persisted.relative_path.clone()),
                thumbnail_path: None,
                width: None,
                height: None,
                duration_secs: Some(persisted.duration_secs),
                fps: None,
                status: MEDIA_STATUS_COMPLETED.to_string(),
                error_message: None,
                provenance: CreateMediaProvenanceInput {
                    generation_spec_id: None,
                    prompt_artifact_id: None,
                    provider_job_id: None,
                    source_media_asset_id: None,
                    source_external_path: Some(persisted.source_file_name.clone()),
                    imported_from_user: true,
                    request_snapshot_json: Some(json!({
                        "source": "video_canvas",
                        "canvas_import": true,
                        "media_kind": "audio",
                        "file_name": persisted.source_file_name,
                        "mime_type": mime_type,
                        "duration_secs": persisted.duration_secs,
                        "imported_at": Utc::now().to_rfc3339(),
                    })),
                },
            })
            .await;
        match create_result {
            Ok(asset) => Ok(asset),
            Err(primary) => {
                match remove_canvas_audio_file(self.data_dir(), &persisted.relative_path) {
                    Ok(()) => Err(primary),
                    Err(cleanup) => Err(AppError::InvalidState {
                        current: primary.to_string(),
                        operation: format!("画布音频记录创建失败后的文件清理也失败：{cleanup}"),
                    }),
                }
            }
        }
    }
}

fn persist_canvas_audio(
    data_dir: &Path,
    input: CanvasAudioImportHostInput,
) -> Result<PersistedCanvasAudioFile, AppError> {
    let source = CanvasAudioImportSource::from_input(&input).map_err(AppError::InvalidArgument)?;
    let output = canvas_audio_output_path(data_dir, input.project_id, source.extension())
        .map_err(AppError::InvalidArgument)?;
    if let Err(error) = source.write_to(&output) {
        let _ = std::fs::remove_file(&output);
        return Err(AppError::InvalidState {
            current: "canvas_audio_import".to_string(),
            operation: error,
        });
    }
    let relative_path = match data_dir_relative_path(data_dir, &output) {
        Ok(path) => path,
        Err(error) => {
            let _ = std::fs::remove_file(&output);
            return Err(AppError::InvalidState {
                current: "canvas_audio_import".to_string(),
                operation: error,
            });
        }
    };
    let duration_secs = a4n_media::probe_audio_duration_secs(&output).map_err(|error| {
        let _ = std::fs::remove_file(&output);
        AppError::InvalidState {
            current: "canvas_audio_import".to_string(),
            operation: format!("无法读取画布音频文件时长：{error}"),
        }
    })?;
    Ok(PersistedCanvasAudioFile {
        relative_path,
        source_file_name: source.file_name().to_string(),
        mime_type: input.mime_type,
        duration_secs,
    })
}

fn remove_canvas_audio_file(data_dir: &Path, relative_path: &str) -> Result<(), AppError> {
    let raw = Path::new(relative_path);
    if raw.is_absolute() {
        return Err(AppError::InvalidArgument(
            "画布音频路径必须是相对路径".to_string(),
        ));
    }
    let canonical_data_dir = data_dir
        .canonicalize()
        .map_err(|error| AppError::InvalidState {
            current: "canvas_audio_import".to_string(),
            operation: format!("解析应用数据目录失败：{error}"),
        })?;
    let path = match data_dir.join(raw).canonicalize() {
        Ok(path) => path,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(error) => {
            return Err(AppError::InvalidState {
                current: "canvas_audio_import".to_string(),
                operation: format!("解析画布音频路径失败：{error}"),
            });
        }
    };
    if !path.starts_with(&canonical_data_dir) {
        return Err(AppError::InvalidArgument(
            "画布音频路径超出应用数据目录".to_string(),
        ));
    }
    if path.is_file() {
        std::fs::remove_file(&path).map_err(|error| AppError::InvalidState {
            current: "canvas_audio_import".to_string(),
            operation: format!("删除画布音频失败：{error}"),
        })?;
    }
    Ok(())
}

fn canvas_audio_output_path(
    data_dir: &Path,
    project_id: i64,
    extension: &str,
) -> Result<PathBuf, String> {
    let dir = ProjectPaths::new(data_dir, project_id).root().join("audio");
    std::fs::create_dir_all(&dir).map_err(|err| format!("无法创建项目音频目录：{err}"))?;
    let stamp = Utc::now()
        .timestamp_nanos_opt()
        .unwrap_or_else(|| Utc::now().timestamp_micros() * 1000);
    Ok(dir.join(format!(
        "canvas_audio_{}_{}.{}",
        std::process::id(),
        stamp,
        extension
    )))
}

fn data_dir_relative_path(data_dir: &Path, path: &Path) -> Result<String, String> {
    let canonical_data_dir = data_dir
        .canonicalize()
        .map_err(|error| format!("解析应用数据目录失败：{error}"))?;
    let canonical_path = path
        .canonicalize()
        .map_err(|error| format!("解析文件路径失败：{error}"))?;
    canonical_path
        .strip_prefix(&canonical_data_dir)
        .map_err(|_| "画布音频文件不在应用数据目录内".to_string())?
        .to_str()
        .map(str::to_string)
        .ok_or_else(|| "文件相对路径包含非 UTF-8 字符".to_string())
}
