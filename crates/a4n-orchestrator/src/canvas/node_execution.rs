use a4n_core::error::AppError;
use serde_json::Value;

use crate::{
    OrchestratorContext,
    workflow::{
        UpsertWorkflowCheckpointInput, WORKFLOW_STATUS_CANCELLED, WORKFLOW_STATUS_COMPLETED,
        WORKFLOW_STATUS_FAILED, WORKFLOW_STATUS_RUNNING, WorkflowProgressEventInput,
    },
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CanvasNodeRunHandle {
    pub workflow_run_id: i64,
    pub finalize_on_success: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CanvasNodeRunSummary {
    pub workflow_run_id: i64,
    pub project_id: i64,
    pub graph_id: i64,
    pub mode: String,
    pub status: String,
    pub error_message: Option<String>,
}

pub(crate) struct CanvasNodeLifecycle {
    pub(crate) run: CanvasNodeRunHandle,
    pub(crate) project_id: i64,
    pub(crate) graph_id: i64,
    pub(crate) mode: String,
    pub(crate) step_key: String,
    pub(crate) input_json: Value,
    pub(crate) generation_spec_id: Option<i64>,
}

impl OrchestratorContext {
    pub(crate) async fn start_canvas_node_execution(
        &self,
        project_id: i64,
        graph_id: i64,
        mode: &str,
        step_key: String,
        input_json: Value,
        shared_run: Option<CanvasNodeRunHandle>,
    ) -> Result<CanvasNodeLifecycle, AppError> {
        let run = if let Some(run) = shared_run {
            run
        } else {
            let run = self
                .begin_canvas_node_step_workflow_validated(
                    project_id,
                    graph_id,
                    mode,
                    &step_key,
                    input_json.clone(),
                )
                .await?;
            CanvasNodeRunHandle {
                workflow_run_id: run.id,
                finalize_on_success: true,
            }
        };
        self.record_canvas_node_checkpoint(
            run.workflow_run_id,
            step_key.clone(),
            WORKFLOW_STATUS_RUNNING,
            input_json.clone(),
            None,
            None,
            false,
        )
        .await?;
        Ok(CanvasNodeLifecycle {
            run,
            project_id,
            graph_id,
            mode: mode.to_string(),
            step_key,
            input_json,
            generation_spec_id: None,
        })
    }

    pub(crate) async fn record_canvas_node_checkpoint(
        &self,
        workflow_run_id: i64,
        step_key: String,
        status: &str,
        input_json: Value,
        artifacts_json: Option<Value>,
        error_message: Option<String>,
        completed: bool,
    ) -> Result<(), AppError> {
        let message = match status {
            WORKFLOW_STATUS_RUNNING => format!("画布节点运行中: {step_key}"),
            WORKFLOW_STATUS_COMPLETED => format!("画布节点已完成: {step_key}"),
            WORKFLOW_STATUS_FAILED => format!("画布节点失败: {step_key}"),
            other => {
                return Err(AppError::InvalidArgument(format!(
                    "画布节点 checkpoint 状态无效: {other}"
                )));
            }
        };
        if status != WORKFLOW_STATUS_COMPLETED {
            self.mark_workflow_current_step(workflow_run_id, &step_key)
                .await?;
        }
        self.upsert_workflow_checkpoint(UpsertWorkflowCheckpointInput {
            workflow_run_id,
            step_key: step_key.clone(),
            step_order: 0,
            status: status.to_string(),
            input_fingerprint_sha256: None,
            input_fingerprint_json: Some(input_json),
            checkpoint_json: None,
            artifacts_json,
            generation_run_ids_json: None,
            error_message,
            completed,
        })
        .await?;
        self.record_workflow_progress_event(WorkflowProgressEventInput {
            workflow_run_id,
            task_id: None,
            current: if status == WORKFLOW_STATUS_RUNNING {
                0
            } else {
                1
            },
            total: 1,
            message: &message,
            meta: None,
        })
        .await?;
        Ok(())
    }

    pub(crate) async fn finish_canvas_node_success(
        &self,
        lifecycle: &CanvasNodeLifecycle,
    ) -> Result<CanvasNodeRunSummary, AppError> {
        if lifecycle.run.finalize_on_success {
            self.complete_workflow_run_validated(lifecycle.run.workflow_run_id)
                .await?;
        }
        Ok(CanvasNodeRunSummary {
            workflow_run_id: lifecycle.run.workflow_run_id,
            project_id: lifecycle.project_id,
            graph_id: lifecycle.graph_id,
            mode: lifecycle.mode.clone(),
            status: WORKFLOW_STATUS_COMPLETED.to_string(),
            error_message: None,
        })
    }

    pub(crate) async fn finish_canvas_node_with_artifacts(
        &self,
        lifecycle: &CanvasNodeLifecycle,
        artifacts_json: Value,
    ) -> Result<CanvasNodeRunSummary, AppError> {
        self.record_canvas_node_checkpoint(
            lifecycle.run.workflow_run_id,
            lifecycle.step_key.clone(),
            WORKFLOW_STATUS_COMPLETED,
            lifecycle.input_json.clone(),
            Some(artifacts_json),
            None,
            true,
        )
        .await?;
        self.finish_canvas_node_success(lifecycle).await
    }

    pub(crate) async fn fail_canvas_node(
        &self,
        lifecycle: CanvasNodeLifecycle,
        primary: AppError,
    ) -> Result<CanvasNodeRunSummary, AppError> {
        let message = primary.to_string();
        let mut state_errors = Vec::new();
        if let Some(spec_id) = lifecycle.generation_spec_id
            && let Err(error) = self
                .mark_canvas_generation_spec_status_record(spec_id, WORKFLOW_STATUS_FAILED)
                .await
        {
            state_errors.push(format!("generation spec 失败状态写入失败: {error}"));
        }
        if let Err(error) = self
            .record_canvas_node_checkpoint(
                lifecycle.run.workflow_run_id,
                lifecycle.step_key,
                WORKFLOW_STATUS_FAILED,
                lifecycle.input_json,
                None,
                Some(message.clone()),
                true,
            )
            .await
        {
            state_errors.push(format!("节点失败 checkpoint 写入失败: {error}"));
        }
        let failed = match self
            .fail_workflow_run_validated(lifecycle.run.workflow_run_id, message.as_str())
            .await
        {
            Ok(run) => Some(run),
            Err(error) => {
                state_errors.push(format!("workflow 失败状态写入失败: {error}"));
                None
            }
        };
        if !state_errors.is_empty() {
            return Err(AppError::Internal(anyhow::anyhow!(
                "{message}；{}",
                state_errors.join("；")
            )));
        }
        let failed = failed.expect("state_errors 为空时 workflow failure 必须存在");
        Ok(CanvasNodeRunSummary {
            workflow_run_id: failed.id,
            project_id: lifecycle.project_id,
            graph_id: lifecycle.graph_id,
            mode: lifecycle.mode,
            status: failed.status,
            error_message: Some(message),
        })
    }

    pub(crate) async fn finish_canvas_node_error(
        &self,
        lifecycle: CanvasNodeLifecycle,
        primary: AppError,
    ) -> Result<CanvasNodeRunSummary, AppError> {
        let cancellation_reason = self
            .canvas_workflow_cancellation_reason(lifecycle.run.workflow_run_id)
            .await
            .map_err(|status_error| {
                AppError::Internal(anyhow::anyhow!(
                    "{primary}；画布 workflow 取消状态复验失败: {status_error}"
                ))
            })?;
        if let Some(message) = cancellation_reason {
            if let Some(spec_id) = lifecycle.generation_spec_id {
                self.mark_canvas_generation_spec_status_record(spec_id, WORKFLOW_STATUS_CANCELLED)
                    .await
                    .map_err(|error| {
                        AppError::Internal(anyhow::anyhow!(
                            "{message}；generation spec 取消状态写入失败: {error}"
                        ))
                    })?;
            }
            return Ok(CanvasNodeRunSummary {
                workflow_run_id: lifecycle.run.workflow_run_id,
                project_id: lifecycle.project_id,
                graph_id: lifecycle.graph_id,
                mode: lifecycle.mode,
                status: WORKFLOW_STATUS_CANCELLED.to_string(),
                error_message: Some(message),
            });
        }
        self.fail_canvas_node(lifecycle, primary).await
    }

    pub(crate) async fn canvas_workflow_cancellation_reason(
        &self,
        workflow_run_id: i64,
    ) -> Result<Option<String>, AppError> {
        let run = self.workflow_run_model_by_id(workflow_run_id).await?;
        if run.status != WORKFLOW_STATUS_CANCELLED {
            return Ok(None);
        }
        let reason = run
            .error_message
            .filter(|reason| !reason.trim().is_empty())
            .ok_or_else(|| AppError::InvalidState {
                current: format!("cancelled_workflow_without_reason:{workflow_run_id}"),
                operation: "读取画布 workflow 取消原因".to_string(),
            })?;
        Ok(Some(reason))
    }
}
