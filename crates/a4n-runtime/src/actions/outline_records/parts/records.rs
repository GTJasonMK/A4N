use crate::AppState;
use crate::actions::access as project_access;
use crate::actions::outline_records::types::{
    GenerationProgressSummary, PartOutlineSummary, part_outline_summary,
};
use a4n_core::error::AppError;
use a4n_core::models::chapter::{chapter_outline, part_outline};
use std::collections::{HashMap, HashSet};

pub async fn list_parts(
    state: &AppState,
    user_id: i64,
    project_id: i64,
) -> Result<Vec<PartOutlineSummary>, AppError> {
    project_access::verify_project_access(state, user_id, project_id).await?;
    let parts = state
        .orchestrator
        .part_outlines_for_project(project_id)
        .await?;
    Ok(parts.iter().map(part_outline_summary).collect())
}

pub async fn delete_latest_parts(
    state: &AppState,
    user_id: i64,
    project_id: i64,
    count: u64,
) -> Result<(), AppError> {
    project_access::verify_project_editor(state, user_id, project_id).await?;
    state
        .orchestrator
        .delete_latest_part_outline_records(project_id, count)
        .await
}

pub async fn get_generation_progress(
    state: &AppState,
    user_id: i64,
    project_id: i64,
) -> Result<GenerationProgressSummary, AppError> {
    project_access::verify_project_access(state, user_id, project_id).await?;
    let rows = state
        .orchestrator
        .outline_generation_progress_rows(project_id)
        .await
        .map_err(AppError::from)?;
    let parts = rows.parts;
    let outlines = rows.chapter_outlines;
    let completed_parts = completed_part_count(&parts, &outlines)?;
    let completed_chapter_outlines = outlines
        .iter()
        .filter(|outline| outline.status == chapter_outline::STATUS_COMPLETED)
        .count() as u64;

    Ok(GenerationProgressSummary {
        total_parts: parts.len() as u64,
        completed_parts,
        total_chapter_outlines: outlines.len() as u64,
        completed_chapter_outlines,
    })
}

pub(in crate::actions::outline_records) fn completed_part_count(
    parts: &[part_outline::Model],
    outlines: &[chapter_outline::Model],
) -> Result<u64, AppError> {
    let mut outlines_by_chapter = HashMap::<i32, &chapter_outline::Model>::new();
    for outline in outlines {
        if !chapter_outline::is_valid_status(&outline.status) {
            return Err(progress_state_error(format!(
                "章节大纲 {} 状态无效：{}",
                outline.id, outline.status
            )));
        }
        if outlines_by_chapter
            .insert(outline.chapter_number, outline)
            .is_some()
        {
            return Err(progress_state_error(format!(
                "章节号 {} 存在重复章节大纲",
                outline.chapter_number
            )));
        }
    }

    let mut claimed_chapters = HashSet::<i32>::new();
    let mut completed = 0_u64;
    for part in parts {
        if part.start_chapter <= 0 || part.end_chapter < part.start_chapter {
            return Err(progress_state_error(format!(
                "分部 {} 章节范围无效：{}..={}",
                part.id, part.start_chapter, part.end_chapter
            )));
        }
        for chapter_number in part.start_chapter..=part.end_chapter {
            if !claimed_chapters.insert(chapter_number) {
                return Err(progress_state_error(format!(
                    "章节号 {} 被多个分部范围覆盖",
                    chapter_number
                )));
            }
        }

        let part_complete = (part.start_chapter..=part.end_chapter).all(|chapter_number| {
            outlines_by_chapter
                .get(&chapter_number)
                .is_some_and(|outline| outline.status == chapter_outline::STATUS_COMPLETED)
        });
        if part_complete {
            completed += 1;
        }
    }

    Ok(completed)
}

fn progress_state_error(current: String) -> AppError {
    AppError::InvalidState {
        current,
        operation: "查询大纲生成进度".to_string(),
    }
}
