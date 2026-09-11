use a4n_db_models::models::chapter::chapter;
use a4n_progress::ProgressSpan;
use a4n_prompt::rendering::PromptRenderer;

use super::super::SceneProfileSyncHostPort;
use super::super::context::{
    build_scene_profile_sync_request, load_scene_profile_sync_anchor_context,
    load_scene_profile_sync_stable_context,
};
use super::super::progress::{
    SCENE_PROFILE_SYNC_CHAPTER_END, SCENE_PROFILE_SYNC_CHAPTER_START,
    SCENE_PROFILE_SYNC_PROGRESS_TOTAL, SCENE_PROFILE_SYNC_RAG_PROGRESS, SceneProfileSyncProgress,
    report_scene_profile_sync_progress, subspan,
};
use super::super::stages::run_scene_profile_sync_stages;
use super::super::types::ChapterAnchorMode;
use super::super::workflow::has_completed_scene_final_checkpoint;
use super::persistence::persist_scene_profile_sync_chapter;
use crate::OrchestratorContext;

pub(super) async fn sync_scene_profile_chapters(
    state: &OrchestratorContext,
    prompt_renderer: &dyn PromptRenderer,
    host_port: &dyn SceneProfileSyncHostPort,
    project_id: i64,
    workflow_run_id: i64,
    chapters: Vec<chapter::Model>,
    progress: Option<&SceneProfileSyncProgress<'_>>,
) -> Result<(), String> {
    let stable = load_scene_profile_sync_stable_context(state, project_id).await?;
    let anchors = load_scene_profile_sync_anchor_context(
        state,
        project_id,
        &chapters,
        ChapterAnchorMode::EnsureCurrent,
    )
    .await?;
    let chapter_count = chapters.len();
    let chapter_progress_span = ProgressSpan::new(
        SCENE_PROFILE_SYNC_CHAPTER_START,
        SCENE_PROFILE_SYNC_CHAPTER_END,
    );
    for (chapter_index, chapter) in chapters.into_iter().enumerate() {
        let chapter_id = chapter.id;
        let context = build_scene_profile_sync_request(
            state,
            project_id,
            chapter,
            anchors
                .get(&chapter_id)
                .cloned()
                .ok_or_else(|| "场景档案同步锚点批量读取缺少章节".to_string())?,
            &stable,
        )
        .await?;
        let chapter_span = chapter_progress_span.split(chapter_index, chapter_count);
        report_scene_profile_sync_progress(
            progress,
            chapter_span.start(),
            format!(
                "正在分阶段同步第 {} 章场景档案",
                context.chapter.chapter_number
            ),
            "场景档案同步已停止",
        )?;
        if has_completed_scene_final_checkpoint(state, workflow_run_id, &context).await? {
            report_scene_profile_sync_progress(
                progress,
                subspan(chapter_span, 5, 6, 6).end(),
                format!(
                    "第 {} 章场景档案已从断点恢复",
                    context.chapter.chapter_number
                ),
                "场景档案同步已停止",
            )?;
            continue;
        }
        let stage_responses = run_scene_profile_sync_stages(
            state,
            prompt_renderer,
            workflow_run_id,
            &context,
            progress,
            chapter_span,
            chapter_index,
        )
        .await?;
        persist_scene_profile_sync_chapter(
            state,
            project_id,
            workflow_run_id,
            &context,
            chapter_index,
            chapter_span,
            progress,
            stage_responses,
        )
        .await?;
    }
    report_scene_profile_sync_progress(
        progress,
        SCENE_PROFILE_SYNC_RAG_PROGRESS,
        "正在同步场景档案 RAG",
        "场景档案同步已停止",
    )?;
    host_port
        .sync_scene_rag_records(project_id, workflow_run_id)
        .await
        .map_err(|error| error.to_string())?;
    report_scene_profile_sync_progress(
        progress,
        SCENE_PROFILE_SYNC_PROGRESS_TOTAL,
        "场景档案同步已完成",
        "场景档案同步已停止",
    )?;
    Ok(())
}
