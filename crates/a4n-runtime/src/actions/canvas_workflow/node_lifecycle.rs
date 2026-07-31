use a4n_orchestrator::canvas::{CanvasNodeRunHandle, CanvasNodeRunSummary};

use super::CanvasRunSummary;

#[derive(Debug, Clone, Copy)]
pub(in crate::actions::canvas_workflow) struct CanvasRunHandle {
    pub(in crate::actions::canvas_workflow) workflow_run_id: i64,
    pub(in crate::actions::canvas_workflow) finalize_on_success: bool,
}

impl From<CanvasRunHandle> for CanvasNodeRunHandle {
    fn from(run: CanvasRunHandle) -> Self {
        Self {
            workflow_run_id: run.workflow_run_id,
            finalize_on_success: run.finalize_on_success,
        }
    }
}

pub(in crate::actions::canvas_workflow) fn runtime_summary(
    summary: CanvasNodeRunSummary,
) -> CanvasRunSummary {
    CanvasRunSummary {
        workflow_run_id: summary.workflow_run_id,
        project_id: summary.project_id,
        graph_id: summary.graph_id,
        mode: summary.mode,
        status: summary.status,
        error_message: summary.error_message,
    }
}
