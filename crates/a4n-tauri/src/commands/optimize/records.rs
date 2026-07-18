use a4n_runtime::AppState;
use a4n_runtime::host_api::optimize::{
    self as optimize_actions, OptimizationPatchSetSummary, OptimizationSessionSummary,
    OptimizationTurnSummary,
};
use tauri::State;

use super::adapters::optimize_session_command;

#[tauri::command]
pub async fn list_optimize_sessions(
    state: State<'_, AppState>,
    user_id: i64,
    project_id: i64,
    chapter_id: Option<i64>,
) -> Result<Vec<OptimizationSessionSummary>, String> {
    optimize_actions::list_optimize_sessions(state.inner(), user_id, project_id, chapter_id).await
}

optimize_session_command!(
    get_optimize_session,
    OptimizationSessionSummary,
    optimize_actions::get_optimize_session,
);
optimize_session_command!(
    list_optimize_turns,
    Vec<OptimizationTurnSummary>,
    optimize_actions::list_optimize_turns,
);
optimize_session_command!(
    list_optimize_patch_sets,
    Vec<OptimizationPatchSetSummary>,
    optimize_actions::list_optimize_patch_sets,
);
optimize_session_command!(
    reject_pending_optimize_patch_sets,
    (),
    optimize_actions::reject_pending_optimize_patch_sets,
);
optimize_session_command!(
    delete_optimize_session,
    (),
    optimize_actions::delete_optimize_session,
);

#[tauri::command]
pub async fn record_optimize_agent_event(
    state: State<'_, AppState>,
    user_id: i64,
    session_id: i64,
    content: String,
) -> Result<(), String> {
    optimize_actions::record_optimize_agent_event(state.inner(), user_id, session_id, content).await
}
