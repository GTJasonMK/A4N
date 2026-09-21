use a4n_db_models::models::ai::generated_image;
use a4n_domain::image::{IMAGE_TYPE_OBJECT_MATERIAL, IMAGE_TYPE_SCENE_MATERIAL};
use serde_json::Value;

pub const PROFILE_KIND_SCENE: &str = "scene";
pub const PROFILE_KIND_OBJECT: &str = "object";

pub fn profile_material_identity(
    image_type: &str,
    metadata: Option<&Value>,
) -> Option<(&'static str, i64)> {
    a4n_domain::profile_material::profile_material_identity(image_type, metadata)
}

pub fn profile_material_identity_from_fields(
    image_type: &str,
    profile_kind: Option<&str>,
    profile_id: Option<i64>,
) -> Option<(&'static str, i64)> {
    let expected_kind = match image_type {
        IMAGE_TYPE_SCENE_MATERIAL => PROFILE_KIND_SCENE,
        IMAGE_TYPE_OBJECT_MATERIAL => PROFILE_KIND_OBJECT,
        _ => return None,
    };
    if profile_kind != Some(expected_kind) {
        return None;
    }
    profile_id
        .filter(|id| *id > 0)
        .map(|id| (expected_kind, id))
}

pub fn generated_image_profile_material_identity(
    image: &generated_image::Model,
) -> Option<(&'static str, i64)> {
    profile_material_identity(&image.image_type, image.metadata_json.as_ref())
}

pub fn is_project_reusable_video_material_image(
    image_type: &str,
    metadata: Option<&Value>,
) -> bool {
    profile_material_identity(image_type, metadata).is_some()
}
