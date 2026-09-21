mod audio;
mod image;
mod video;

use super::{ImportCanvasMediaInput, ImportedCanvasMedia};
use crate::AppState;
use a4n_domain::media::{MEDIA_KIND_AUDIO, MEDIA_KIND_IMAGE, MEDIA_KIND_VIDEO};

pub async fn import_canvas_media(
    state: &AppState,
    user_id: i64,
    input: ImportCanvasMediaInput,
) -> Result<ImportedCanvasMedia, String> {
    match input.media_kind.as_str() {
        MEDIA_KIND_IMAGE => image::import_canvas_image(state, user_id, input).await,
        MEDIA_KIND_VIDEO => video::import_canvas_video_media(state, user_id, input).await,
        MEDIA_KIND_AUDIO => audio::import_canvas_audio(state, user_id, input).await,
        other => Err(format!("画布媒体类型无效: {other}")),
    }
}
