use super::super::{ImportCanvasMediaInput, ImportedCanvasMedia};
use crate::{
    AppState,
    actions::{app_error_message, image_generation::to_summary},
};
use a4n_orchestrator::canvas::CanvasImageImportHostInput;

pub(super) async fn import_canvas_image(
    state: &AppState,
    user_id: i64,
    input: ImportCanvasMediaInput,
) -> Result<ImportedCanvasMedia, String> {
    let result = state
        .canvas()
        .import_canvas_image_validated(CanvasImageImportHostInput {
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
        })
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
