use a4n_db::project::MangaCharacterPortraitRefSummary;
use a4n_domain::media::GeneratedImageRecord;

pub fn manga_pipeline_business_key(project_id: i64, chapter_id: i64) -> String {
    format!("manga_pipeline:{project_id}:{chapter_id}")
}

pub fn manga_pipeline_run_fingerprint(
    content: &str,
    character_portraits: &[MangaCharacterPortraitRefSummary],
    available_reference_images: &[GeneratedImageRecord],
    run_stage: Option<a4n_domain::manga::MangaPipelineArtifactStage>,
) -> serde_json::Value {
    let available_reference_images = serde_json::json!(
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
    a4n_manga_core::manga_pipeline_fingerprint_with_reference_images(
        content,
        character_portraits,
        available_reference_images,
        run_stage,
    )
}

#[cfg(test)]
mod tests;
