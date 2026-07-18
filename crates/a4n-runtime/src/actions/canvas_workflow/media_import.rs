mod audio;
mod image;
mod sync;
mod video;

use super::{ImportCanvasMediaInput, ImportedCanvasMedia};
use crate::AppState;
use crate::actions::access;

pub async fn import_canvas_media(
    state: &AppState,
    user_id: i64,
    input: ImportCanvasMediaInput,
) -> Result<ImportedCanvasMedia, String> {
    access::verify_project_editor_string(state, user_id, input.project_id).await?;
    match input.media_kind.trim() {
        "image" => image::import_canvas_image(state, user_id, input).await,
        "video" => video::import_canvas_video_media(state, user_id, input).await,
        "audio" => audio::import_canvas_audio(state, input).await,
        other => Err(format!("画布媒体类型无效: {other}")),
    }
}
