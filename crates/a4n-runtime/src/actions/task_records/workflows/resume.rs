use crate::{
    AppState,
    actions::{blueprint_records, canvas_workflow, chapter_content, video_generation},
};
use a4n_api_contract::workflow::{ResumeWorkflowRunInput, WorkflowRunSummary};
use a4n_domain::workflow::require_positive_workflow_run_id;
use a4n_orchestrator::workflow::{UnifiedResumeDispatch, plan_unified_resume};

use super::super::access::verify_workflow_run_access;

pub async fn resume_workflow_run_for_user(
    state: &AppState,
    user_id: i64,
    input: ResumeWorkflowRunInput,
) -> Result<WorkflowRunSummary, String> {
    require_positive_workflow_run_id(input.workflow_run_id)?;
    let run = state
        .workflow_task_lookup_application()
        .workflow_run_for_resume(input.workflow_run_id)
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
                .workflow_cancellation_application()
                .mark_running(input.workflow_run_id)
                .await
                .map_err(|err| err.to_string())?;
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
            crate::workflow::resume_chapter_analysis_workflow_run(state, user_id, run.id).await?;
            load_workflow_run_summary(state, input.workflow_run_id).await
        }
        UnifiedResumeDispatch::ChapterContentSync { chapter_id } => {
            if run.chapter_id != Some(chapter_id) {
                return Err("章节资料同步工作流 chapter_id 与作用域不一致".to_string());
            }
            chapter_content::resume_chapter_content_sync_workflow_run(state, run.id).await?;
            load_workflow_run_summary(state, input.workflow_run_id).await
        }
        UnifiedResumeDispatch::BlueprintGeneration => {
            blueprint_records::resume_blueprint_generation_workflow(state, user_id, &run).await?;
            load_workflow_run_summary(state, input.workflow_run_id).await
        }
        UnifiedResumeDispatch::BlueprintRefinement => {
            blueprint_records::resume_blueprint_refinement_workflow(state, user_id, &run).await?;
            load_workflow_run_summary(state, input.workflow_run_id).await
        }
    }
}

async fn load_workflow_run_summary(
    state: &AppState,
    workflow_run_id: i64,
) -> Result<WorkflowRunSummary, String> {
    state
        .workflow_task_lookup_application()
        .workflow_run_summary(workflow_run_id)
        .await
        .map_err(|err| err.to_string())
}
