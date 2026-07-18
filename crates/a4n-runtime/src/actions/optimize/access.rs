use crate::{
    AppState,
    actions::{access, app_error_message},
};
use a4n_core::models::{ai::optimization_session, chapter::chapter};

use super::OptimizationTaskScope;

pub(super) async fn optimize_session(
    state: &AppState,
    session_id: i64,
) -> Result<optimization_session::Model, String> {
    state
        .orchestrator
        .optimization_session_by_id(session_id)
        .await
        .map_err(|err| err.to_string())?
        .ok_or_else(|| "优化会话不存在".to_string())
}

pub(super) async fn verify_optimize_session_access(
    state: &AppState,
    user_id: i64,
    session_id: i64,
) -> Result<i64, String> {
    let project_id = optimize_session(state, session_id).await?.project_id;
    verify_optimize_project_access(state, user_id, project_id).await?;
    Ok(project_id)
}

pub(super) async fn optimize_session_task_scope(
    state: &AppState,
    user_id: i64,
    session_id: i64,
    total: u32,
) -> Result<OptimizationTaskScope, String> {
    let project_id = verify_optimize_session_access(state, user_id, session_id).await?;
    Ok(OptimizationTaskScope { project_id, total })
}

pub(super) async fn verify_optimize_project_access(
    state: &AppState,
    user_id: i64,
    project_id: i64,
) -> Result<(), String> {
    access::verify_project_editor(state, user_id, project_id)
        .await
        .map_err(|err| err.to_string())
}

pub(super) async fn find_chapter_for_optimize(
    state: &AppState,
    project_id: i64,
    chapter_number: i32,
) -> Result<chapter::Model, String> {
    state
        .orchestrator
        .chapter_by_project_number(project_id, chapter_number)
        .await
        .map_err(app_error_message)?
        .ok_or_else(|| {
            format!("章节不存在: project_id={project_id}, chapter_number={chapter_number}")
        })
}

pub(super) async fn find_chapter_by_id(
    state: &AppState,
    chapter_id: i64,
) -> Result<chapter::Model, String> {
    state
        .orchestrator
        .chapter_by_id(chapter_id)
        .await
        .map_err(app_error_message)?
        .ok_or_else(|| format!("章节不存在: {chapter_id}"))
}
