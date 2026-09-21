use a4n_api_contract::media::MediaAudioProbeHostPort;
use a4n_error::AppError;
use a4n_media::decode_uploaded_audio_data;
use a4n_storage::remove_existing_data_file;
use chrono::Utc;
use serde_json::json;
use std::io::Write;

use crate::{
    OrchestratorContext,
    media::{
        CreateMediaAssetInput, CreateMediaProvenanceInput, MEDIA_KIND_AUDIO, MEDIA_ORIGIN_IMPORTED,
        MEDIA_STATUS_COMPLETED, galgame_media_binding_required_media_kind, persist_audio_file,
    },
};

use super::{
    BindGalgameMediaInput, GalgameAudioRole, GalgameMediaBindingSummary, GalgameMediaRole,
    GalgameMediaTargetKind, GalgameTargetContext, ImportGalgameAudioInput, UnbindGalgameMediaInput,
    read_models::galgame_media_binding_summary,
};

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
        host_port: &dyn MediaAudioProbeHostPort,
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
        let source =
            decode_uploaded_audio_data(&input.file_name, &input.mime_type, &input.data_base64)
                .map_err(AppError::InvalidArgument)?;
        if !matches!(source.extension, "mp3" | "wav" | "ogg") {
            return Err(AppError::InvalidArgument(format!(
                "Galgame 音频格式不支持: {}",
                input.mime_type.trim()
            )));
        }
        let persisted = persist_audio_file(
            self.data_dir(),
            input.project_id,
            "galgame_audio",
            source.extension,
            "Galgame 音频",
            "Galgame 音频文件",
            host_port,
            |output| {
                output
                    .write_all(&source.bytes)
                    .map_err(|error| format!("写入 Galgame 音频文件失败：{error}"))
            },
        )
        .await
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
                        "file_size": source.bytes.len(),
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
                    remove_existing_data_file(
                        self.data_dir(),
                        &persisted.relative_path,
                        "Galgame 音频",
                    )
                    .err(),
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
                    Ok(()) => remove_existing_data_file(
                        self.data_dir(),
                        &persisted.relative_path,
                        "Galgame 音频",
                    ),
                    Err(error) => Err(error.to_string()),
                };
                Err(append_cleanup_error(error, cleanup_result.err()))
            }
        }
    }
}

fn append_cleanup_error(primary: AppError, cleanup_error: Option<String>) -> AppError {
    match cleanup_error.filter(|error| !error.trim().is_empty()) {
        Some(cleanup_error) => crate::errors::app_error_with_secondary(
            primary,
            format!("后续清理也失败：{cleanup_error}"),
        ),
        None => primary,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
