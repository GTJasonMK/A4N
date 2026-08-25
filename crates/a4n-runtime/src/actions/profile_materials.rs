mod list;
mod primary;
mod queries;
mod upload;

pub use a4n_api_contract::profile::{
    ListProfileMaterialsInput, ProfileMaterialGroup, SetProfilePrimaryMaterialInput,
    UploadProfileMaterialImageInput,
};
pub use list::list_profile_materials;
pub use primary::set_profile_primary_material;
pub use upload::upload_profile_material_image;

pub(crate) use queries::{
    count_profile_material_images, count_profile_material_images_by_profile,
    list_profile_material_image_ids,
};
