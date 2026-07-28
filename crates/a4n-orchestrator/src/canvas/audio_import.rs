use a4n_core::{error::AppError, models::media::media_asset};
use async_trait::async_trait;
use chrono::Utc;
use serde_json::json;

use crate::{
    OrchestratorContext,
    media::{
        CreateMediaAssetInput, CreateMediaProvenanceInput, MEDIA_KIND_AUDIO, MEDIA_ORIGIN_IMPORTED,
        MEDIA_STATUS_COMPLETED,
    },
    project::{PROJECT_KIND_VIDEO, SOURCE_MODE_FREEFORM},
};

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
}

#[async_trait]
pub trait CanvasAudioImportHostPort: Send + Sync {
    async fn persist_canvas_audio(
        &self,
        input: CanvasAudioImportHostInput,
    ) -> Result<PersistedCanvasAudioFile, AppError>;

    async fn remove_canvas_audio(&self, relative_path: &str) -> Result<(), AppError>;
}

impl OrchestratorContext {
    pub async fn import_canvas_audio_validated(
        &self,
        host_port: &dyn CanvasAudioImportHostPort,
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
        let persisted = host_port.persist_canvas_audio(input).await?;
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
                duration_secs: None,
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
                        "imported_at": Utc::now().to_rfc3339(),
                    })),
                },
            })
            .await;
        match create_result {
            Ok(asset) => Ok(asset),
            Err(primary) => match host_port
                .remove_canvas_audio(&persisted.relative_path)
                .await
            {
                Ok(()) => Err(primary),
                Err(cleanup) => Err(AppError::InvalidState {
                    current: primary.to_string(),
                    operation: format!("画布音频记录创建失败后的文件清理也失败：{cleanup}"),
                }),
            },
        }
    }
}
