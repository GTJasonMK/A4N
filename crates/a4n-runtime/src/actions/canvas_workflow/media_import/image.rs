use super::super::{ImportCanvasMediaInput, ImportedCanvasMedia};
use crate::{
    AppState,
    actions::{
        app_error_message,
        image_generation::to_summary,
        image_records::{import_image_record, UploadImageInput},
    },
};
use a4n_core::{error::AppError, models::ai::generated_image};
use a4n_domain::image::IMAGE_TYPE_CANVAS;
use a4n_orchestrator::canvas::{CanvasImageImportHostInput, CanvasImageImportHostPort};
use async_trait::async_trait;
use serde_json::{json, Map};

pub(super) async fn import_canvas_image(
    state: &AppState,
    user_id: i64,
    input: ImportCanvasMediaInput,
) -> Result<ImportedCanvasMedia, String> {
    if input.file_path.is_some() {
        return Err("导入画布图片不支持 file_path，请提供上传文件字段".to_string());
    }
    let host_port = RuntimeCanvasImageImportHostPort { state, user_id };
    let result = state
        .orchestrator()
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
        let file_name = input.file_name;
        import_image_record(
            self.state,
            self.user_id,
            UploadImageInput {
                project_id: input.project_id,
                file_name: file_name.clone(),
                mime_type: input.mime_type,
                data_base64: input.data_base64,
                image_type: IMAGE_TYPE_CANVAS.to_string(),
                prompt: file_name,
                chapter_id: None,
                panel_id: None,
                metadata_json: Some(Map::from_iter([
                    ("canvas_import".to_string(), json!(true)),
                    ("source".to_string(), json!("video_canvas")),
                ])),
            },
        )
        .await
        .map_err(|operation| AppError::InvalidState {
            current: "canvas_image_import_host".to_string(),
            operation,
        })
    }
}
