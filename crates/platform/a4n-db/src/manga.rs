mod artifacts;
mod records;
mod sync;
mod types;

pub async fn connect_manga_record_worker_database(
    database_url: String,
) -> Result<crate::DbHandle, sea_orm::DbErr> {
    let mut options = sea_orm::ConnectOptions::new(database_url);
    options
        .max_connections(8)
        .min_connections(1)
        .connect_timeout(std::time::Duration::from_secs(5));
    let database = sea_orm::Database::connect(options).await?;
    Ok(crate::DbHandle::new(std::sync::Arc::new(database)))
}

pub use a4n_db_models::models::chapter::chapter_manga_prompt::MANGA_PAGE_PROMPT_TYPE;
pub use artifacts::{
    MangaPipelineArtifactProjectionReset, delete_manga_pipeline_artifact, manga_pipeline_artifact,
    manga_pipeline_artifact_deletion_plan, manga_pipeline_artifacts_by_chapter,
    upsert_manga_pipeline_artifact,
};
pub use records::{
    delete_manga_page_by_number, delete_manga_prompts_by_chapter, manga_page_prompt_by_id,
    manga_page_prompt_for_image_generation, manga_page_prompt_statuses_by_chapter,
    manga_page_prompt_statuses_by_chapters, manga_page_prompts_by_chapter,
    manga_reference_bound_asset, mark_manga_page_prompt_image_completed_with_image,
    update_manga_page_prompt_image_prompt, update_manga_page_prompt_layout,
};
pub use sync::{sync_manga_page_prompts_by_chapter, upsert_manga_page_prompt};
pub use types::{MangaPipelineDbError, MangaPromptRecordError, MangaPromptUpdatedAt};
pub(in crate::manga) use types::{
    manga_record_database, manga_video_error, require_manga_text, require_positive_manga_id,
    validate_manga_page_number, validate_manga_page_prompt_layout,
};

#[cfg(test)]
mod tests;
