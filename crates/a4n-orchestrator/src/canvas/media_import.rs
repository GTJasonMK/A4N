use a4n_core::{
    error::AppError,
    models::{
        ai::{generated_image, generated_video},
        media::media_asset,
    },
};
use async_trait::async_trait;

use crate::{
    OrchestratorContext,
    canvas::{CanvasImageMediaAssetEnsureInput, CanvasVideoMediaAssetEnsureInput},
    media::MEDIA_ORIGIN_IMPORTED,
    project::PROJECT_KIND_VIDEO,
};

#[derive(Debug, Clone)]
pub struct CanvasImageImportHostInput {
    pub user_id: i64,
    pub project_id: i64,
    pub file_name: String,
    pub mime_type: String,
    pub data_base64: String,
}

#[derive(Debug, Clone)]
pub struct CanvasVideoImportHostInput {
    pub user_id: i64,
    pub project_id: i64,
    pub file_path: Option<String>,
    pub file_name: Option<String>,
    pub mime_type: Option<String>,
    pub data_base64: Option<String>,
}

#[async_trait]
pub trait CanvasImageImportHostPort: Send + Sync {
    async fn import_canvas_image(
        &self,
        input: CanvasImageImportHostInput,
    ) -> Result<generated_image::Model, AppError>;
}

#[async_trait]
pub trait CanvasVideoImportHostPort: Send + Sync {
    async fn import_canvas_video(
        &self,
        input: CanvasVideoImportHostInput,
    ) -> Result<generated_video::Model, AppError>;

    async fn delete_canvas_video(&self, video_id: i64) -> Result<(), AppError>;
}

#[derive(Debug, Clone)]
pub struct CanvasImageImportApplicationResult {
    pub image: generated_image::Model,
    pub asset: media_asset::Model,
}

#[derive(Debug, Clone)]
pub struct CanvasVideoImportApplicationResult {
    pub video: generated_video::Model,
    pub asset: media_asset::Model,
}

impl OrchestratorContext {
    pub async fn import_canvas_image_validated(
        &self,
        host_port: &dyn CanvasImageImportHostPort,
        input: CanvasImageImportHostInput,
    ) -> Result<CanvasImageImportApplicationResult, AppError> {
        self.verify_project_permission(
            input.project_id,
            input.user_id,
            crate::collaboration::ProjectAccessRole::Editor,
        )
        .await?;
        let project_id = input.project_id;
        let user_id = input.user_id;
        self.require_canvas_video_project(project_id).await?;
        let image = host_port.import_canvas_image(input).await?;
        let asset = match self
            .ensure_canvas_image_media_asset_record(CanvasImageMediaAssetEnsureInput {
                project_id,
                image: image.clone(),
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
        Ok(CanvasImageImportApplicationResult { image, asset })
    }

    pub async fn import_canvas_video_validated(
        &self,
        host_port: &dyn CanvasVideoImportHostPort,
        input: CanvasVideoImportHostInput,
    ) -> Result<CanvasVideoImportApplicationResult, AppError> {
        self.verify_project_permission(
            input.project_id,
            input.user_id,
            crate::collaboration::ProjectAccessRole::Editor,
        )
        .await?;
        let project_id = input.project_id;
        self.require_canvas_video_project(project_id).await?;
        let video = host_port.import_canvas_video(input).await?;
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
        Ok(CanvasVideoImportApplicationResult { video, asset })
    }

    async fn require_canvas_video_project(&self, project_id: i64) -> Result<(), AppError> {
        if project_id <= 0 {
            return Err(AppError::InvalidArgument(
                "项目 ID 必须是正整数".to_string(),
            ));
        }
        let project = self
            .project_by_id(project_id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("项目不存在: {project_id}")))?;
        if project.project_kind != PROJECT_KIND_VIDEO {
            return Err(AppError::InvalidArgument(
                "画布媒体导入只支持视频项目".to_string(),
            ));
        }
        Ok(())
    }
}
