use a4n_db_models::models::chapter::chapter;
use serde_json::Value;

use super::super::chapter_entries::{
    chapter_summary_for_reverse, sample_chapter_entry, sampled_chapters,
};

// Keep sampled summaries and full sample entries on the same projection boundary.
pub(super) struct BlueprintChapterContext {
    pub(super) summaries: Vec<Value>,
    pub(super) samples: Vec<Value>,
}

pub(super) fn blueprint_chapter_context(
    chapters: &[chapter::Model],
    summary_limit: usize,
) -> Result<BlueprintChapterContext, String> {
    let summaries = sampled_chapters(chapters, summary_limit)
        .into_iter()
        .map(chapter_summary_for_reverse)
        .collect();
    let samples = sampled_chapters(chapters, 10)
        .into_iter()
        .map(sample_chapter_entry)
        .collect::<Result<Vec<_>, _>>()?;
    Ok(BlueprintChapterContext { summaries, samples })
}
