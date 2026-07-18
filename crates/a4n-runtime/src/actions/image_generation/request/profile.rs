mod builders;
mod direct_input;
mod metadata;

pub(in crate::actions::image_generation) use builders::{
    build_object_profile_material, build_scene_profile_material,
};
pub(in crate::actions::image_generation) use direct_input::profile_material_direct_input;
