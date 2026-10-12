mod object;
mod records;
mod scene;
mod sync;

pub(in crate::routes::commands) use object::{
    generate_object_profile_image_prompt, preview_object_profile_image_request,
};
pub(in crate::routes::commands) use records::{
    create_foreshadowing, create_object_profile, create_scene_profile, delete_foreshadowing,
    delete_object_profile, delete_scene_profile, list_foreshadowing, list_object_profile_index,
    list_object_profiles, list_scene_profiles, update_foreshadowing, update_object_profile,
    update_scene_profile,
};
pub(in crate::routes::commands) use scene::generate_scene_profile_image_prompt;
pub(in crate::routes::commands) use sync::sync_scene_profiles;
