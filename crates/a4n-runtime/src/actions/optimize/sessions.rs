use crate::AppState;

use super::{
    ListOptimizeSessionsInput, OptimizationPatchSetSummary, OptimizationSessionSummary,
    OptimizationTurnSummary, OptimizeSessionInput, RecordOptimizeAgentEventInput,
};

pub async fn list_optimize_sessions(
    state: &AppState,
    user_id: i64,
    input: ListOptimizeSessionsInput,
) -> Result<Vec<OptimizationSessionSummary>, String> {
    state
        .optimize_application()
        .list_sessions(user_id, input)
        .await
        .map_err(|err| err.to_string())
}

pub async fn get_optimize_session(
    state: &AppState,
    user_id: i64,
    input: OptimizeSessionInput,
) -> Result<OptimizationSessionSummary, String> {
    state
        .optimize_application()
        .get_session(user_id, input)
        .await
        .map_err(|error| error.to_string())
}

pub async fn list_optimize_turns(
    state: &AppState,
    user_id: i64,
    input: OptimizeSessionInput,
) -> Result<Vec<OptimizationTurnSummary>, String> {
    state
        .optimize_application()
        .list_turns(user_id, input)
        .await
        .map_err(|err| err.to_string())
}

pub async fn list_optimize_patch_sets(
    state: &AppState,
    user_id: i64,
    input: OptimizeSessionInput,
) -> Result<Vec<OptimizationPatchSetSummary>, String> {
    state
        .optimize_application()
        .list_patch_sets(user_id, input)
        .await
        .map_err(|err| err.to_string())
}

pub async fn delete_optimize_session(
    state: &AppState,
    user_id: i64,
    input: OptimizeSessionInput,
) -> Result<(), String> {
    state
        .optimize_application()
        .delete_session(user_id, input)
        .await
        .map_err(|err| err.to_string())
}

pub async fn record_optimize_agent_event(
    state: &AppState,
    user_id: i64,
    input: RecordOptimizeAgentEventInput,
) -> Result<(), String> {
    state
        .optimize_application()
        .record_agent_event(user_id, input)
        .await
        .map_err(|err| err.to_string())
}
