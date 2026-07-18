use crate::actions::profile_materials::ProfileMaterialStyleMetadata;

pub(in crate::actions::image_generation) type ProfileImageSpec = (
    &'static str,
    &'static str,
    &'static str,
    &'static str,
    &'static str,
    &'static str,
    &'static str,
);

pub(in crate::actions::image_generation) const OBJECT_PROFILE_IMAGE_SPEC: ProfileImageSpec = (
    "object",
    "object_material",
    "generate_object_profile_image",
    "object_profile_image_generation",
    "object_profile_image",
    "物体素材图 provider 请求",
    "素材图请求预览",
);

pub(in crate::actions::image_generation) const SCENE_PROFILE_IMAGE_SPEC: ProfileImageSpec = (
    "scene",
    "scene_material",
    "generate_scene_profile_image",
    "scene_profile_image_generation",
    "scene_profile_image",
    "场景素材图 provider 请求",
    "场景素材图请求预览",
);

pub(in crate::actions::image_generation) struct ProfileImageMaterial {
    pub(in crate::actions::image_generation) spec: ProfileImageSpec,
    pub(in crate::actions::image_generation) profile_id: i64,
    pub(in crate::actions::image_generation) project_id: i64,
    pub(in crate::actions::image_generation) name: String,
    pub(in crate::actions::image_generation) prompt: String,
    pub(in crate::actions::image_generation) profile_content: String,
    pub(in crate::actions::image_generation) source_material: String,
    pub(in crate::actions::image_generation) style_metadata: ProfileMaterialStyleMetadata,
}
