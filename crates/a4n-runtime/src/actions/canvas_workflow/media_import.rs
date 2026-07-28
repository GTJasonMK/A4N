mod audio;
mod image;
mod video;

use super::{ImportCanvasMediaInput, ImportedCanvasMedia};
use crate::AppState;

pub async fn import_canvas_media(
    state: &AppState,
    user_id: i64,
    input: ImportCanvasMediaInput,
) -> Result<ImportedCanvasMedia, String> {
    match input.media_kind.trim() {
        "image" => image::import_canvas_image(state, user_id, input).await,
        "video" => video::import_canvas_video_media(state, user_id, input).await,
        "audio" => audio::import_canvas_audio(state, user_id, input).await,
        other => Err(format!("画布媒体类型无效: {other}")),
    }
}
