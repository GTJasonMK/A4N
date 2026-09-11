use crate::actions::app_error_message;
use crate::state::AppState;
use a4n_api_contract::character::{RollbackResultSummary, RollbackToChapterInput};

pub async fn rollback_to_chapter(
    state: &AppState,
    user_id: i64,
    input: RollbackToChapterInput,
) -> Result<RollbackResultSummary, String> {
    if input.chapter_number < 0 {
        return Err("回滚章节号不能小于 0".to_string());
    }
    let project_id = state
        .character_profile_read_application()
        .profile_project_id(user_id, input.profile_id)
        .await
        .map_err(app_error_message)?;
    let trace_id = crate::sidecar_clients::character_profile_rollback_rag_operation_trace_id(
        project_id,
        input.profile_id,
        input.chapter_number,
    )?;
    crate::sidecar_clients::scope_rag_operation_trace(
        trace_id,
        state
            .character_profile_timeline_mutation_application()
            .rollback(user_id, input),
    )
    .await
    .map_err(app_error_message)
}
