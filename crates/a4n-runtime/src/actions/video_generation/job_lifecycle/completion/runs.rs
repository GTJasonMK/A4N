use a4n_core::models::ai::generated_video;
use serde_json::json;

use crate::AppState;
use crate::actions::video_generation::ensure_provider_generation_run_updated;

pub(super) async fn complete_video_generation_runs(
    state: &AppState,
    video: &generated_video::Model,
) -> Result<(), String> {
    state
        .orchestrator
        .complete_generation_runs_for_target(
            "generated_video",
            video.id,
            Some(json!({
                "generated_video_id": video.id,
                "file_path": video.file_path,
                "thumbnail_path": video.thumbnail_path,
                "file_size": video.file_size,
                "status": video.status,
                "provider_result": video.metadata_json,
            })),
        )
        .await
        .map_err(crate::actions::app_error_message)
        .and_then(|runs| {
            ensure_provider_generation_run_updated(video.id, runs.len(), "完成 provider 视频生成")
        })?;
    Ok(())
}
