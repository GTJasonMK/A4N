use a4n_core::{
    error::AppError,
    files::{
        paths::{ProjectPaths, data_dir_relative_file_path},
        create_unique_timestamped_file, remove_created_file_on_error_sync,
        remove_existing_data_file,
    },
    models::media::media_asset,
};
use chrono::Utc;
use serde_json::json;
use std::{
    fs::File,
    io::Write,
    path::{Path, PathBuf},
};

use crate::{
    OrchestratorContext,
    media::{
        CreateMediaAssetInput, CreateMediaProvenanceInput, MEDIA_KIND_AUDIO,
        MEDIA_ORIGIN_IMPORTED, MEDIA_STATUS_COMPLETED, MAX_IMPORT_AUDIO_BYTES,
        decode_uploaded_audio_data,
    },
    project::ProjectSourceMode,
};

use super::audio_rules::canvas_audio_format;

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
struct PersistedCanvasAudioFile {
    pub relative_path: String,
    pub source_file_name: String,
    pub mime_type: String,
    pub duration_secs: f32,
}

enum CanvasAudioSourceData {
    Path(PathBuf),
    Bytes(Vec<u8>),
}

struct CanvasAudioImportSource {
    file_name: String,
    extension: &'static str,
    mime_type: &'static str,
    data: CanvasAudioSourceData,
}

impl CanvasAudioImportSource {
    fn from_input(input: &CanvasAudioImportHostInput) -> Result<Self, String> {
        match (input.file_path.as_deref(), input.data_base64.as_deref()) {
            (Some(_), Some(_)) => Err(
                "导入画布音频只能提供 file_path 或 data_base64，不能同时提供".to_string(),
            ),
            (None, Some(data_base64)) => Self::from_upload(
                input
                    .file_name
                    .as_deref()
                    .ok_or_else(|| "导入画布音频缺少 file_name".to_string())?,
                input
                    .mime_type
                    .as_deref()
                    .ok_or_else(|| "导入画布音频缺少 mime_type".to_string())?,
                data_base64,
            ),
            (Some(_), None) if input.file_name.is_some() || input.mime_type.is_some() => Err(
                "file_path 画布音频导入不能同时提供 file_name 或 mime_type".to_string(),
            ),
            (Some(file_path), None) => Self::from_path(file_path),
            (None, None) => Err("导入画布音频缺少 file_path 或 data_base64".to_string()),
        }
    }

    fn file_name(&self) -> &str {
        &self.file_name
    }

    fn extension(&self) -> &'static str {
        self.extension
    }

    fn mime_type(&self) -> &'static str {
        self.mime_type
    }

    fn write_to(&self, mut output: File) -> Result<(), String> {
        match &self.data {
            CanvasAudioSourceData::Bytes(bytes) => output
                .write_all(bytes)
                .map_err(|err| format!("写入画布音频文件失败：{err}"))?,
            CanvasAudioSourceData::Path(path) => {
                let mut input = File::open(path)
                    .map_err(|err| format!("打开导入画布音频文件失败：{err}"))?;
                std::io::copy(&mut input, &mut output)
                    .map_err(|err| format!("复制画布音频文件失败：{err}"))?;
            }
        }
        output
            .flush()
            .map_err(|err| format!("刷新画布音频文件失败：{err}"))
    }

    fn from_upload(file_name: &str, mime_type: &str, data_base64: &str) -> Result<Self, String> {
        let source = decode_uploaded_audio_data(file_name, mime_type, data_base64)?;
        Ok(Self {
            file_name: source.file_name,
            extension: source.extension,
            mime_type: source.mime_type,
            data: CanvasAudioSourceData::Bytes(source.bytes),
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
        if metadata.len() > MAX_IMPORT_AUDIO_BYTES as u64 {
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
            .ok_or_else(|| "导入音频路径缺少有效文件名".to_string())?;
        let (mime_type, extension) = canvas_audio_format(&canonical)?;
        Ok(Self {
            file_name,
            extension,
            mime_type,
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
        if project.source_mode().map_err(|error| {
            AppError::InvalidArgument(format!("项目 {}: {error}", project.id))
        })? != ProjectSourceMode::Freeform
        {
            return Err(AppError::InvalidArgument(
                "画布音频导入只支持自由视频项目".to_string(),
            ));
        }

        let project_id = input.project_id;
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
                        "mime_type": persisted.mime_type,
                        "duration_secs": persisted.duration_secs,
                        "imported_at": Utc::now().to_rfc3339(),
                    })),
                },
            })
            .await;
        match create_result {
            Ok(asset) => Ok(asset),
            Err(primary) => {
                match remove_existing_data_file(
                    self.data_dir(),
                    &persisted.relative_path,
                    "画布音频",
                ) {
                    Ok(()) => Err(primary),
                    Err(cleanup) => Err(crate::errors::app_error_with_secondary(
                        primary,
                        format!("画布音频记录创建失败后的文件清理也失败：{cleanup}"),
                    )),
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
    let dir = ProjectPaths::new(data_dir, input.project_id).audio_dir();
    std::fs::create_dir_all(&dir).map_err(|error| AppError::InvalidState {
        current: "canvas_audio_import".to_string(),
        operation: format!("无法创建项目音频目录：{error}"),
    })?;
    let (output, output_file) =
        create_unique_timestamped_file(&dir, "canvas_audio", source.extension()).map_err(
            |error| AppError::InvalidState {
                current: "canvas_audio_import".to_string(),
                operation: format!("创建画布音频文件失败：{error}"),
            },
        )?;
    remove_created_file_on_error_sync(&output, "画布音频文件", || {
        source.write_to(output_file)?;
        let relative_path = data_dir_relative_file_path(data_dir, &output, "画布音频文件")?;
        let duration_secs = a4n_media::probe_audio_duration_secs(&output)
            .map_err(|error| format!("无法读取画布音频文件时长：{error}"))?;
        Ok(PersistedCanvasAudioFile {
            relative_path,
            source_file_name: source.file_name().to_string(),
            mime_type: source.mime_type().to_string(),
            duration_secs,
        })
    })
    .map_err(|operation| AppError::InvalidState {
            current: "canvas_audio_import".to_string(),
            operation,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn upload_source_requires_mime_and_rejects_mixed_path_input() {
        let mut input = CanvasAudioImportHostInput {
            user_id: 1,
            project_id: 1,
            file_path: None,
            file_name: Some("voice.mp3".to_string()),
            mime_type: None,
            data_base64: Some("SUQz".to_string()),
        };
        assert!(CanvasAudioImportSource::from_input(&input).is_err());

        input.mime_type = Some("audio/mpeg".to_string());
        input.file_path = Some("voice.mp3".to_string());
        assert!(CanvasAudioImportSource::from_input(&input).is_err());
    }
}
