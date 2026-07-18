use super::super::{ImportCanvasMediaInput, ImportedCanvasMedia};
use super::sync::{sync_imported_canvas_media_asset_or_cleanup, sync_imported_canvas_video_asset};
use crate::AppState;
use crate::actions::video_records::{ImportCanvasVideoInput, delete_video, import_canvas_video};

pub(super) async fn import_canvas_video_media(
    state: &AppState,
    user_id: i64,
    input: ImportCanvasMediaInput,
) -> Result<ImportedCanvasMedia, String> {
    let video = import_canvas_video(
        state,
        user_id,
        input.project_id,
        ImportCanvasVideoInput {
            file_path: input.file_path,
            file_name: input.file_name,
            mime_type: input.mime_type,
            data_base64: input.data_base64,
            prompt: input.prompt,
        },
    )
    .await?;
    let asset = sync_imported_canvas_media_asset_or_cleanup(
        sync_imported_canvas_video_asset(state, input.project_id, video.id),
        delete_video(state, user_id, video.id),
        "画布导入视频补偿删除失败",
    )
    .await?;
    Ok(ImportedCanvasMedia::Video { video, asset })
}
