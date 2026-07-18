use serde_json::Value;

use super::super::prelude::{AppState, chapter, json, required_checkpoint_field};
use super::super::{
    CHAPTER_ANALYSIS_PROGRESS_TOTAL, CHAPTER_ANALYSIS_STEP_CHARACTER_DISCOVERY,
    CHAPTER_ANALYSIS_STEP_OBJECT_DISCOVERY, ChapterAnalysisProgress,
    completed_chapter_analysis_phase, mark_chapter_analysis_current_step,
    report_chapter_analysis_progress, report_chapter_subprogress,
    require_completed_chapter_analysis_phase, save_completed_chapter_analysis_phase,
};

pub(super) async fn sync_object_discovery_phase(
    state: &AppState,
    ch: &chapter::Model,
    workflow_run_id: Option<i64>,
    progress: Option<&ChapterAnalysisProgress<'_>>,
) -> Result<(), String> {
    if let Some(checkpoint) = completed_chapter_analysis_phase(
        state,
        workflow_run_id,
        ch,
        CHAPTER_ANALYSIS_STEP_OBJECT_DISCOVERY,
    )
    .await?
    {
        require_completed_chapter_analysis_phase(
            &checkpoint,
            CHAPTER_ANALYSIS_STEP_OBJECT_DISCOVERY,
        )
        .map_err(|error| error.to_string())?;
        report_chapter_analysis_progress(
            progress,
            68,
            CHAPTER_ANALYSIS_PROGRESS_TOTAL,
            "物体档案发现已从断点恢复",
            "章节结构化分析已停止",
        )?;
        return Ok(());
    }

    report_chapter_analysis_progress(
        progress,
        58,
        CHAPTER_ANALYSIS_PROGRESS_TOTAL,
        "正在识别本章关键物体并同步物体档案",
        "章节结构化分析已停止",
    )?;
    mark_chapter_analysis_current_step(
        state,
        workflow_run_id,
        CHAPTER_ANALYSIS_STEP_OBJECT_DISCOVERY,
    )
    .await?;
    let object_reporter = |current: u32, total: u32, message: &str| {
        report_chapter_subprogress(progress, current, total, 58, 68, message);
    };
    let object_progress = progress.map(|progress| {
        crate::actions::object_discovery::ObjectDiscoveryProgress::new(
            Some(&object_reporter),
            progress.cancel_token,
        )
    });
    let object_result = crate::actions::object_discovery::discover_object_profiles_for_chapters(
        state,
        ch.project_id,
        Some(ch.chapter_number),
        object_progress.as_ref(),
    )
    .await
    .map_err(|err| format!("分析后物体档案发现失败: {err}"))?;
    save_completed_chapter_analysis_phase(
        state,
        workflow_run_id,
        ch,
        CHAPTER_ANALYSIS_STEP_OBJECT_DISCOVERY,
        20,
        json!({ "completed": true, "result": object_result }),
    )
    .await
}

pub(super) async fn sync_character_discovery_phase(
    state: &AppState,
    user_id: i64,
    ch: &chapter::Model,
    workflow_run_id: Option<i64>,
    progress: Option<&ChapterAnalysisProgress<'_>>,
) -> Result<Value, String> {
    if let Some(checkpoint) = completed_chapter_analysis_phase(
        state,
        workflow_run_id,
        ch,
        CHAPTER_ANALYSIS_STEP_CHARACTER_DISCOVERY,
    )
    .await?
    {
        require_completed_chapter_analysis_phase(
            &checkpoint,
            CHAPTER_ANALYSIS_STEP_CHARACTER_DISCOVERY,
        )
        .map_err(|error| error.to_string())?;
        report_chapter_analysis_progress(
            progress,
            78,
            CHAPTER_ANALYSIS_PROGRESS_TOTAL,
            "角色发现已从断点恢复",
            "章节结构化分析已停止",
        )?;
        return required_checkpoint_field(
            &checkpoint,
            CHAPTER_ANALYSIS_STEP_CHARACTER_DISCOVERY,
            "result",
        )
        .map_err(|error| error.to_string());
    }

    report_chapter_analysis_progress(
        progress,
        68,
        CHAPTER_ANALYSIS_PROGRESS_TOTAL,
        "正在发现本章新出场角色",
        "章节结构化分析已停止",
    )?;
    mark_chapter_analysis_current_step(
        state,
        workflow_run_id,
        CHAPTER_ANALYSIS_STEP_CHARACTER_DISCOVERY,
    )
    .await?;
    let character_discovery_reporter = |current: u32, total: u32, message: &str| {
        report_chapter_subprogress(progress, current, total, 68, 78, message);
    };
    let character_discovery_progress = progress.map(|progress| {
        crate::actions::character_ai::CharacterAiProgress::new(
            Some(&character_discovery_reporter),
            progress.cancel_token,
        )
    });
    let discovery_result = crate::actions::character_ai::discover_character_profiles_for_chapters(
        state,
        user_id,
        ch.project_id,
        Some(ch.chapter_number),
        character_discovery_progress.as_ref(),
    )
    .await
    .map_err(|err| format!("分析后角色发现失败: {err}"))?;
    save_completed_chapter_analysis_phase(
        state,
        workflow_run_id,
        ch,
        CHAPTER_ANALYSIS_STEP_CHARACTER_DISCOVERY,
        30,
        json!({ "completed": true, "result": discovery_result.clone() }),
    )
    .await?;
    Ok(discovery_result)
}
