use a4n_db::canvas::validate_canvas_record_document_edge_contract;

use crate::{
    OrchestratorContext,
    canvas::{CanvasNodeRunHandle, CanvasNodeRunSummary},
    workflow::{
        WORKFLOW_STATUS_CANCELLED, WORKFLOW_STATUS_COMPLETED, WORKFLOW_STATUS_FAILED,
        WORKFLOW_STATUS_RUNNING,
    },
};

use super::contracts::{
    CanvasGraphExecutionHostPort, CanvasGraphNodeExecutionInput, CanvasGraphNodeExecutionKind,
    PreparedCanvasGraphRun,
};

impl OrchestratorContext {
    pub async fn run_prepared_canvas_graph_validated(
        &self,
        prepared: PreparedCanvasGraphRun,
        host_port: &dyn CanvasGraphExecutionHostPort,
    ) -> Result<CanvasNodeRunSummary, String> {
        match self
            .execute_prepared_canvas_graph(&prepared, host_port)
            .await
        {
            Ok(summary) => Ok(summary),
            Err(primary) => {
                let cancellation_reason = self
                    .canvas_workflow_cancellation_reason(prepared.workflow_run_id)
                    .await
                    .map_err(|status_error| {
                        format!("{primary}；画布 workflow 取消状态复验失败: {status_error}")
                    })?;
                if let Some(reason) = cancellation_reason {
                    return Ok(cancelled_canvas_graph_summary(&prepared, reason));
                }
                match self
                    .fail_workflow_run_validated(prepared.workflow_run_id, Some(primary.as_str()))
                    .await
                {
                    Ok(failed) => Ok(CanvasNodeRunSummary {
                        workflow_run_id: failed.id,
                        project_id: prepared.project_id,
                        graph_id: prepared.graph_id,
                        mode: prepared.mode,
                        status: failed.status,
                        error_message: Some(primary),
                    }),
                    Err(mark_error) => Err(format!(
                        "{primary}；画布 workflow 失败状态写入失败: {mark_error}"
                    )),
                }
            }
        }
    }

    async fn execute_prepared_canvas_graph(
        &self,
        prepared: &PreparedCanvasGraphRun,
        host_port: &dyn CanvasGraphExecutionHostPort,
    ) -> Result<CanvasNodeRunSummary, String> {
        let shared_run = CanvasNodeRunHandle {
            workflow_run_id: prepared.workflow_run_id,
            finalize_on_success: false,
        };
        let mut last_summary = None;
        for node_id in prepared.runnable_node_ids.iter().copied() {
            if let Some(reason) = self
                .canvas_workflow_cancellation_reason(prepared.workflow_run_id)
                .await
                .map_err(|error| error.to_string())?
            {
                return Ok(cancelled_canvas_graph_summary(prepared, reason));
            }
            let current_document = self
                .get_or_create_canvas_graph_record(
                    prepared.project_id,
                    Some(prepared.graph_id),
                    chrono::Utc::now().naive_utc(),
                )
                .await
                .map_err(|error| error.to_string())?;
            validate_canvas_record_document_edge_contract(&current_document)
                .map_err(|error| error.to_string())?;
            let node = current_document
                .nodes
                .iter()
                .find(|node| node.id == node_id)
                .ok_or_else(|| format!("画布节点不存在: {node_id}"))?;
            let kind = CanvasGraphNodeExecutionKind::from_node_kind(&node.node_kind)?;
            let summary = host_port
                .execute_node(CanvasGraphNodeExecutionInput {
                    kind,
                    user_id: prepared.user_id,
                    project_id: prepared.project_id,
                    graph_id: prepared.graph_id,
                    node,
                    document: &current_document,
                    shared_run,
                    mode: &prepared.mode,
                })
                .await?;
            let should_stop = matches!(
                summary.status.as_str(),
                WORKFLOW_STATUS_FAILED | WORKFLOW_STATUS_RUNNING | WORKFLOW_STATUS_CANCELLED
            );
            last_summary = Some(summary);
            if should_stop {
                break;
            }
        }

        let Some(summary) = last_summary else {
            return Err("画布中没有可执行的操作节点".to_string());
        };
        if let Some(reason) = self
            .canvas_workflow_cancellation_reason(prepared.workflow_run_id)
            .await
            .map_err(|error| error.to_string())?
        {
            return Ok(cancelled_canvas_graph_summary(prepared, reason));
        }
        if summary.status == WORKFLOW_STATUS_COMPLETED {
            let completed = self
                .complete_canvas_graph_workflow_validated(prepared.workflow_run_id)
                .await
                .map_err(|error| error.to_string())?;
            return Ok(CanvasNodeRunSummary {
                workflow_run_id: completed.id,
                project_id: prepared.project_id,
                graph_id: prepared.graph_id,
                mode: prepared.mode.clone(),
                status: completed.status,
                error_message: None,
            });
        }
        Ok(summary)
    }
}

fn cancelled_canvas_graph_summary(
    prepared: &PreparedCanvasGraphRun,
    reason: String,
) -> CanvasNodeRunSummary {
    prepared.summary(WORKFLOW_STATUS_CANCELLED, Some(reason))
}
