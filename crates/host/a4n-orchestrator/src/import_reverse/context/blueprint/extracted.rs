use a4n_db::import_reverse::ReverseBlueprintContextRows;
use a4n_domain::chapter::ChapterRecord;
use serde_json::{Value, json};

use super::super::extraction::{
    collect_analysis_foreshadowing, collect_extracted_characters, dedup_character_records,
};
use super::projections::{existing_character_projections, protagonist_projections};

// Merge chapter-derived and persisted blueprint evidence before projecting JSON.
pub(super) struct BlueprintExtractedContext {
    pub(super) characters: Vec<Value>,
    pub(super) foreshadowing: Vec<Value>,
}

pub(super) fn blueprint_extracted_context(
    chapters: &[ChapterRecord],
    blueprint_rows: &ReverseBlueprintContextRows,
) -> BlueprintExtractedContext {
    let mut foreshadowing = collect_analysis_foreshadowing(chapters, 30);
    for item in &blueprint_rows.foreshadowing {
        if foreshadowing.len() >= 40 {
            break;
        }
        foreshadowing.push(json!({
            "chapter": item.setup_chapter,
            "content": item.description,
            "payoff_chapter": item.payoff_chapter,
            "status": item.status,
            "importance": item.importance,
            "related_characters": item.related_characters,
        }));
    }

    let mut characters = collect_extracted_characters(chapters, 40);
    characters.extend(existing_character_projections(blueprint_rows));
    characters.extend(protagonist_projections(blueprint_rows));
    let characters = dedup_character_records(characters)
        .into_iter()
        .take(40)
        .collect();

    BlueprintExtractedContext {
        characters,
        foreshadowing,
    }
}
