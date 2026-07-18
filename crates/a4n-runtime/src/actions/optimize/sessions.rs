use crate::AppState;
use a4n_orchestrator::optimize::OptimizationSessionMessageInsertInput;

use super::{
    OptimizationPatchSetSummary, OptimizationSessionSummary, OptimizationTurnSummary,
    access::{optimize_session, verify_optimize_project_access, verify_optimize_session_access},
    projections::{to_patch_set_summary, to_session_summary, to_turn_summary},
};

pub async fn list_optimize_sessions(
    state: &AppState,
    user_id: i64,
    project_id: i64,
    chapter_id: Option<i64>,
) -> Result<Vec<OptimizationSessionSummary>, String> {
    verify_optimize_project_access(state, user_id, project_id).await?;
    let sessions = state
        .orchestrator
        .optimization_sessions(project_id, chapter_id)
        .await
        .map_err(|err| err.to_string())?;
    Ok(sessions.iter().map(to_session_summary).collect())
}

pub async fn get_optimize_session(
    state: &AppState,
    user_id: i64,
    session_id: i64,
) -> Result<OptimizationSessionSummary, String> {
    verify_optimize_session_access(state, user_id, session_id).await?;
    let session = optimize_session(state, session_id).await?;
    Ok(to_session_summary(&session))
}

pub async fn list_optimize_turns(
    state: &AppState,
    user_id: i64,
    session_id: i64,
) -> Result<Vec<OptimizationTurnSummary>, String> {
    verify_optimize_session_access(state, user_id, session_id).await?;
    let turns = state
        .orchestrator
        .optimization_turns(session_id)
        .await
        .map_err(|err| err.to_string())?;
    let mut summaries = Vec::with_capacity(turns.len());
    for turn in turns {
        let events = state
            .orchestrator
            .optimization_turn_events(turn.id)
            .await
            .map_err(|err| err.to_string())?;
        summaries.push(to_turn_summary(&turn, events));
    }
    Ok(summaries)
}

pub async fn list_optimize_patch_sets(
    state: &AppState,
    user_id: i64,
    session_id: i64,
) -> Result<Vec<OptimizationPatchSetSummary>, String> {
    verify_optimize_session_access(state, user_id, session_id).await?;
    let patch_sets = state
        .orchestrator
        .optimization_patch_sets(session_id)
        .await
        .map_err(|err| err.to_string())?;
    let mut summaries = Vec::with_capacity(patch_sets.len());
    for patch_set in patch_sets {
        let patches = state
            .orchestrator
            .optimization_patch_set_patches(patch_set.id)
            .await
            .map_err(|err| err.to_string())?;
        summaries.push(to_patch_set_summary(&patch_set, patches));
    }
    Ok(summaries)
}

pub async fn delete_optimize_session(
    state: &AppState,
    user_id: i64,
    session_id: i64,
) -> Result<(), String> {
    verify_optimize_session_access(state, user_id, session_id).await?;
    optimize_session(state, session_id).await?;
    let rows_affected = state
        .orchestrator
        .delete_optimization_session_with_messages(session_id)
        .await
        .map_err(|err| err.to_string())?;
    if rows_affected != 1 {
        return Err(format!("优化会话不存在: {session_id}"));
    }
    Ok(())
}

pub async fn record_optimize_agent_event(
    state: &AppState,
    user_id: i64,
    session_id: i64,
    content: String,
) -> Result<(), String> {
    verify_optimize_session_access(state, user_id, session_id).await?;
    let content = content.trim();
    if content.is_empty() {
        return Err("Agent 事件内容不能为空".to_string());
    }
    state
        .orchestrator
        .insert_optimization_session_message(OptimizationSessionMessageInsertInput {
            session_id,
            role: "agent_event".to_string(),
            content: content.to_string(),
            tool_calls: None,
        })
        .await
        .map_err(|err| err.to_string())?;
    Ok(())
}
