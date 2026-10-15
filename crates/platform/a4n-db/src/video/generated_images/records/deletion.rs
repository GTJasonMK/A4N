mod locking;
mod manga;
mod normal;
mod references;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImageReferenceCleanupIds {
    pub updated_prompt_ids: Vec<i64>,
    pub updated_video_ids: Vec<i64>,
}

pub use manga::delete_manga_page_image_records;
pub(crate) use manga::{
    delete_manga_chapter_image_records_in, delete_manga_page_image_records_in,
    manga_page_image_ids_for_deletion,
};
pub use normal::delete_generated_image_record_with_cleanup;
pub use normal::delete_generated_image_record_with_cleanup_effects;
pub(crate) use references::clear_generated_image_references_record;
