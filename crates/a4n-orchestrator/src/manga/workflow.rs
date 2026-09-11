use a4n_db::project::MangaCharacterPortraitRefSummary;
use a4n_domain::source_spans::{ChapterSourceContext, source_metadata};

use super::reference_assets::manga_reference_assets_json;

pub fn manga_pipeline_business_key(project_id: i64, chapter_id: i64) -> String {
    format!("manga_pipeline:{project_id}:{chapter_id}")
}

pub fn manga_pipeline_fingerprint(
    content: &str,
    character_portraits: &[MangaCharacterPortraitRefSummary],
) -> serde_json::Value {
    serde_json::json!({
        "source": source_metadata(ChapterSourceContext { content }),
        "reference_assets": manga_reference_assets_json(character_portraits),
    })
}

#[cfg(test)]
mod tests;
