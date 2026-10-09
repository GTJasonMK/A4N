use a4n_db::project::MangaCharacterPortraitRefSummary;
use a4n_domain::media::GeneratedImageRecord;

pub fn manga_pipeline_business_key(project_id: i64, chapter_id: i64) -> String {
    format!("manga_pipeline:{project_id}:{chapter_id}")
}

pub fn manga_pipeline_fingerprint(
    content: &str,
    character_portraits: &[MangaCharacterPortraitRefSummary],
    available_reference_images: &[GeneratedImageRecord],
) -> serde_json::Value {
    let mut fingerprint = a4n_manga_core::manga_pipeline_fingerprint(content, character_portraits);
    fingerprint["available_reference_images"] = serde_json::json!(
        available_reference_images
            .iter()
            .map(|image| (
                image.id,
                &image.image_type,
                &image.file_path,
                &image.prompt,
                &image.metadata_json,
            ))
            .collect::<Vec<_>>()
    );
    fingerprint
}

pub fn manga_pipeline_run_fingerprint(
    content: &str,
    character_portraits: &[MangaCharacterPortraitRefSummary],
    available_reference_images: &[GeneratedImageRecord],
    run_stage: Option<a4n_domain::manga::MangaPipelineArtifactStage>,
) -> serde_json::Value {
    let mut fingerprint =
        manga_pipeline_fingerprint(content, character_portraits, available_reference_images);
    if let Some(stage) = run_stage {
        fingerprint["run_stage"] = serde_json::json!(stage);
    }
    fingerprint
}

#[cfg(test)]
mod tests;
