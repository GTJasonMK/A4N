use super::super::{ImportCanvasMediaInput, ImportedCanvasMedia};
use crate::{
    AppState,
    actions::{
        app_error_message,
        image_generation::to_summary,
        image_records::{UploadImageInput, delete_image, upload_image},
    },
};
use a4n_core::{error::AppError, models::ai::generated_image};
use a4n_orchestrator::canvas::{CanvasImageImportHostInput, CanvasImageImportHostPort};
use async_trait::async_trait;
use serde_json::json;

pub(super) async fn import_canvas_image(
    state: &AppState,
    user_id: i64,
    input: ImportCanvasMediaInput,
) -> Result<ImportedCanvasMedia, String> {
    let host_port = RuntimeCanvasImageImportHostPort { state, user_id };
    let result = state
        .canvas()
        .import_canvas_image_validated(
            &host_port,
            CanvasImageImportHostInput {
                user_id,
                project_id: input.project_id,
                file_name: input
                    .file_name
                    .ok_or_else(|| "导入画布图片缺少 file_name".to_string())?,
                mime_type: input
                    .mime_type
                    .ok_or_else(|| "导入画布图片缺少 mime_type".to_string())?,
                data_base64: input
                    .data_base64
                    .ok_or_else(|| "导入画布图片缺少 data_base64".to_string())?,
                prompt: input.prompt.unwrap_or_else(|| "导入画布图片".to_string()),
            },
        )
        .await
        .map_err(app_error_message)?;
    Ok(ImportedCanvasMedia {
        media_kind: "image".to_string(),
        image: Some(
            serde_json::to_value(to_summary(&result.image))
                .map_err(|error| format!("序列化画布图片摘要失败: {error}"))?,
        ),
        video: None,
        asset: serde_json::to_value(result.asset)
            .map_err(|error| format!("序列化画布图片资产失败: {error}"))?,
    })
}

struct RuntimeCanvasImageImportHostPort<'a> {
    state: &'a AppState,
    user_id: i64,
}

#[async_trait]
impl CanvasImageImportHostPort for RuntimeCanvasImageImportHostPort<'_> {
    async fn import_canvas_image(
        &self,
        input: CanvasImageImportHostInput,
    ) -> Result<generated_image::Model, AppError> {
        let summary = upload_image(
            self.state,
            self.user_id,
            input.project_id,
            UploadImageInput {
                file_name: input.file_name,
                mime_type: input.mime_type,
                data_base64: input.data_base64,
                image_type: "canvas_image".to_string(),
                prompt: input.prompt,
                model: Some("external-import".to_string()),
                chapter_id: None,
                panel_id: None,
                metadata_json: Some(json!({
                    "canvas_import": true,
                    "source": "video_canvas"
                })),
            },
        )
        .await
        .map_err(host_error)?;
        self.state
            .video()
            .generated_image_by_id(summary.id)
            .await
            .map_err(|error| host_error(error.to_string()))?
            .ok_or_else(|| AppError::NotFound(format!("画布导入图片不存在: {}", summary.id)))
    }

    async fn delete_canvas_image(&self, image_id: i64) -> Result<(), AppError> {
        delete_image(self.state, self.user_id, image_id)
            .await
            .map_err(host_error)
    }
}

fn host_error(operation: String) -> AppError {
    AppError::InvalidState {
        current: "canvas_image_import_host".to_string(),
        operation,
    }
}
