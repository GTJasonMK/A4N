use super::ChapterSummary;
use crate::AppState;
use a4n_core::{error::AppError, models::chapter::chapter};
use std::collections::HashMap;

pub fn chapter_summary(ch: &chapter::Model) -> ChapterSummary {
    ChapterSummary {
        id: ch.id,
        project_id: ch.project_id,
        outline_id: ch.outline_id,
        chapter_number: ch.chapter_number,
        title: ch.title.clone(),
        content: ch.content.clone(),
        word_count: ch.word_count,
        status: ch.status.clone(),
        summary: ch.summary.clone(),
        style_tags: ch.style_tags.clone(),
        analysis_data: ch.analysis_data.clone(),
        analysis_sync_status: None,
        analysis_sync_can_resume: false,
        analysis_sync_resume_block_reason: None,
        created_at: Some(ch.created_at.to_string()),
        updated_at: Some(ch.updated_at.to_string()),
    }
}

pub(super) async fn attach_chapter_analysis_sync_state(
    state: &AppState,
    chapters: &[chapter::Model],
    summaries: &mut [ChapterSummary],
) -> Result<(), AppError> {
    let Some(project_id) = chapters.first().map(|chapter| chapter.project_id) else {
        return Ok(());
    };
    let runs = state
        .orchestrator
        .workflow_runs_by_project_kind(project_id, "chapter_analysis")
        .await
        .map_err(AppError::from)?;
    let mut latest_by_chapter = HashMap::new();
    for run in runs {
        let Some(chapter_id) = run.chapter_id else {
            continue;
        };
        latest_by_chapter.entry(chapter_id).or_insert(run);
    }
    for (chapter, summary) in chapters.iter().zip(summaries.iter_mut()) {
        let Some(run) = latest_by_chapter.get(&chapter.id) else {
            continue;
        };
        if !a4n_orchestrator::chapter_analysis::chapter_analysis_run_input_matches(
            &run.input_fingerprint_json,
            chapter,
        ) {
            continue;
        }
        summary.analysis_sync_status = Some(run.status.clone());
        summary.analysis_sync_can_resume = chapter.analysis_data.is_some() && run.can_resume;
        summary.analysis_sync_resume_block_reason = run.resume_block_reason.clone();
    }
    Ok(())
}
