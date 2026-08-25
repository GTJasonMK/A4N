mod audio;
mod image;
mod video;

use super::{ImportCanvasMediaInput, ImportedCanvasMedia};
use crate::AppState;
use a4n_api_contract::canvas::CanvasMediaAssetSummary;
use a4n_core::models::media::media_asset;
use a4n_orchestrator::media::{MEDIA_KIND_AUDIO, MEDIA_KIND_IMAGE, MEDIA_KIND_VIDEO};

pub(super) fn canvas_media_asset_summary(asset: media_asset::Model) -> CanvasMediaAssetSummary {
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
