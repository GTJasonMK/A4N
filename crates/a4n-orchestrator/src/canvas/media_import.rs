use a4n_api_contract::canvas::CanvasMediaAssetSummary;
use a4n_canvas_contract::{
    CanvasAudioImportApplicationPort, CanvasAudioImportHostInput, CanvasImageImportApplicationPort,
    CanvasImageImportApplicationResult, CanvasImageImportHostInput, CanvasImageImportHostPort,
    CanvasVideoImportApplicationPort, CanvasVideoImportApplicationResult,
    CanvasVideoImportHostInput, CanvasVideoImportHostPort,
};
use a4n_db_models::models::media::media_asset;
use a4n_error::AppError;
use async_trait::async_trait;

use crate::{
    OrchestratorContext,
    canvas::{CanvasImageMediaAssetEnsureInput, CanvasVideoMediaAssetEnsureInput},
    media::MEDIA_ORIGIN_IMPORTED,
    project::PROJECT_KIND_VIDEO,
};

impl OrchestratorContext {
    pub async fn import_canvas_image_validated(
        &self,
        host_port: &dyn CanvasImageImportHostPort,
        user_id: i64,
        input: CanvasImageImportHostInput,
    ) -> Result<CanvasImageImportApplicationResult, AppError> {
        self.authorized_canvas_video_project(input.project_id, user_id)
            .await?;
        let project_id = input.project_id;
        let image = host_port.import_canvas_image(input).await?;
        let image_model = self
            .generated_image_by_id(image.id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("图片记录不存在: {}", image.id)))?;
        if image_model.project_id != project_id {
            return Err(AppError::InvalidArgument(format!(
                "图片 #{} 不属于项目 {}",
                image.id, project_id
            )));
        }
        let asset = match self
            .ensure_canvas_image_media_asset_record(CanvasImageMediaAssetEnsureInput {
                project_id,
                image: image_model,
                generation_spec_id: None,
                origin: MEDIA_ORIGIN_IMPORTED.to_string(),
            })
            .await
        {
            Ok(asset) => asset,
            Err(primary) => {
                if let Err(cleanup) = self
                    .delete_generated_image_for_user_validated(user_id, image.id)
                    .await
                {
                    return Err(crate::errors::app_error_with_secondary(
                        primary,
                        format!("画布导入图片媒体资产登记失败后的补偿删除也失败：{cleanup}"),
                    ));
                }
                return Err(primary);
            }
        };
        Ok(CanvasImageImportApplicationResult {
            image,
            asset: canvas_media_asset_summary(asset),
        })
    }

    pub async fn import_canvas_video_validated(
        &self,
        host_port: &dyn CanvasVideoImportHostPort,
        user_id: i64,
        input: CanvasVideoImportHostInput,
    ) -> Result<CanvasVideoImportApplicationResult, AppError> {
        self.authorized_canvas_video_project(input.project_id, user_id)
            .await?;
        let project_id = input.project_id;
        let video_id = host_port.import_canvas_video(input).await?;
        let video = self.required_video_record(video_id).await?;
        if video.project_id != project_id {
            return Err(AppError::InvalidArgument(format!(
                "视频 #{} 不属于项目 {}",
                video.id, project_id
            )));
        }
        let asset = match self
            .ensure_canvas_video_media_asset_record(CanvasVideoMediaAssetEnsureInput {
                project_id,
                video: video.clone(),
                generation_spec_id: None,
                origin: MEDIA_ORIGIN_IMPORTED.to_string(),
            })
            .await
        {
            Ok(asset) => asset,
            Err(primary) => {
                if let Err(cleanup) = host_port.delete_canvas_video(video.id).await {
                    return Err(crate::errors::app_error_with_secondary(
                        primary,
                        format!("画布导入视频媒体资产登记失败后的补偿删除也失败：{cleanup}"),
                    ));
                }
                return Err(primary);
            }
        };
        Ok(CanvasVideoImportApplicationResult {
            video: crate::video::generated_video_summary(&video),
            asset: canvas_media_asset_summary(asset),
        })
    }

    pub(crate) async fn authorized_canvas_video_project(
        &self,
        project_id: i64,
        user_id: i64,
    ) -> Result<a4n_db_models::models::project::novel_project::Model, AppError> {
        if project_id <= 0 {
            return Err(AppError::InvalidArgument(
                "项目 ID 必须是正整数".to_string(),
            ));
        }
        self.verify_project_permission(
            project_id,
            user_id,
            crate::collaboration::ProjectAccessRole::Editor,
        )
        .await?;
        let project = self
            .project_by_id(project_id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("项目不存在: {project_id}")))?;
        if project.project_kind != PROJECT_KIND_VIDEO {
            return Err(AppError::InvalidArgument(
                "画布媒体导入只支持视频项目".to_string(),
            ));
        }
        Ok(project)
    }
}

#[async_trait]
impl CanvasImageImportApplicationPort for OrchestratorContext {
    async fn import_image(
        &self,
        host_port: &dyn CanvasImageImportHostPort,
        user_id: i64,
        input: CanvasImageImportHostInput,
    ) -> Result<CanvasImageImportApplicationResult, AppError> {
        self.import_canvas_image_validated(host_port, user_id, input)
            .await
    }
}

#[async_trait]
impl CanvasVideoImportApplicationPort for OrchestratorContext {
    async fn import_video(
        &self,
        host_port: &dyn CanvasVideoImportHostPort,
        user_id: i64,
        input: CanvasVideoImportHostInput,
    ) -> Result<CanvasVideoImportApplicationResult, AppError> {
        self.import_canvas_video_validated(host_port, user_id, input)
            .await
    }
}

#[async_trait]
impl CanvasAudioImportApplicationPort for OrchestratorContext {
    async fn import_audio(
        &self,
        user_id: i64,
        input: CanvasAudioImportHostInput,
    ) -> Result<CanvasMediaAssetSummary, AppError> {
        self.import_canvas_audio_validated(user_id, input)
            .await
            .map(canvas_media_asset_summary)
    }
}

fn canvas_media_asset_summary(asset: media_asset::Model) -> CanvasMediaAssetSummary {
    CanvasMediaAssetSummary {
        id: asset.id,
        project_id: asset.project_id,
        root_project_id: asset.root_project_id,
        source_snapshot_id: asset.source_snapshot_id,
        source_chapter_id: asset.source_chapter_id,
        source_snapshot_chapter_id: asset.source_snapshot_chapter_id,
        media_kind: asset.media_kind,
        origin: asset.origin,
        file_path: asset.file_path,
        thumbnail_path: asset.thumbnail_path,
        width: asset.width,
        height: asset.height,
        duration_secs: asset.duration_secs,
        fps: asset.fps,
        file_size: asset.file_size,
        checksum: asset.checksum,
        status: asset.status,
        error_message: asset.error_message,
        created_at: asset.created_at.to_string(),
        updated_at: asset.updated_at.to_string(),
    }
}
