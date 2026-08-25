mod generation;
mod materials;
mod records;

pub(in crate::routes::commands) use generation::{
    preview_batch_images_request, preview_page_image_request, preview_scene_image_request,
};
pub(in crate::routes::commands) use materials::{
    list_profile_materials, set_profile_primary_material, upload_profile_material_image,
};
pub(in crate::routes::commands) use records::{
    delete_image, list_images_by_chapter, list_images_by_project, list_images_by_projects,
    list_video_reference_images, update_image, upload_image,
};
