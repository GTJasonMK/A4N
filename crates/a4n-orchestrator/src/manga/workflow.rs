use a4n_db::project::MangaCharacterPortraitRefSummary;

pub fn manga_pipeline_business_key(project_id: i64, chapter_id: i64) -> String {
    format!("manga_pipeline:{project_id}:{chapter_id}")
}

pub fn manga_pipeline_fingerprint(
    content: &str,
    character_portraits: &[MangaCharacterPortraitRefSummary],
) -> serde_json::Value {
    a4n_manga_core::manga_pipeline_fingerprint(content, character_portraits)
}

#[cfg(test)]
mod tests;
