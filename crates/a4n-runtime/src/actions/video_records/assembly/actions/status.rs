use crate::AppState;
use crate::actions::app_error_message;
use crate::actions::video_generation::GeneratedVideoSummary;
use a4n_api_contract::video::UpdateAssemblyPushStatusInput;

pub async fn update_assembly_push_status(
    state: &AppState,
    user_id: i64,
    video_id: i64,
    input: UpdateAssemblyPushStatusInput,
) -> Result<GeneratedVideoSummary, String> {
    state
        .video_record_application()
        .update_assembly_push_status(user_id, video_id, input)
        .await
        .map_err(app_error_message)
}
