mod deletion;
mod library;
mod mutations;
mod queries;
mod validation;

#[cfg(test)]
mod tests;

pub(crate) use deletion::clear_generated_image_references_record;
pub use deletion::{delete_generated_image_record_with_cleanup, delete_manga_page_image_records};
pub(crate) use deletion::{
    delete_manga_chapter_image_records_in, delete_manga_page_image_records_in,
    manga_page_image_ids_for_deletion,
};
pub use library::{
    video_library_chapter_image_records, video_library_explicit_image_records,
    video_profile_material_read_model, video_project_context_read_model,
};
pub use mutations::update_generated_image_record;
pub(in crate::video::generated_images) use queries::generated_image_records_by_ids;
pub(crate) use queries::generated_images_for_chapter_by_id_desc;
pub use queries::{
    generated_image_by_id, generated_image_ids_by_ids, generated_images_by_ids,
    generated_images_for_chapter, generated_images_for_project, generated_images_for_projects,
    generated_images_for_projects_chapter_by_id_desc,
};
