use super::super::prelude::{AppState, chapter, json};
use super::super::{
    CHAPTER_ANALYSIS_PROGRESS_TOTAL, CHAPTER_ANALYSIS_STEP_REFRESH_STATUS, ChapterAnalysisProgress,
    completed_chapter_analysis_phase, mark_chapter_analysis_current_step,
    report_chapter_analysis_progress, require_completed_chapter_analysis_phase,
    save_completed_chapter_analysis_phase,
};

pub(super) async fn refresh_project_status_phase(
    state: &AppState,
    ch: &chapter::Model,
    workflow_run_id: Option<i64>,
    progress: Option<&ChapterAnalysisProgress<'_>>,
) -> Result<(), String> {
    if let Some(checkpoint) = completed_chapter_analysis_phase(
        state,
        workflow_run_id,
        ch,
        CHAPTER_ANALYSIS_STEP_REFRESH_STATUS,
    )
    .await?
    {
        require_completed_chapter_analysis_phase(&checkpoint, CHAPTER_ANALYSIS_STEP_REFRESH_STATUS)
            .map_err(|error| error.to_string())?;
        report_chapter_analysis_progress(
            progress,
            99,
            CHAPTER_ANALYSIS_PROGRESS_TOTAL,
            "项目进度状态刷新已从断点恢复",
            "章节结构化分析已停止",
        )?;
        return Ok(());
    }

    report_chapter_analysis_progress(
        progress,
        97,
        CHAPTER_ANALYSIS_PROGRESS_TOTAL,
        "正在刷新项目进度状态",
        "章节结构化分析已停止",
    )?;
    mark_chapter_analysis_current_step(
        state,
        workflow_run_id,
        CHAPTER_ANALYSIS_STEP_REFRESH_STATUS,
    )
    .await?;
    state
        .orchestrator
        .refresh_project_current_chapter(ch.project_id)
        .await
        .map_err(|err| err.to_string())?;
    state
        .orchestrator
        .sync_project_status_from_artifacts(ch.project_id)
        .await
        .map_err(|err| err.to_string())?;
    save_completed_chapter_analysis_phase(
        state,
        workflow_run_id,
        ch,
        CHAPTER_ANALYSIS_STEP_REFRESH_STATUS,
        60,
        json!({ "completed": true }),
    )
    .await
}
