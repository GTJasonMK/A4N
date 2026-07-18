use std::collections::{BTreeMap, BTreeSet};

use a4n_core::models::chapter::chapter;
use a4n_core::structured_progress::ProgressSpan;
use serde_json::{Value, json};

use super::super::context::build_scene_profile_sync_request;
use super::super::progress::{
    SCENE_PROFILE_SYNC_CHAPTER_END, SCENE_PROFILE_SYNC_CHAPTER_START,
    SCENE_PROFILE_SYNC_PROGRESS_TOTAL, SCENE_PROFILE_SYNC_RAG_PROGRESS, SceneProfileSyncProgress,
    report_scene_profile_sync_progress, subspan,
};
use super::super::stages::run_scene_profile_sync_stages;
use super::super::types::ChapterAnchorMode;
use super::super::workflow::{load_completed_scene_final_checkpoint, merge_scene_final_checkpoint};
use super::persistence::persist_scene_profile_sync_chapter;
use crate::state::AppState;

pub(super) async fn sync_scene_profile_chapters(
    state: &AppState,
    project_id: i64,
    workflow_run_id: i64,
    chapters: Vec<chapter::Model>,
    progress: Option<&SceneProfileSyncProgress<'_>>,
) -> Result<Value, String> {
    let mut profile_ids = BTreeSet::new();
    let mut occurrence_count = 0usize;
    let mut transition_count = 0usize;
    let mut entity_state_count = 0usize;
    let mut profile_summaries = BTreeMap::new();
    let chapter_count = chapters.len();
    let chapter_progress_span = ProgressSpan::new(
        SCENE_PROFILE_SYNC_CHAPTER_START,
        SCENE_PROFILE_SYNC_CHAPTER_END,
    );
    for (chapter_index, chapter) in chapters.into_iter().enumerate() {
        let context = build_scene_profile_sync_request(
            state,
            project_id,
            chapter,
            ChapterAnchorMode::EnsureCurrent,
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
        if let Some(checkpoint) =
            load_completed_scene_final_checkpoint(state, workflow_run_id, &context, chapter_index)
                .await?
        {
            merge_scene_final_checkpoint(
                checkpoint,
                &mut profile_ids,
                &mut occurrence_count,
                &mut transition_count,
                &mut entity_state_count,
                &mut profile_summaries,
            )?;
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
            workflow_run_id,
            &context,
            progress,
            chapter_span,
            chapter_index,
        )
        .await?;
        let result = persist_scene_profile_sync_chapter(
            state,
            project_id,
            workflow_run_id,
            &context,
            chapter_index,
            chapter_span,
            progress,
            stage_responses,
            &mut profile_ids,
            &mut profile_summaries,
        )
        .await?;
        occurrence_count += result.occurrence_count;
        transition_count += result.transition_count;
        entity_state_count += result.entity_state_count;
    }
    report_scene_profile_sync_progress(
        progress,
        SCENE_PROFILE_SYNC_RAG_PROGRESS,
        "正在同步场景档案 RAG",
        "场景档案同步已停止",
    )?;
    let rag = crate::rag_ingest::sync_scene_rag_records(
        state,
        project_id,
        crate::rag_ingest::SceneRagSyncTraceSource::WorkflowRun(workflow_run_id),
    )
    .await?;
    report_scene_profile_sync_progress(
        progress,
        SCENE_PROFILE_SYNC_PROGRESS_TOTAL,
        "场景档案同步已完成",
        "场景档案同步已停止",
    )?;
    Ok(json!({
        "status": "synced",
        "profile_ids": profile_ids.iter().copied().collect::<Vec<_>>(),
        "profiles": profile_summaries.into_values().collect::<Vec<_>>(),
        "occurrence_count": occurrence_count,
        "transition_count": transition_count,
        "entity_state_count": entity_state_count,
        "rag": rag
    }))
}
