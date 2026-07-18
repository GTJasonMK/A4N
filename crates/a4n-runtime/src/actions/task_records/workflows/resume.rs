use crate::{
    AppState,
    actions::{app_error_message, canvas_workflow, video_generation},
};

use super::super::{
    ResumeWorkflowRunInput, WorkflowRunSummary, access::verify_workflow_run_access,
};

pub async fn resume_workflow_run_for_user(
    state: &AppState,
    user_id: i64,
    input: ResumeWorkflowRunInput,
) -> Result<WorkflowRunSummary, String> {
    let run = state
        .orchestrator
        .workflow_run_model_by_id(input.workflow_run_id)
        .await
        .map_err(|err| err.to_string())?;
    verify_workflow_run_access(state, user_id, &run).await?;
    if !run.can_resume {
        return Err(run
            .resume_block_reason
            .unwrap_or_else(|| "该工作流当前不可恢复".to_string()));
    }
    match run.workflow_kind.as_str() {
        "video_provider_job" => {
            let video_id = run
                .scope_id
                .filter(|_| run.scope_table.as_deref() == Some("generated_video"))
                .ok_or_else(|| "视频 provider 工作流缺少 generated_video 目标".to_string())?;
            video_generation::poll_video_job(state, user_id, video_id).await?;
            workflow_run_summary(state, input.workflow_run_id).await
        }
        "video_canvas_graph" => {
            let summary =
                canvas_workflow::resume_canvas_graph_run(state, user_id, input.workflow_run_id)
                    .await?;
            workflow_run_summary(state, summary.workflow_run_id).await
        }
        "chapter_analysis" => {
            crate::workflow::resume_chapter_analysis_workflow_run(state, user_id, &run).await?;
            workflow_run_summary(state, input.workflow_run_id).await
        }
        "manga_pipeline" => Err("漫画流水线请在漫画章节页使用“续传流水线”入口恢复".to_string()),
        other => Err(format!("工作流类型 {other} 暂未接入统一恢复入口")),
    }
}

async fn workflow_run_summary(
    state: &AppState,
    workflow_run_id: i64,
) -> Result<WorkflowRunSummary, String> {
    state
        .orchestrator
        .workflow_run_summary(workflow_run_id)
        .await
        .map_err(app_error_message)
}
