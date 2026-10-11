use std::collections::{HashMap, HashSet};

use a4n_api_contract::outline::{
    ChapterOutlineSummary, DeleteLatestOutlineRecordsInput, OutlineGenerationProgressSummary,
    PartOutlineSummary, UpdateChapterOutlineInput,
};
use a4n_db::project::OutlineGenerationProgressRows;
use a4n_domain::chapter::{ChapterOutlineRecord, PartOutlineRecord, chapter_outline_status};

use super::WorkerFailure;

pub(super) fn chapter_outline_summary(
    outline: &ChapterOutlineRecord,
    source_kind: Option<&String>,
) -> ChapterOutlineSummary {
    ChapterOutlineSummary {
        id: outline.id,
        project_id: outline.project_id,
        part_id: outline.part_id,
        chapter_number: outline.chapter_number,
        title: outline.title.clone(),
        summary: outline.summary.clone(),
        beats: outline.beats.clone(),
        key_scenes: outline.key_scenes.clone(),
        pacing_score: outline.pacing_score,
        foreshadowing_setup: outline.foreshadowing_setup.clone(),
        foreshadowing_payoff: outline.foreshadowing_payoff.clone(),
        characters_involved: outline.characters_involved.clone(),
        word_count_target: outline.word_count_target,
        source_kind: source_kind.cloned(),
        status: outline.status.clone(),
        created_at: Some(outline.created_at.to_string()),
        updated_at: Some(outline.updated_at.to_string()),
    }
}

pub(super) fn part_outline_summary(part: &PartOutlineRecord) -> PartOutlineSummary {
    PartOutlineSummary {
        id: part.id,
        project_id: part.project_id,
        part_number: part.part_number,
        title: part.title.clone(),
        start_chapter: part.start_chapter,
        end_chapter: part.end_chapter,
        summary: part.summary.clone(),
        theme: part.theme.clone(),
        key_events: part.key_events.clone(),
        character_focus: part.character_focus.clone(),
        created_at: Some(part.created_at.to_string()),
        updated_at: Some(part.updated_at.to_string()),
    }
}

pub(super) fn generation_progress_summary(
    rows: &OutlineGenerationProgressRows,
) -> Result<OutlineGenerationProgressSummary, WorkerFailure> {
    let mut outlines_by_chapter = HashMap::<i32, &str>::new();
    for outline in &rows.chapter_outlines {
        if !chapter_outline_status::is_valid_status(&outline.status) {
            return Err(WorkerFailure::conflict(format!(
                "章节大纲 {} 状态无效：{}",
                outline.id, outline.status
            )));
        }
        if outlines_by_chapter
            .insert(outline.chapter_number, outline.status.as_str())
            .is_some()
        {
            return Err(WorkerFailure::conflict(format!(
                "章节号 {} 存在重复章节大纲",
                outline.chapter_number
            )));
        }
    }

    let mut claimed_chapters = HashSet::new();
    let mut completed_parts = 0_u64;
    for part in &rows.parts {
        if part.start_chapter <= 0 || part.end_chapter < part.start_chapter {
            return Err(WorkerFailure::conflict(format!(
                "分部 {} 章节范围无效：{}..={}",
                part.id, part.start_chapter, part.end_chapter
            )));
        }
        for chapter_number in part.start_chapter..=part.end_chapter {
            if !claimed_chapters.insert(chapter_number) {
                return Err(WorkerFailure::conflict(format!(
                    "章节号 {} 被多个分部范围覆盖",
                    chapter_number
                )));
            }
        }
        if (part.start_chapter..=part.end_chapter).all(|chapter_number| {
            outlines_by_chapter
                .get(&chapter_number)
                .is_some_and(|status| *status == chapter_outline_status::STATUS_COMPLETED)
        }) {
            completed_parts += 1;
        }
    }

    Ok(OutlineGenerationProgressSummary {
        total_parts: rows.parts.len() as u64,
        completed_parts,
        total_chapter_outlines: rows.chapter_outlines.len() as u64,
        completed_chapter_outlines: rows
            .chapter_outlines
            .iter()
            .filter(|outline| outline.status == chapter_outline_status::STATUS_COMPLETED)
            .count() as u64,
    })
}

pub(super) fn delete_input(
    input: DeleteLatestOutlineRecordsInput,
) -> a4n_db::project::DeleteLatestOutlineRecordsInput {
    a4n_db::project::DeleteLatestOutlineRecordsInput {
        project_id: input.project_id,
        targets: input
            .targets
            .into_iter()
            .map(|target| a4n_db::project::OutlineDeleteTarget {
                id: target.id,
                expected_updated_at: target.expected_updated_at,
            })
            .collect(),
    }
}

pub(super) fn normalize_update_input(
    input: UpdateChapterOutlineInput,
) -> Result<a4n_db::project::ProjectChapterOutlineUpdateInput, WorkerFailure> {
    WorkerFailure::require_positive_id(input.project_id, "project_id")?;
    WorkerFailure::require_positive_id(input.outline_id, "outline_id")?;
    let expected_updated_at = input.expected_updated_at.trim().to_string();
    if expected_updated_at.is_empty() {
        return Err(WorkerFailure::invalid_request(
            "缺少章节大纲资料版本，请刷新后重试",
        ));
    }
    Ok(a4n_db::project::ProjectChapterOutlineUpdateInput {
        project_id: input.project_id,
        outline_id: input.outline_id,
        expected_updated_at,
        title: optional_text(input.title, "title")?,
        summary: optional_text(input.summary, "summary")?,
        beats: optional_array(input.beats, "beats")?,
        key_scenes: optional_array(input.key_scenes, "key_scenes")?,
    })
}

fn optional_text(value: Option<String>, field: &str) -> Result<Option<String>, WorkerFailure> {
    value
        .map(|value| {
            let value = value.trim().to_string();
            if value.is_empty() {
                Err(WorkerFailure::invalid_request(format!(
                    "章节大纲字段 {field} 不能为空"
                )))
            } else {
                Ok(value)
            }
        })
        .transpose()
}

fn optional_array(
    value: Option<serde_json::Value>,
    field: &str,
) -> Result<Option<serde_json::Value>, WorkerFailure> {
    value
        .map(|value| {
            if value.is_array() {
                Ok(value)
            } else {
                Err(WorkerFailure::invalid_request(format!(
                    "章节大纲字段 {field} 必须是数组"
                )))
            }
        })
        .transpose()
}
