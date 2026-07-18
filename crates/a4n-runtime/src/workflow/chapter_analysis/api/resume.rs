use super::super::{
    CHAPTER_ANALYSIS_PROGRESS_TOTAL, CHAPTER_ANALYSIS_STEP_DERIVED_SYNC, ChapterAnalysisProgress,
    ensure_chapter_not_generating, generate_and_sync_chapter_analysis_with_progress,
    load_chapter_by_id, preflight_chapter_analysis, report_chapter_analysis_progress,
    sync_project_state_after_chapter_analysis_with_progress,
};
use crate::actions::access;
use crate::state::AppState;
use a4n_core::models::{chapter::chapter, generation::workflow_run};
use a4n_orchestrator::{
    chapter_analysis::{chapter_analysis_run_input_matches, chapter_analysis_workflow_run_input},
    workflow::{WORKFLOW_STATUS_COMPLETED, WORKFLOW_STATUS_FAILED},
};

pub async fn preflight_resume_chapter_analysis_sync(
    state: &AppState,
    user_id: i64,
    chapter_id: i64,
) -> Result<chapter::Model, String> {
    preflight_chapter_analysis(
        state,
        user_id,
        chapter_id,
        "继续同步结构化分析",
        "章节正文为空，不能继续同步结构化分析",
        true,
    )
    .await
}

pub async fn resume_chapter_analysis_sync_with_progress(
    state: &AppState,
    user_id: i64,
    chapter_id: i64,
    progress: Option<&ChapterAnalysisProgress<'_>>,
) -> Result<chapter::Model, String> {
    report_chapter_analysis_progress(
        progress,
        0,
        CHAPTER_ANALYSIS_PROGRESS_TOTAL,
        "正在检查已保存的章节分析",
        "章节分析继续同步已停止",
    )?;
    let ch = preflight_resume_chapter_analysis_sync(state, user_id, chapter_id).await?;
    let workflow_run = state
        .orchestrator
        .upsert_workflow_run(chapter_analysis_workflow_run_input(
            &ch,
            CHAPTER_ANALYSIS_STEP_DERIVED_SYNC,
        ))
        .await
        .map_err(|err| format!("创建章节分析继续同步工作流失败: {err}"))?;

    report_chapter_analysis_progress(
        progress,
        45,
        CHAPTER_ANALYSIS_PROGRESS_TOTAL,
        "已找到结构化分析，正在继续同步派生数据",
        "章节分析继续同步已停止",
    )?;
    if let Err(err) = sync_project_state_after_chapter_analysis_with_progress(
        state,
        user_id,
        &ch,
        workflow_run.id,
        progress,
    )
    .await
    {
        state
            .orchestrator
            .mark_workflow_status(workflow_run.id, WORKFLOW_STATUS_FAILED, true, Some(&err))
            .await
            .map_err(|status_err| {
                format!("{err}; 同步章节分析继续同步工作流状态失败: {status_err}")
            })?;
        return Err(err);
    }
    state
        .orchestrator
        .mark_workflow_status(workflow_run.id, WORKFLOW_STATUS_COMPLETED, false, None)
        .await
        .map_err(|err| format!("同步章节分析继续同步完成状态失败: {err}"))?;

    load_chapter_by_id(
        state,
        chapter_id,
        "继续同步后刷新章节失败",
        format!("继续同步后刷新章节失败：章节不存在: chapter_id={chapter_id}"),
    )
    .await
}

pub async fn resume_chapter_analysis_workflow_run(
    state: &AppState,
    user_id: i64,
    run: &workflow_run::Model,
) -> Result<(), String> {
    if run.workflow_kind != "chapter_analysis" {
        return Err(format!(
            "工作流类型 {} 不是章节结构化分析",
            run.workflow_kind
        ));
    }
    if !run.can_resume {
        return Err(run
            .resume_block_reason
            .as_deref()
            .unwrap_or("该章节结构化分析工作流当前不可恢复")
            .to_string());
    }

    let chapter_id = run
        .chapter_id
        .ok_or_else(|| "章节结构化分析工作流缺少 chapter_id".to_string())?;
    let ch = load_chapter_by_id(
        state,
        chapter_id,
        "恢复章节结构化分析失败：查询章节失败",
        format!("恢复章节结构化分析失败：章节不存在: chapter_id={chapter_id}"),
    )
    .await?;
    access::verify_project_editor(state, user_id, ch.project_id)
        .await
        .map_err(|err| err.to_string())?;
    ensure_chapter_not_generating(&ch, "恢复结构化分析")?;
    if !chapter_analysis_run_input_matches(&run.input_fingerprint_json, &ch) {
        return Err(
            "章节正文、标题或章节归属已变化，不能从旧的章节结构化分析断点恢复；请重新生成结构化分析"
                .to_string(),
        );
    }

    if ch.analysis_data.is_some() {
        resume_chapter_analysis_sync_with_progress(state, user_id, ch.id, None).await?;
    } else {
        generate_and_sync_chapter_analysis_with_progress(state, user_id, ch.id, None).await?;
    }
    Ok(())
}
