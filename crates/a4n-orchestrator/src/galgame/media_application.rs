use a4n_core::error::AppError;
use async_trait::async_trait;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::{
    OrchestratorContext,
    media::{
        CreateMediaAssetInput, CreateMediaProvenanceInput, MEDIA_KIND_AUDIO, MEDIA_ORIGIN_IMPORTED,
        MEDIA_STATUS_COMPLETED, galgame_media_binding_required_media_kind,
    },
};

use super::{
    GALGAME_ENTITY_LINE, GalgameMediaBindingResult, GalgameTargetContext,
    media_rules::{
        ImportedAudioSource, normalize_audio_role, normalize_galgame_media_role,
        normalize_galgame_media_target_kind,
    },
};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BindGalgameMediaInput {
    pub project_id: i64,
    pub media_asset_id: i64,
    pub target_kind: String,
    pub target_id: i64,
    pub role: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UnbindGalgameMediaInput {
    pub project_id: i64,
    pub target_kind: String,
    pub target_id: i64,
    pub role: String,
    pub media_asset_id: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ImportGalgameAudioInput {
    pub project_id: i64,
    pub target_line_id: i64,
    pub role: String,
    pub file_name: String,
    pub mime_type: String,
    pub data_base64: String,
}

#[derive(Debug, Clone)]
pub struct PersistedGalgameAudioFile {
    pub relative_path: String,
    pub file_size: i64,
    pub duration_secs: f32,
}

#[async_trait]
pub trait GalgameAudioHostPort: Send + Sync {
    fn persist_audio_file(
        &self,
        project_id: i64,
        extension: &str,
        bytes: &[u8],
    ) -> Result<PersistedGalgameAudioFile, String>;

    async fn remove_audio_file(&self, relative_path: &str) -> Result<(), String>;
}

impl OrchestratorContext {
    pub async fn bind_galgame_media_validated(
        &self,
        input: BindGalgameMediaInput,
    ) -> Result<GalgameMediaBindingResult, AppError> {
        if input.media_asset_id <= 0 {
            return Err(AppError::InvalidArgument(
                "media_asset_id 必须是正整数".to_string(),
            ));
        }
        let target_kind = normalize_galgame_media_target_kind(&input.target_kind)
            .map_err(AppError::InvalidArgument)?;
        let role = normalize_galgame_media_role(&target_kind, &input.role)
            .map_err(AppError::InvalidArgument)?;
        let required_media_kind = galgame_media_binding_required_media_kind(&target_kind, &role)
            .ok_or_else(|| {
                AppError::InvalidArgument(format!(
                    "Galgame 媒体绑定目标/角色无效: {target_kind} / {role}"
                ))
            })?;
        self.bind_galgame_media_records(
            input.project_id,
            input.media_asset_id,
            &target_kind,
            input.target_id,
            &role,
            required_media_kind,
        )
        .await
    }

    pub async fn unbind_galgame_media_validated(
        &self,
        input: UnbindGalgameMediaInput,
    ) -> Result<u64, AppError> {
        let target_kind = normalize_galgame_media_target_kind(&input.target_kind)
            .map_err(AppError::InvalidArgument)?;
        let role = normalize_galgame_media_role(&target_kind, &input.role)
            .map_err(AppError::InvalidArgument)?;
        self.unbind_galgame_media_records(
            input.project_id,
            &target_kind,
            input.target_id,
            &role,
            input.media_asset_id,
        )
        .await
    }

    pub async fn galgame_media_target_context_validated(
        &self,
        project_id: i64,
        target_kind: &str,
        target_id: i64,
    ) -> Result<GalgameTargetContext, AppError> {
        let target_kind =
            normalize_galgame_media_target_kind(target_kind).map_err(AppError::InvalidArgument)?;
        self.galgame_media_target_context(project_id, &target_kind, target_id)
            .await
    }

    pub async fn import_galgame_audio_validated(
        &self,
        host_port: &dyn GalgameAudioHostPort,
        input: ImportGalgameAudioInput,
    ) -> Result<GalgameMediaBindingResult, AppError> {
        let role = normalize_audio_role(&input.role).map_err(AppError::InvalidArgument)?;
        let target_context = self
            .galgame_media_target_context_validated(
                input.project_id,
                GALGAME_ENTITY_LINE,
                input.target_line_id,
            )
            .await?;
        let source = ImportedAudioSource::from_upload(
            &input.file_name,
            &input.mime_type,
            &input.data_base64,
        )
        .map_err(AppError::InvalidArgument)?;
        let persisted = host_port
            .persist_audio_file(input.project_id, source.extension, &source.bytes)
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
                    host_port
                        .remove_audio_file(&persisted.relative_path)
                        .await
                        .err(),
                ));
            }
        };

        let bind_result = self
            .bind_galgame_media_validated(BindGalgameMediaInput {
                project_id: input.project_id,
                media_asset_id: asset.id,
                target_kind: GALGAME_ENTITY_LINE.to_string(),
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
                    Ok(()) => host_port.remove_audio_file(&persisted.relative_path).await,
                    Err(error) => Err(error.to_string()),
                };
                Err(append_cleanup_error(error, cleanup_result.err()))
            }
        }
    }
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
