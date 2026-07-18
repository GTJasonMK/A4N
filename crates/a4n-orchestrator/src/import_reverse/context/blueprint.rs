use a4n_db::import_reverse::ReverseContextRows;
use serde_json::{Value, json};

use self::chapters::blueprint_chapter_context;
use self::extracted::blueprint_extracted_context;
use self::projections::existing_blueprint_projection;
use super::chapter_entries::chapter_part_entry;

mod chapters;
mod extracted;
mod projections;

// Build the compact project context consumed by reverse-engineering requests.
pub(super) fn blueprint_context(rows: &ReverseContextRows, limit: usize) -> Result<Value, String> {
    let blueprint_rows = rows
        .blueprint_context
        .as_ref()
        .ok_or_else(|| "蓝图反推上下文缺少蓝图读取结果".to_string())?;
    let chapter_context = blueprint_chapter_context(&rows.chapters, limit)?;
    let extracted_context = blueprint_extracted_context(&rows.chapters, blueprint_rows);
    let existing_blueprint = existing_blueprint_projection(blueprint_rows);

    Ok(json!({
        "novel_title": rows.project.title,
        "total_chapters": rows.chapters.len(),
        "sampled_chapters_count": chapter_context.summaries.len(),
        "chapter_summaries": chapter_context.summaries,
        "part_outlines": rows.part_outlines.iter().map(chapter_part_entry).collect::<Vec<_>>(),
        "sample_chapters": chapter_context.samples,
        "extracted_characters": extracted_context.characters,
        "extracted_foreshadowing": extracted_context.foreshadowing,
        "existing_blueprint": existing_blueprint,
    }))
}
