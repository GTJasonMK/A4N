use crate::{
    AppState,
    actions::{app_error_message, canvas_workflow, video_generation},
};
use a4n_orchestrator::workflow::{UnifiedResumeDispatch, plan_unified_resume};

use super::super::{
    ResumeWorkflowRunInput, WorkflowRunSummary, access::verify_workflow_run_access,
};

pub async fn resume_workflow_run_for_user(
    state: &AppState,
    user_id: i64,
    input: ResumeWorkflowRunInput,
) -> Result<WorkflowRunSummary, String> {
    let run = state
        .workflow()
        .workflow_run_model_by_id(input.workflow_run_id)
        .await
        .map_err(|err| err.to_string())?;
    verify_workflow_run_access(state, user_id, &run).await?;
    let dispatch = plan_unified_resume(
        run.can_resume,
        run.resume_block_reason.as_deref(),
        &run.workflow_kind,
        run.scope_table.as_deref(),
        run.scope_id,
    )?;
    match dispatch {
        UnifiedResumeDispatch::VideoProviderJob { video_id } => {
            video_generation::poll_video_job(state, user_id, video_id).await?;
            workflow_run_summary(state, input.workflow_run_id).await
        }
        UnifiedResumeDispatch::VideoCanvasGraph => {
            let summary =
                canvas_workflow::resume_canvas_graph_run(state, user_id, input.workflow_run_id)
                    .await?;
            workflow_run_summary(state, summary.workflow_run_id).await
        }
        UnifiedResumeDispatch::ChapterAnalysis => {
            crate::workflow::resume_chapter_analysis_workflow_run(state, user_id, &run).await?;
            workflow_run_summary(state, input.workflow_run_id).await
        }
    }
}

async fn workflow_run_summary(
    state: &AppState,
    workflow_run_id: i64,
) -> Result<WorkflowRunSummary, String> {
    state
        .workflow()
        .workflow_run_summary(workflow_run_id)
        .await
        .map_err(app_error_message)
}
