use crate::AppState;
use a4n_core::error::AppError;
use a4n_orchestrator::workflow::{
    UpsertWorkflowRunInput, WORKFLOW_STATUS_COMPLETED, WORKFLOW_STATUS_FAILED,
    WORKFLOW_STATUS_RUNNING,
};
use serde_json::json;

use super::access::optimize_session;

#[derive(Debug)]
pub(super) struct OptimizeCommandError {
    message: String,
    cancelled: bool,
}

impl OptimizeCommandError {
    pub(super) fn from_app(error: AppError) -> Self {
        Self {
            cancelled: matches!(&error, AppError::Cancelled),
            message: error.to_string(),
        }
    }

    pub(super) fn from_message(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            cancelled: false,
        }
    }

    fn append(mut self, suffix: impl std::fmt::Display) -> Self {
        self.message = format!("{}；{suffix}", self.message);
        self
    }
}

pub(super) async fn start_optimize_workflow_run(
    state: &AppState,
    session_id: i64,
    workflow_kind: &str,
    mode: &str,
) -> Result<i64, String> {
    let session = optimize_session(state, session_id).await?;
    let current_step_key = format!("optimization_session:{session_id}:{mode}");
    let run = state
        .orchestrator
        .upsert_workflow_run(UpsertWorkflowRunInput {
            business_key: format!("{workflow_kind}:{session_id}:{mode}"),
            workflow_kind: workflow_kind.to_string(),
            project_id: Some(session.project_id),
            root_project_id: None,
            source_snapshot_id: None,
            scope_table: Some("optimization_session".to_string()),
            scope_id: Some(session_id),
            chapter_id: Some(session.chapter_id),
            mode: Some(mode.to_string()),
            input_fingerprint_json: Some(json!({
                "workflow": workflow_kind,
                "mode": mode,
                "session_id": session_id,
                "project_id": session.project_id,
                "chapter_id": session.chapter_id,
            })),
            status: WORKFLOW_STATUS_RUNNING.to_string(),
            current_step_key: Some(current_step_key),
            active_task_id: None,
            can_resume: true,
            resume_block_reason: None,
            error_message: None,
        })
        .await
        .map_err(|err| err.to_string())?;
    Ok(run.id)
}

pub(super) async fn finalize_optimize_workflow_run<T>(
    state: &AppState,
    workflow_run_id: i64,
    operation: &str,
    result: Result<T, OptimizeCommandError>,
) -> Result<T, OptimizeCommandError> {
    let (status, can_resume, error_message) = match &result {
        Ok(_) => (WORKFLOW_STATUS_COMPLETED, false, None),
        Err(error) => (WORKFLOW_STATUS_FAILED, true, Some(error.message.as_str())),
    };
    match state
        .orchestrator
        .mark_workflow_status(workflow_run_id, status, can_resume, error_message)
        .await
    {
        Ok(_) => result,
        Err(status_err) => match result {
            Ok(_) => Err(OptimizeCommandError::from_message(format!(
                "{operation}完成，但工作流状态写入失败: {status_err}"
            ))),
            Err(error) => Err(error.append(format!("工作流失败状态写入失败: {status_err}"))),
        },
    }
}

async fn record_optimize_command_error(
    state: &AppState,
    session_id: i64,
    phase: &str,
    error: &OptimizeCommandError,
) -> Result<(), String> {
    state
        .record_command_error(session_id, phase, &error.message, error.cancelled)
        .await
        .map_err(|save_err| format!("记录 {phase} 失败事件失败：{save_err}"))
}

pub(super) async fn record_optimize_command_result<T>(
    state: &AppState,
    session_id: i64,
    phase: &str,
    result: Result<T, OptimizeCommandError>,
) -> Result<T, String> {
    match result {
        Ok(value) => Ok(value),
        Err(error) => {
            if let Err(save_err) =
                record_optimize_command_error(state, session_id, phase, &error).await
            {
                return Err(format!("{}；{save_err}", error.message));
            }
            Err(error.message)
        }
    }
}
