use a4n_core::models::chapter::chapter;
use serde_json::Value;

use super::chapter_outline::{
    analysis_metadata_array, analysis_value_path, chapter_outline_summary,
    collect_part_array_values,
};
use crate::{OrchestratorContext, project::ImportPartOutlineUpsertInput};

pub async fn upsert_import_part_outlines(
    context: &OrchestratorContext,
    project_id: i64,
    chapters: &[chapter::Model],
) -> Result<usize, String> {
    const CHAPTERS_PER_PART: i32 = 25;
    if chapters.len() < 50 {
        return Ok(0);
    }

    let mut generated = 0usize;
    let mut start = 1i32;
    let max_chapter = chapters
        .iter()
        .map(|chapter| chapter.chapter_number)
        .max()
        .ok_or_else(|| "项目没有可用于生成分部大纲的章节".to_string())?;

    while start <= max_chapter {
        let end = (start + CHAPTERS_PER_PART - 1).min(max_chapter);
        let part_number = ((start - 1) / CHAPTERS_PER_PART) + 1;
        let part_chapters = chapters
            .iter()
            .filter(|chapter| chapter.chapter_number >= start && chapter.chapter_number <= end)
            .collect::<Vec<_>>();
        if part_chapters.is_empty() {
            start = end + 1;
            continue;
        }

        let summary = part_chapters
            .iter()
            .map(|chapter| {
                Ok(format!(
                    "第{}章：{}",
                    chapter.chapter_number,
                    chapter_outline_summary(chapter)?
                ))
            })
            .collect::<Result<Vec<_>, String>>()?
            .join("\n");
        let key_events = Value::Array(collect_part_array_values(&part_chapters, 30, |chapter| {
            analysis_value_path(
                chapter.analysis_data.as_ref(),
                &["key_events"],
                chapter.chapter_number,
            )
        })?);
        let characters = Value::Array(collect_part_array_values(&part_chapters, 30, |chapter| {
            analysis_metadata_array(
                chapter.analysis_data.as_ref(),
                "characters",
                chapter.chapter_number,
            )
        })?);

        let title = format!("第{part_number}部分");
        context
            .upsert_import_part_outline(ImportPartOutlineUpsertInput {
                project_id,
                part_number,
                title,
                start_chapter: start,
                end_chapter: end,
                summary,
                theme: "导入分析".to_string(),
                key_events,
                character_focus: characters,
            })
            .await
            .map_err(|err| err.to_string())?;

        generated += 1;
        start = end + 1;
    }
    Ok(generated)
}
