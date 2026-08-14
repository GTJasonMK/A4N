use crate::AppState;
use crate::actions::access;
use crate::actions::app_error_message;
use crate::actions::video_generation::{GeneratedVideoSummary, video_to_summary};
use a4n_api_contract::video::{SelectSegmentVideoInput, UpdateGeneratedVideoInput};
use a4n_core::models::ai::generated_video;

pub async fn list_videos_by_chapter(
    state: &AppState,
    user_id: i64,
    project_id: i64,
    chapter_id: i64,
) -> Result<Vec<GeneratedVideoSummary>, String> {
    access::verify_project_access_string(state, user_id, project_id).await?;
    let rows = state
        .orchestrator()
        .list_video_records_by_chapter_validated(project_id, chapter_id)
        .await
        .map_err(app_error_message)?;
    Ok(rows.iter().map(video_to_summary).collect())
}

pub async fn update_video(
    state: &AppState,
    user_id: i64,
    video_id: i64,
    input: UpdateGeneratedVideoInput,
) -> Result<GeneratedVideoSummary, String> {
    let existing = required_generated_video(state.orchestrator(), video_id).await?;
    access::verify_project_editor_string(state, user_id, existing.project_id).await?;
    let row = state
        .orchestrator()
        .update_video_record_validated(video_id, input)
        .await
        .map_err(app_error_message)?;
    Ok(video_to_summary(&row))
}

pub async fn select_segment_video(
    state: &AppState,
    user_id: i64,
    project_id: i64,
    chapter_id: i64,
    video_id: i64,
    input: SelectSegmentVideoInput,
) -> Result<GeneratedVideoSummary, String> {
    access::verify_project_editor_string(state, user_id, project_id).await?;
    let row = state
        .orchestrator()
        .select_segment_video_validated(
            project_id,
            chapter_id,
            input.segment_plan_prompt_id,
            (
                &input.target_segment.source_shot_id,
                input.target_segment.segment_index_in_shot,
            ),
            video_id,
        )
        .await
        .map_err(app_error_message)?;
    Ok(video_to_summary(&row))
}

pub(super) async fn required_generated_video(
    video: &a4n_orchestrator::OrchestratorContext,
    video_id: i64,
) -> Result<generated_video::Model, String> {
    video
        .generated_video_by_id(video_id)
        .await
        .map_err(app_error_message)?
        .ok_or_else(|| "视频不存在".to_string())
}
