mod contracts;
mod list;
mod metadata;
mod primary;
mod prompts;
mod queries;
mod upload;

pub use contracts::*;
pub use list::list_profile_materials;
pub use primary::set_profile_primary_material;
pub use upload::upload_profile_material_image;

pub(crate) use metadata::{ProfileMaterialStyleMetadata, profile_material_metadata};
pub(crate) use prompts::{
    object_profile_content, object_profile_image_prompt, scene_profile_content,
    scene_profile_image_prompt,
};
pub(crate) use queries::{
    count_profile_material_images, count_profile_material_images_by_profile,
    list_profile_material_image_ids,
};

#[cfg(test)]
mod tests;
