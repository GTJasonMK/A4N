use serde_json::{Value, json};

use super::super::prelude::{AppState, chapter};
use super::super::{
    CHAPTER_ANALYSIS_PROGRESS_TOTAL, CHAPTER_ANALYSIS_STEP_CHARACTER_SYNC,
    CHAPTER_ANALYSIS_STEP_SCENE_SYNC, ChapterAnalysisProgress, completed_chapter_analysis_phase,
    ensure_chapter_analysis_character_sync_completed, mark_chapter_analysis_current_step,
    report_chapter_analysis_progress, report_chapter_subprogress,
    require_completed_chapter_analysis_phase, save_completed_chapter_analysis_phase,
};

pub(super) async fn sync_scene_profiles_phase(
    state: &AppState,
    user_id: i64,
    ch: &chapter::Model,
    workflow_run_id: Option<i64>,
    progress: Option<&ChapterAnalysisProgress<'_>>,
) -> Result<(), String> {
    if let Some(checkpoint) = completed_chapter_analysis_phase(
        state,
        workflow_run_id,
        ch,
        CHAPTER_ANALYSIS_STEP_SCENE_SYNC,
    )
    .await?
    {
        require_completed_chapter_analysis_phase(&checkpoint, CHAPTER_ANALYSIS_STEP_SCENE_SYNC)
            .map_err(|error| error.to_string())?;
        report_chapter_analysis_progress(
            progress,
            88,
            CHAPTER_ANALYSIS_PROGRESS_TOTAL,
            "场景档案同步已从断点恢复",
            "章节结构化分析已停止",
        )?;
        return Ok(());
    }

    report_chapter_analysis_progress(
        progress,
        78,
        CHAPTER_ANALYSIS_PROGRESS_TOTAL,
        "正在同步本章场景档案和场景内状态",
        "章节结构化分析已停止",
    )?;
    mark_chapter_analysis_current_step(state, workflow_run_id, CHAPTER_ANALYSIS_STEP_SCENE_SYNC)
        .await?;
    let scene_reporter = |current: u32, total: u32, message: &str| {
        report_chapter_subprogress(progress, current, total, 78, 88, message);
    };
    let scene_progress = progress.map(|progress| {
        crate::actions::scene_profile_sync::SceneProfileSyncProgress::new(
            Some(&scene_reporter),
            progress.cancel_token,
        )
    });
    crate::actions::scene_profile_sync::sync_scene_profiles_for_chapters(
        state,
        user_id,
        ch.project_id,
        Some(ch.chapter_number),
        scene_progress.as_ref(),
    )
    .await
    .map_err(|err| format!("分析后场景档案同步失败: {err}"))?;
    save_completed_chapter_analysis_phase(
        state,
        workflow_run_id,
        ch,
        CHAPTER_ANALYSIS_STEP_SCENE_SYNC,
        40,
        json!({ "completed": true }),
    )
    .await
}

pub(super) async fn sync_character_profiles_phase(
    state: &AppState,
    user_id: i64,
    ch: &chapter::Model,
    workflow_run_id: Option<i64>,
    discovery_result: &Value,
    progress: Option<&ChapterAnalysisProgress<'_>>,
) -> Result<(), String> {
    if let Some(checkpoint) = completed_chapter_analysis_phase(
        state,
        workflow_run_id,
        ch,
        CHAPTER_ANALYSIS_STEP_CHARACTER_SYNC,
    )
    .await?
    {
        require_completed_chapter_analysis_phase(&checkpoint, CHAPTER_ANALYSIS_STEP_CHARACTER_SYNC)
            .map_err(|error| error.to_string())?;
        report_chapter_analysis_progress(
            progress,
            97,
            CHAPTER_ANALYSIS_PROGRESS_TOTAL,
            "角色档案同步已从断点恢复",
            "章节结构化分析已停止",
        )?;
        return Ok(());
    }

    report_chapter_analysis_progress(
        progress,
        88,
        CHAPTER_ANALYSIS_PROGRESS_TOTAL,
        "正在同步角色档案、事件和状态索引",
        "章节结构化分析已停止",
    )?;
    mark_chapter_analysis_current_step(
        state,
        workflow_run_id,
        CHAPTER_ANALYSIS_STEP_CHARACTER_SYNC,
    )
    .await?;
    let character_sync_reporter = |current: u32, total: u32, message: &str| {
        report_chapter_subprogress(progress, current, total, 88, 97, message);
    };
    let character_progress = progress.map(|progress| {
        crate::actions::character_ai::CharacterAiProgress::new(
            Some(&character_sync_reporter),
            progress.cancel_token,
        )
    });
    let sync_result = crate::actions::character_ai::sync_analysis_character_profiles_to_chapter(
        state,
        user_id,
        ch.project_id,
        ch.chapter_number,
        ch.analysis_data.as_ref(),
        Some(discovery_result),
        character_progress.as_ref(),
    )
    .await
    .map_err(|err| format!("分析后角色档案同步失败: {err}"))?;
    ensure_chapter_analysis_character_sync_completed(&sync_result)
        .map_err(|error| error.to_string())?;
    save_completed_chapter_analysis_phase(
        state,
        workflow_run_id,
        ch,
        CHAPTER_ANALYSIS_STEP_CHARACTER_SYNC,
        50,
        json!({ "completed": true, "result": sync_result }),
    )
    .await
}
