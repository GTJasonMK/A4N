use crate::{
    AppState,
    actions::{app_error_message, canvas_workflow, chapter_content, video_generation},
};
use a4n_api_contract::workflow::{ResumeWorkflowRunInput, WorkflowRunSummary};
use a4n_orchestrator::workflow::{
    UnifiedResumeDispatch, plan_unified_resume, require_positive_workflow_run_id,
};

use super::super::{
    access::verify_workflow_run_access,
    projections::workflow_run_summary as project_workflow_run_summary,
};

pub async fn resume_workflow_run_for_user(
    state: &AppState,
    user_id: i64,
    input: ResumeWorkflowRunInput,
) -> Result<WorkflowRunSummary, String> {
    require_positive_workflow_run_id(input.workflow_run_id)?;
    let run = state
        .orchestrator()
        .workflow_run_model_by_id(input.workflow_run_id)
        .await
        .map_err(|err| err.to_string())?;
    verify_workflow_run_access(state, user_id, &run).await?;
    let dispatch = plan_unified_resume(
        &run.status,
        run.can_resume,
        run.resume_block_reason.as_deref(),
        &run.workflow_kind,
        run.scope_table.as_deref(),
        run.scope_id,
    )?;
    match dispatch {
        UnifiedResumeDispatch::VideoProviderJob { video_id } => {
            state
                .orchestrator()
                .mark_workflow_running_validated(input.workflow_run_id)
                .await
                .map_err(app_error_message)?;
            video_generation::poll_video_job(state, user_id, video_id).await?;
            load_workflow_run_summary(state, input.workflow_run_id).await
        }
        UnifiedResumeDispatch::VideoCanvasGraph => {
            let summary =
                canvas_workflow::resume_canvas_graph_run(state, user_id, input.workflow_run_id)
                    .await?;
            load_workflow_run_summary(state, summary.workflow_run_id).await
        }
        UnifiedResumeDispatch::ChapterAnalysis => {
            crate::workflow::resume_chapter_analysis_workflow_run(state, user_id, &run).await?;
            load_workflow_run_summary(state, input.workflow_run_id).await
        }
        UnifiedResumeDispatch::ChapterContentSync { chapter_id } => {
            if run.chapter_id != Some(chapter_id) {
                return Err("章节资料同步工作流 chapter_id 与作用域不一致".to_string());
            }
            chapter_content::resume_chapter_content_sync_workflow_run(state, &run).await?;
            load_workflow_run_summary(state, input.workflow_run_id).await
        }
    }
}

async fn load_workflow_run_summary(
    state: &AppState,
    workflow_run_id: i64,
) -> Result<WorkflowRunSummary, String> {
    state
        .orchestrator()
        .workflow_run_summary(workflow_run_id)
        .await
        .map_err(app_error_message)
        .map(project_workflow_run_summary)
}
