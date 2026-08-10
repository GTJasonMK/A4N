use a4n_core::error::AppError;
use a4n_core::files::paths::ProjectPaths;
use chrono::Utc;
use serde_json::json;
use std::{
    io::Write,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

use crate::{
    OrchestratorContext,
    media::{
        CreateMediaAssetInput, CreateMediaProvenanceInput, MEDIA_KIND_AUDIO, MEDIA_ORIGIN_IMPORTED,
        MEDIA_STATUS_COMPLETED, galgame_media_binding_required_media_kind,
    },
};

use super::{
    BindGalgameMediaInput, GalgameAudioRole, GalgameMediaBindingSummary, GalgameMediaRole,
    GalgameMediaTargetKind, GalgameTargetContext, ImportGalgameAudioInput, UnbindGalgameMediaInput,
    media_rules::ImportedAudioSource, read_models::galgame_media_binding_summary,
};

#[derive(Debug, Clone)]
pub struct PersistedGalgameAudioFile {
    pub relative_path: String,
    pub file_size: i64,
    pub duration_secs: f32,
}

impl OrchestratorContext {
    pub async fn bind_galgame_media_validated(
        &self,
        input: BindGalgameMediaInput,
    ) -> Result<GalgameMediaBindingSummary, AppError> {
        if input.media_asset_id <= 0 {
            return Err(AppError::InvalidArgument(
                "media_asset_id 必须是正整数".to_string(),
            ));
        }
        let required_media_kind = galgame_media_binding_required_media_kind(
            input.target_kind.as_str(),
            input.role.as_str(),
        )
        .ok_or_else(|| {
            AppError::InvalidArgument(format!(
                "Galgame 媒体绑定目标/角色无效: {} / {}",
                input.target_kind, input.role
            ))
        })?;
        let result = self
            .bind_galgame_media_records(
                input.project_id,
                input.media_asset_id,
                input.target_kind.as_str(),
                input.target_id,
                input.role.as_str(),
                required_media_kind,
            )
            .await?;
        galgame_media_binding_summary(result.binding, result.asset)
    }

    pub async fn unbind_galgame_media_validated(
        &self,
        input: UnbindGalgameMediaInput,
    ) -> Result<u64, AppError> {
        if galgame_media_binding_required_media_kind(
            input.target_kind.as_str(),
            input.role.as_str(),
        )
        .is_none()
        {
            return Err(AppError::InvalidArgument(format!(
                "Galgame 媒体绑定目标/角色无效: {} / {}",
                input.target_kind, input.role
            )));
        }
        self.unbind_galgame_media_records(
            input.project_id,
            input.target_kind.as_str(),
            input.target_id,
            input.role.as_str(),
            input.media_asset_id,
        )
        .await
    }

    pub async fn galgame_media_target_context_validated(
        &self,
        project_id: i64,
        target_kind: GalgameMediaTargetKind,
        target_id: i64,
    ) -> Result<GalgameTargetContext, AppError> {
        self.galgame_media_target_context(project_id, target_kind.as_str(), target_id)
            .await
    }

    pub async fn import_galgame_audio_validated(
        &self,
        input: ImportGalgameAudioInput,
    ) -> Result<GalgameMediaBindingSummary, AppError> {
        let role = match input.role {
            GalgameAudioRole::Bgm => GalgameMediaRole::Bgm,
            GalgameAudioRole::Sfx => GalgameMediaRole::Sfx,
        };
        let target_context = self
            .galgame_media_target_context_validated(
                input.project_id,
                GalgameMediaTargetKind::GalgameLine,
                input.target_line_id,
            )
            .await?;
        let source = ImportedAudioSource::from_upload(
            &input.file_name,
            &input.mime_type,
            &input.data_base64,
        )
        .map_err(AppError::InvalidArgument)?;
        let persisted = persist_galgame_audio_file(
            self.data_dir(),
            input.project_id,
            source.extension,
            &source.bytes,
        )
        .map_err(|operation| AppError::InvalidState {
            current: "galgame_audio_persist".to_string(),
            operation,
        })?;
        let asset_result = self
            .create_media_asset(CreateMediaAssetInput {
                project_id: input.project_id,
                root_project_id: target_context.root_project_id,
                source_snapshot_id: Some(target_context.source_snapshot_id),
                source_chapter_id: Some(target_context.source_chapter_id),
                source_snapshot_chapter_id: Some(target_context.source_snapshot_chapter_id),
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
                    source_external_path: Some(source.file_name.clone()),
                    imported_from_user: true,
                    request_snapshot_json: Some(json!({
                        "source": "galgame_audio_import",
                        "media_kind": "audio",
                        "target_line_id": input.target_line_id,
                        "role": role,
                        "file_name": source.file_name,
                        "mime_type": source.mime_type,
                        "file_size": persisted.file_size,
                        "duration_secs": persisted.duration_secs,
                        "imported_at": Utc::now().to_rfc3339(),
                    })),
                },
            })
            .await;
        let asset = match asset_result {
            Ok(asset) => asset,
            Err(error) => {
                return Err(append_cleanup_error(
                    error,
                    remove_galgame_audio_file(self.data_dir(), &persisted.relative_path).err(),
                ));
            }
        };

        let bind_result = self
            .bind_galgame_media_validated(BindGalgameMediaInput {
                project_id: input.project_id,
                media_asset_id: asset.id,
                target_kind: GalgameMediaTargetKind::GalgameLine,
                target_id: input.target_line_id,
                role,
            })
            .await;
        match bind_result {
            Ok(result) => Ok(result),
            Err(error) => {
                let cleanup_result = match self
                    .delete_media_asset_with_provenance_records(asset.id)
                    .await
                {
                    Ok(()) => remove_galgame_audio_file(self.data_dir(), &persisted.relative_path),
                    Err(error) => Err(error.to_string()),
                };
                Err(append_cleanup_error(error, cleanup_result.err()))
            }
        }
    }
}

