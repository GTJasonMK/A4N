use a4n_core::import_reverse::ReverseContextKind;
use a4n_db::import_reverse::ReverseContextRows;
use serde_json::{Value, json};

use super::blueprint::blueprint_context;
use super::chapter_entries::{chapter_outline_entry, chapter_summary_for_reverse};

// Select the target-specific projection only after validating the canonical rows scope.
pub(super) fn project_reverse_context(
    rows: &ReverseContextRows,
    project_id: i64,
    kind: ReverseContextKind,
    limit: usize,
) -> Result<Value, String> {
    if rows.project.id != project_id {
        return Err(format!("项目不存在: project_id={project_id}"));
    }
    if rows.chapters.is_empty() {
        return Err("项目没有可用于反推的章节正文".to_string());
    }

    match kind {
        ReverseContextKind::Outline => Ok(json!({
            "novel_title": rows.project.title,
            "total_chapters": rows.chapters.len(),
            "summaries": rows.chapters
                .iter()
                .take(limit)
                .map(chapter_summary_for_reverse)
                .collect::<Vec<_>>(),
        })),
        ReverseContextKind::PartOutline => {
            let outline_values = if rows.chapter_outlines.is_empty() {
                rows.chapters
                    .iter()
                    .take(limit)
                    .map(chapter_outline_entry)
                    .collect::<Vec<_>>()
            } else {
                rows.chapter_outlines
                    .iter()
                    .take(limit)
                    .map(|outline| {
                        json!({
                            "chapter_number": outline.chapter_number,
                            "title": outline.title,
                            "summary": outline.summary,
                            "key_scenes": outline.key_scenes,
                            "characters_involved": outline.characters_involved,
                        })
                    })
                    .collect::<Vec<_>>()
            };
            Ok(json!({
                "novel_title": rows.project.title,
                "total_chapters": rows.chapters.len(),
                "chapter_outlines": outline_values,
            }))
        }
        ReverseContextKind::Blueprint => blueprint_context(rows, limit),
    }
}
