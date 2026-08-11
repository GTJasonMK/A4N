use crate::AppState;

use super::{
    OptimizationPatchSetSummary, OptimizationSessionSummary, OptimizationTurnSummary,
    projections::{to_patch_set_summary, to_session_summary, to_turn_summary},
};

pub async fn list_optimize_sessions(
    state: &AppState,
    user_id: i64,
    project_id: i64,
    chapter_id: Option<i64>,
) -> Result<Vec<OptimizationSessionSummary>, String> {
    let sessions = state
        .orchestrator()
        .optimize_sessions_for_user(user_id, project_id, chapter_id)
        .await
        .map_err(|err| err.to_string())?;
    Ok(sessions.iter().map(to_session_summary).collect())
}

pub async fn get_optimize_session(
    state: &AppState,
    user_id: i64,
    session_id: i64,
) -> Result<OptimizationSessionSummary, String> {
    let session = state
        .orchestrator()
        .optimize_session_for_user(user_id, session_id)
        .await
        .map_err(|error| error.to_string())?;
    Ok(to_session_summary(&session))
}

pub async fn list_optimize_turns(
    state: &AppState,
    user_id: i64,
    session_id: i64,
) -> Result<Vec<OptimizationTurnSummary>, String> {
    let turns = state
        .orchestrator()
        .optimize_turns_for_user(user_id, session_id)
        .await
        .map_err(|err| err.to_string())?;
    Ok(turns
        .into_iter()
        .map(|(turn, events)| to_turn_summary(&turn, events))
        .collect())
}

pub async fn list_optimize_patch_sets(
    state: &AppState,
    user_id: i64,
    session_id: i64,
) -> Result<Vec<OptimizationPatchSetSummary>, String> {
    let patch_sets = state
        .orchestrator()
        .optimize_patch_sets_for_user(user_id, session_id)
        .await
        .map_err(|err| err.to_string())?;
    Ok(patch_sets
        .into_iter()
        .map(|(patch_set, patches)| to_patch_set_summary(&patch_set, patches))
        .collect())
}

pub async fn delete_optimize_session(
    state: &AppState,
    user_id: i64,
    session_id: i64,
) -> Result<(), String> {
    state
        .orchestrator()
        .delete_optimize_session_for_user_validated(user_id, session_id)
        .await
        .map_err(|err| err.to_string())
}

pub async fn record_optimize_agent_event(
    state: &AppState,
    user_id: i64,
    session_id: i64,
    content: String,
) -> Result<(), String> {
    state
        .orchestrator()
        .record_optimize_agent_event_for_user_validated(user_id, session_id, &content)
        .await
        .map_err(|err| err.to_string())
}
