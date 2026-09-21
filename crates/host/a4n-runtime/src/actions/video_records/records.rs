use crate::AppState;
use crate::actions::app_error_message;
use crate::actions::video_generation::GeneratedVideoSummary;
use a4n_api_contract::video::{SelectSegmentVideoInput, UpdateGeneratedVideoInput};

pub async fn list_videos_by_chapter(
    state: &AppState,
    user_id: i64,
    project_id: i64,
    chapter_id: i64,
) -> Result<Vec<GeneratedVideoSummary>, String> {
    state
        .video_record_read_application()
        .list_by_chapter(user_id, project_id, chapter_id)
        .await
        .map_err(app_error_message)
}

pub async fn update_video(
    state: &AppState,
    user_id: i64,
    video_id: i64,
    input: UpdateGeneratedVideoInput,
) -> Result<GeneratedVideoSummary, String> {
    state
        .video_record_mutation_application()
        .update(user_id, video_id, input)
        .await
        .map_err(app_error_message)
}

pub async fn select_segment_video(
    state: &AppState,
    user_id: i64,
    project_id: i64,
    chapter_id: i64,
    video_id: i64,
    input: SelectSegmentVideoInput,
) -> Result<GeneratedVideoSummary, String> {
    state
        .video_record_mutation_application()
        .select_segment(user_id, project_id, chapter_id, video_id, input)
        .await
        .map_err(app_error_message)
}