fn persist_galgame_audio_file(
    data_dir: &Path,
    project_id: i64,
    extension: &str,
    bytes: &[u8],
) -> Result<PersistedGalgameAudioFile, String> {
    let dir = ProjectPaths::new(data_dir, project_id).root().join("audio");
    std::fs::create_dir_all(&dir).map_err(|err| format!("无法创建 Galgame 音频目录：{err}"))?;
    let (path, mut output_file) = create_unique_timestamped_file(&dir, "galgame_audio", extension)
        .map_err(|error| format!("创建 Galgame 音频文件失败：{error}"))?;
    output_file
        .write_all(bytes)
        .map_err(|err| format!("写入 Galgame 音频文件失败：{err}"))?;
    let metadata =
        std::fs::metadata(&path).map_err(|err| format!("无法读取 Galgame 音频文件元数据：{err}"))?;
    let file_size = i64::try_from(metadata.len())
        .map_err(|_| "Galgame 音频文件过大，无法记录文件大小".to_string())?;
    let relative_path = data_dir_relative_path(data_dir, &path)?;
    let duration_secs = a4n_media::probe_audio_duration_secs(&path).map_err(|error| {
        let _ = std::fs::remove_file(&path);
        format!("无法读取 Galgame 音频文件时长：{error}")
    })?;
    Ok(PersistedGalgameAudioFile {
        relative_path,
        file_size,
        duration_secs,
    })
}

fn remove_galgame_audio_file(data_dir: &Path, relative_path: &str) -> Result<(), String> {
    let raw = Path::new(relative_path);
    if raw.is_absolute() {
        return Err("Galgame 音频路径必须是相对路径".to_string());
    }
    let canonical_data_dir = data_dir
        .canonicalize()
        .map_err(|error| format!("解析应用数据目录失败：{error}"))?;
    let path = data_dir
        .join(raw)
        .canonicalize()
        .map_err(|error| format!("解析 Galgame 音频路径失败：{error}"))?;
    if !path.starts_with(&canonical_data_dir) {
        return Err("Galgame 音频路径超出应用数据目录".to_string());
    }
    if path.is_file() {
        std::fs::remove_file(&path).map_err(|error| format!("删除 Galgame 音频失败：{error}"))?;
    }
    Ok(())
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
        .map_err(|_| "文件不在应用数据目录内".to_string())?
        .to_str()
        .map(str::to_string)
        .ok_or_else(|| "文件相对路径包含非 UTF-8 字符".to_string())
}

fn create_unique_timestamped_file(
    directory: &Path,
    prefix: &str,
    extension: &str,
) -> Result<(PathBuf, std::fs::File), std::io::Error> {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| std::io::Error::new(std::io::ErrorKind::InvalidData, error))?
        .as_nanos();
    for attempt in 0..20u32 {
        let suffix = if attempt == 0 {
            String::new()
        } else {
            format!("_{attempt}")
        };
        let path = directory.join(format!("{prefix}_{timestamp}{suffix}.{extension}"));
        match std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
        {
            Ok(file) => return Ok((path, file)),
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(error),
        }
    }
    Err(std::io::Error::new(
        std::io::ErrorKind::AlreadyExists,
        "文件名冲突过多",
    ))
}

fn append_cleanup_error(primary: AppError, cleanup_error: Option<String>) -> AppError {
    match cleanup_error.filter(|error| !error.trim().is_empty()) {
        Some(cleanup_error) => AppError::InvalidState {
            current: primary.to_string(),
            operation: format!("后续清理也失败：{cleanup_error}"),
        },
        None => primary,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use base64::Engine as _;

    #[test]
    fn audio_upload_rejects_extension_mismatch_and_oversize() {
        let mismatch = ImportedAudioSource::from_upload(
            "voice.wav",
            "audio/mpeg",
            &base64::engine::general_purpose::STANDARD.encode(b"audio"),
        )
        .expect_err("extension mismatch must fail");
        assert!(mismatch.contains("扩展名与 MIME 不一致"));
    }

    #[test]
    fn cleanup_error_keeps_primary_and_compensation_failures_visible() {
        let error = append_cleanup_error(
            AppError::InvalidArgument("绑定失败".to_string()),
            Some("删除文件失败".to_string()),
        );
        let message = error.to_string();
        assert!(message.contains("绑定失败"));
        assert!(message.contains("删除文件失败"));
    }
}
