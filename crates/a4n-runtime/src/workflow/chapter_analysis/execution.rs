use super::prelude::*;
use super::*;
use a4n_orchestrator::chapter_analysis::{
    ChapterAnalysisInput, chapter_analysis_workflow_run_input,
    ensure_previous_chapter_analysis_complete_from_rows,
};

pub async fn generate_and_sync_chapter_analysis_with_progress(
    state: &AppState,
    user_id: i64,
    chapter_id: i64,
    progress: Option<&ChapterAnalysisProgress<'_>>,
) -> Result<chapter::Model, String> {
    report_chapter_analysis_progress(
        progress,
        10,
        CHAPTER_ANALYSIS_PROGRESS_TOTAL,
        "正在读取章节并校验角色同步前置条件",
        "章节结构化分析已停止",
    )?;
    let ch = load_chapter_by_id(
        state,
        chapter_id,
        "章节结构化分析失败：查询章节失败",
        format!("章节结构化分析失败：章节不存在: chapter_id={chapter_id}"),
    )
    .await?;
    access::verify_project_editor(state, user_id, ch.project_id)
        .await
        .map_err(|err| err.to_string())?;
    ensure_previous_chapter_analysis_complete(state, &ch, "生成结构化分析").await?;
    crate::actions::character_ai::preflight_analysis_character_profile_sync_to_chapter(
        state,
        ch.project_id,
        ch.chapter_number,
    )
    .await?;
    let workflow_run = state
        .orchestrator
        .upsert_workflow_run(chapter_analysis_workflow_run_input(
            &ch,
            CHAPTER_ANALYSIS_STEP_FACTS,
        ))
        .await
        .map_err(|err| format!("创建章节分析工作流失败: {err}"))?;

    report_chapter_analysis_progress(
        progress,
        20,
        CHAPTER_ANALYSIS_PROGRESS_TOTAL,
        "正在调用模型生成结构化分析",
        "章节结构化分析已停止",
    )?;
    let prompt_renderer = RuntimePromptRenderer::new(state);
    let analysis_result = if let Some(progress_state) = progress {
        let cancel_token = progress_state.cancel_token;
        let mut analysis_progress = |current: u32, total: u32, message: &str| {
            report_chapter_subprogress(progress, current, total, 20, 45, message);
            Ok::<(), AppError>(())
        };
        state
            .orchestrator
            .analyze_and_store_chapter(ChapterAnalysisInput {
                chapter_id,
                prompt_renderer: &prompt_renderer,
                progress: Some(&mut analysis_progress),
                cancel_token,
                workflow_run_id: Some(workflow_run.id),
            })
            .await
    } else {
        state
            .orchestrator
            .analyze_and_store_chapter(ChapterAnalysisInput {
                chapter_id,
                prompt_renderer: &prompt_renderer,
                progress: None,
                cancel_token: None,
                workflow_run_id: Some(workflow_run.id),
            })
            .await
    };
    if let Err(err) = analysis_result {
        let err_text = format!("章节结构化分析失败: {err}");
        state
            .orchestrator
            .mark_workflow_status(
                workflow_run.id,
                WORKFLOW_STATUS_FAILED,
                true,
                Some(&err_text),
            )
            .await
            .map_err(|status_err| {
                format!("{err_text}; 同步章节分析工作流状态失败: {status_err}")
            })?;
        return Err(err_text);
    }

    report_chapter_analysis_progress(
        progress,
        45,
        CHAPTER_ANALYSIS_PROGRESS_TOTAL,
        "结构化分析已保存，正在读取章节",
        "章节结构化分析已停止",
    )?;
    let updated_chapter = load_chapter_by_id(
        state,
        chapter_id,
        "章节结构化分析失败：查询章节失败",
        format!("章节结构化分析失败：章节不存在: chapter_id={chapter_id}"),
    )
    .await?;

    if let Err(err) = sync_project_state_after_chapter_analysis_with_progress(
        state,
        user_id,
        &updated_chapter,
        workflow_run.id,
        progress,
    )
    .await
    {
        state
            .orchestrator
            .mark_workflow_status(workflow_run.id, WORKFLOW_STATUS_FAILED, true, Some(&err))
            .await
            .map_err(|status_err| format!("{err}; 同步章节分析工作流状态失败: {status_err}"))?;
        return Err(err);
    }
    state
        .orchestrator
        .mark_workflow_status(workflow_run.id, WORKFLOW_STATUS_COMPLETED, false, None)
        .await
        .map_err(|err| format!("同步章节分析工作流完成状态失败: {err}"))?;
    Ok(updated_chapter)
}

pub(super) async fn load_chapter_by_id(
    state: &AppState,
    chapter_id: i64,
    query_error_prefix: &str,
    missing_error: String,
) -> Result<chapter::Model, String> {
    state
        .orchestrator
        .chapter_by_id(chapter_id)
        .await
        .map_err(|err| format!("{query_error_prefix}: {}", app_error_message(err)))?
        .ok_or(missing_error)
}

pub async fn ensure_previous_chapter_analysis_complete(
    state: &AppState,
    ch: &chapter::Model,
    operation: &str,
) -> Result<(), String> {
    let previous = state
        .orchestrator
        .chapters_before_number(ch.project_id, ch.chapter_number)
        .await
        .map_err(|err| {
            format!(
                "{operation}前序章节分析顺序校验失败：查询章节失败: {}",
                app_error_message(err)
            )
        })?;
    ensure_previous_chapter_analysis_complete_from_rows(operation, ch.chapter_number, &previous)
        .map_err(|error| error.to_string())
}
