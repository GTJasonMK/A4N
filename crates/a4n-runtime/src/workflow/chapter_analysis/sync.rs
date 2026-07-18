mod discovery;
mod finalization;
mod outputs;
mod profiles;

use super::prelude::{AppState, access, chapter};
use super::{
    CHAPTER_ANALYSIS_PROGRESS_TOTAL, CHAPTER_ANALYSIS_STEP_DERIVED_SYNC, ChapterAnalysisProgress,
    mark_chapter_analysis_current_step, report_chapter_analysis_progress,
};
use discovery::{sync_character_discovery_phase, sync_object_discovery_phase};
use finalization::refresh_project_status_phase;
use outputs::sync_analysis_outputs_phase;
use profiles::{sync_character_profiles_phase, sync_scene_profiles_phase};

pub async fn sync_project_state_after_chapter_analysis_with_progress(
    state: &AppState,
    user_id: i64,
    ch: &chapter::Model,
    workflow_run_id: i64,
    progress: Option<&ChapterAnalysisProgress<'_>>,
) -> Result<(), String> {
    let rag_trace_id = crate::sidecar_clients::workflow_rag_operation_trace_id(workflow_run_id)?;
    let workflow_run_id = Some(workflow_run_id);
    access::verify_project_editor(state, user_id, ch.project_id)
        .await
        .map_err(|err| err.to_string())?;
    mark_chapter_analysis_current_step(state, workflow_run_id, CHAPTER_ANALYSIS_STEP_DERIVED_SYNC)
        .await?;
    report_chapter_analysis_progress(
        progress,
        50,
        CHAPTER_ANALYSIS_PROGRESS_TOTAL,
        "正在同步章节分析、伏笔和 RAG",
        "章节结构化分析已停止",
    )?;

    sync_analysis_outputs_phase(state, ch, workflow_run_id, rag_trace_id, progress).await?;
    sync_object_discovery_phase(state, ch, workflow_run_id, progress).await?;
    let discovery_result =
        sync_character_discovery_phase(state, user_id, ch, workflow_run_id, progress).await?;
    sync_scene_profiles_phase(state, user_id, ch, workflow_run_id, progress).await?;
    sync_character_profiles_phase(
        state,
        user_id,
        ch,
        workflow_run_id,
        &discovery_result,
        progress,
    )
    .await?;
    refresh_project_status_phase(state, ch, workflow_run_id, progress).await?;

    report_chapter_analysis_progress(
        progress,
        CHAPTER_ANALYSIS_PROGRESS_TOTAL,
        CHAPTER_ANALYSIS_PROGRESS_TOTAL,
        "章节分析和项目状态同步已完成",
        "章节结构化分析已停止",
    )?;
    Ok(())
}
