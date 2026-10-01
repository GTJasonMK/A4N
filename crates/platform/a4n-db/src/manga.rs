mod pipeline;
mod records;
mod sync;
mod types;

pub use pipeline::cancel_manga_page_prompts;
pub(crate) use records::manga_page_prompt_record_from_model;
pub use records::{
    delete_manga_prompt_record, delete_manga_prompts_by_chapter, manga_page_prompt_by_id,
    manga_page_prompt_for_image_generation, manga_page_prompt_statuses_by_chapter,
    manga_page_prompt_statuses_by_chapters, manga_page_prompts_by_chapter,
    manga_reference_bound_asset, mark_manga_page_prompt_image_completed_with_image,
    update_manga_page_prompt_image_prompt, update_manga_page_prompt_layout,
};
pub use sync::sync_manga_page_prompts_by_chapter;
pub use types::{MangaPipelineDbError, MangaPromptRecordError, MangaPromptUpdatedAt};
pub(in crate::manga) use types::{
    manga_record_database, require_manga_text, require_positive_manga_id,
    validate_manga_page_number, validate_manga_page_prompt_layout,
};

#[cfg(test)]
mod tests;
