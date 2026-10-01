use a4n_db_models::models::chapter::chapter_manga_prompt;
use a4n_domain::manga::MangaPagePromptRecord;

mod mutations;
mod queries;
mod references;

pub fn manga_page_prompt_record(prompt: &chapter_manga_prompt::Model) -> MangaPagePromptRecord {
    MangaPagePromptRecord {
        id: prompt.id,
        chapter_id: prompt.chapter_id,
        project_id: prompt.project_id,
        page_number: prompt.page_number,
        image_prompt: prompt.image_prompt.clone(),
        dialogue_text: prompt.dialogue_text.clone(),
        layout: prompt.layout.clone(),
        generated_image_id: prompt.generated_image_id,
        status: prompt.status.clone(),
        created_at: prompt.created_at,
        updated_at: prompt.updated_at,
    }
}

pub use mutations::{
    delete_manga_prompt_record, delete_manga_prompts_by_chapter,
    mark_manga_page_prompt_image_completed_with_image, update_manga_page_prompt_image_prompt,
    update_manga_page_prompt_layout,
};
pub use queries::{
    manga_page_prompt_by_id, manga_page_prompt_for_image_generation,
    manga_page_prompt_statuses_by_chapter, manga_page_prompt_statuses_by_chapters,
    manga_page_prompts_by_chapter,
};
pub use references::manga_reference_bound_asset;
