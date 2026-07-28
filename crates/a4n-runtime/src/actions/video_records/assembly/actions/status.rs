use crate::AppState;
use crate::actions::access;
use crate::actions::app_error_message;
use crate::actions::video_generation::{GeneratedVideoSummary, video_to_summary};
use a4n_orchestrator::video::UpdateAssemblyPushStatusInput;

pub async fn update_assembly_push_status(
    state: &AppState,
    user_id: i64,
    video_id: i64,
    input: UpdateAssemblyPushStatusInput,
) -> Result<GeneratedVideoSummary, String> {
    let existing = state
        .video()
        .generated_video_by_id(video_id)
        .await
        .map_err(app_error_message)?
        .ok_or_else(|| "视频不存在".to_string())?;
    access::verify_project_editor_string(state, user_id, existing.project_id).await?;
    let row = state
        .video()
        .update_assembly_push_status_validated(existing.id, input)
        .await
        .map_err(app_error_message)?;
    Ok(video_to_summary(&row))
}
