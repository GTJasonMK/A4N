use super::super::{ImportCanvasMediaInput, ImportedCanvasMedia};
use super::sync::{sync_imported_canvas_image_asset, sync_imported_canvas_media_asset_or_cleanup};
use crate::AppState;
use crate::actions::image_records::{UploadImageInput, delete_image, upload_image};
use serde_json::json;

pub(super) async fn import_canvas_image(
    state: &AppState,
    user_id: i64,
    input: ImportCanvasMediaInput,
) -> Result<ImportedCanvasMedia, String> {
    let image = upload_image(
        state,
        user_id,
        input.project_id,
        UploadImageInput {
            file_name: input
                .file_name
                .ok_or_else(|| "导入画布图片缺少 file_name".to_string())?,
            mime_type: input
                .mime_type
                .ok_or_else(|| "导入画布图片缺少 mime_type".to_string())?,
            data_base64: input
                .data_base64
                .ok_or_else(|| "导入画布图片缺少 data_base64".to_string())?,
            image_type: "canvas_image".to_string(),
            prompt: input.prompt.unwrap_or_else(|| "导入画布图片".to_string()),
            model: Some("external-import".to_string()),
            chapter_id: None,
            panel_id: None,
            metadata_json: Some(json!({
                "canvas_import": true,
                "source": "video_canvas"
            })),
        },
    )
    .await?;
    let asset = sync_imported_canvas_media_asset_or_cleanup(
        sync_imported_canvas_image_asset(state, input.project_id, image.id),
        delete_image(state, user_id, image.id),
        "画布导入图片补偿删除失败",
    )
    .await?;
    Ok(ImportedCanvasMedia::Image { image, asset })
}
