use super::super::prelude::{AppState, chapter, json};
use super::super::{
    CHAPTER_ANALYSIS_PROGRESS_TOTAL, CHAPTER_ANALYSIS_STEP_SYNC_OUTPUTS, ChapterAnalysisProgress,
    completed_chapter_analysis_phase, mark_chapter_analysis_current_step,
    report_chapter_analysis_progress, require_completed_chapter_analysis_phase,
    save_completed_chapter_analysis_phase, sync_chapter_analysis_outputs,
};

pub(super) async fn sync_analysis_outputs_phase(
    state: &AppState,
    ch: &chapter::Model,
    workflow_run_id: Option<i64>,
    rag_trace_id: String,
    progress: Option<&ChapterAnalysisProgress<'_>>,
) -> Result<(), String> {
    if let Some(checkpoint) = completed_chapter_analysis_phase(
        state,
        workflow_run_id,
        ch,
        CHAPTER_ANALYSIS_STEP_SYNC_OUTPUTS,
    )
    .await?
    {
        require_completed_chapter_analysis_phase(&checkpoint, CHAPTER_ANALYSIS_STEP_SYNC_OUTPUTS)
            .map_err(|error| error.to_string())?;
        report_chapter_analysis_progress(
            progress,
            58,
            CHAPTER_ANALYSIS_PROGRESS_TOTAL,
            "章节分析输出同步已从断点恢复",
            "章节结构化分析已停止",
        )?;
        return Ok(());
    }

    mark_chapter_analysis_current_step(state, workflow_run_id, CHAPTER_ANALYSIS_STEP_SYNC_OUTPUTS)
        .await?;
    crate::sidecar_clients::scope_rag_operation_trace(
        rag_trace_id,
        sync_chapter_analysis_outputs(state, ch),
    )
    .await?;
    save_completed_chapter_analysis_phase(
        state,
        workflow_run_id,
        ch,
        CHAPTER_ANALYSIS_STEP_SYNC_OUTPUTS,
        10,
        json!({ "completed": true }),
    )
    .await
}
