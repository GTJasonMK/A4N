use a4n_core::source_spans::{ChapterSourceContext, source_metadata};

use super::reference_assets::{MangaReferenceContext, manga_reference_assets_json};

pub fn manga_pipeline_business_key(project_id: i64, chapter_id: i64) -> String {
    format!("manga_pipeline:{project_id}:{chapter_id}")
}

pub fn manga_pipeline_fingerprint(
    content: &str,
    reference_context: &MangaReferenceContext,
) -> serde_json::Value {
    serde_json::json!({
        "source": source_metadata(ChapterSourceContext { content }),
        "reference_assets": manga_reference_assets_json(&reference_context.character_portraits),
    })
}

#[cfg(test)]
mod tests;
