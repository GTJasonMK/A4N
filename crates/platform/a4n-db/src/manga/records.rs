pub(in crate::manga) mod mapping;
mod mutations;
mod queries;
mod references;

pub use mutations::{
    delete_manga_page_by_number, delete_manga_prompts_by_chapter,
    mark_manga_page_prompt_image_completed_with_image, update_manga_page_prompt_image_prompt,
    update_manga_page_prompt_layout,
};
pub use queries::{
    manga_page_prompt_by_id, manga_page_prompt_for_image_generation,
    manga_page_prompt_statuses_by_chapter, manga_page_prompt_statuses_by_chapters,
    manga_page_prompts_by_chapter,
};
pub use references::manga_reference_bound_asset;
