//! 档案素材身份识别。

use a4n_core::models::ai::generated_image;
use serde_json::Value;

pub const PROFILE_KIND_SCENE: &str = "scene";
pub const PROFILE_KIND_OBJECT: &str = "object";
pub const IMAGE_TYPE_SCENE_MATERIAL: &str = "scene_material";
pub const IMAGE_TYPE_OBJECT_MATERIAL: &str = "object_material";

pub fn profile_material_identity(
    image_type: &str,
    metadata: Option<&Value>,
) -> Option<(&'static str, i64)> {
    match image_type {
        IMAGE_TYPE_SCENE_MATERIAL => {
            if metadata_profile_kind(metadata) != Some(PROFILE_KIND_SCENE) {
                return None;
            }
            metadata_number(metadata, "profile_id").map(|id| (PROFILE_KIND_SCENE, id))
        }
        IMAGE_TYPE_OBJECT_MATERIAL => {
            if metadata_profile_kind(metadata) != Some(PROFILE_KIND_OBJECT) {
                return None;
            }
            metadata_number(metadata, "profile_id").map(|id| (PROFILE_KIND_OBJECT, id))
        }
        _ => None,
    }
}

pub fn generated_image_profile_material_identity(
    image: &generated_image::Model,
) -> Option<(&'static str, i64)> {
    profile_material_identity(&image.image_type, image.metadata_json.as_ref())
}

pub fn is_video_frame_input_metadata(metadata: Option<&Value>) -> bool {
    let Some(metadata) = metadata.and_then(Value::as_object) else {
        return false;
    };
    metadata
        .get("generation_context")
        .and_then(Value::as_object)
        .and_then(|context| context.get("source"))
        .and_then(Value::as_str)
        .is_some_and(|source| {
            matches!(
                source.trim(),
                "video_boundary_frame_prompt"
                    | "video_first_frame_prompt"
                    | "video_tail_frame_capture"
            )
        })
        || metadata
            .get("video_frame_role")
            .and_then(Value::as_str)
            .is_some_and(|role| matches!(role.trim(), "key_frame" | "start_frame" | "end_frame"))
}

pub fn is_project_reusable_video_material_image(
    image_type: &str,
    metadata: Option<&Value>,
) -> bool {
    if is_video_frame_input_metadata(metadata) {
        return false;
    }
    profile_material_identity(image_type, metadata).is_some()
}

fn metadata_number(metadata: Option<&Value>, key: &str) -> Option<i64> {
    metadata
        .and_then(|metadata| metadata.get(key))
        .and_then(Value::as_i64)
        .filter(|id| *id > 0)
}

fn metadata_profile_kind(metadata: Option<&Value>) -> Option<&str> {
    metadata
        .and_then(|metadata| metadata.get("profile_kind"))
        .and_then(Value::as_str)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn profile_material_identity_requires_current_profile_metadata() {
        assert_eq!(
            profile_material_identity(
                IMAGE_TYPE_OBJECT_MATERIAL,
                Some(&json!({ "profile_kind": "object", "profile_id": 8 })),
            ),
            Some((PROFILE_KIND_OBJECT, 8))
        );
        assert_eq!(
            profile_material_identity(
                IMAGE_TYPE_SCENE_MATERIAL,
                Some(&json!({ "profile_id": 12 }))
            ),
            None
        );
    }

    #[test]
    fn profile_material_identity_rejects_conflicting_profile_kind() {
        assert_eq!(
            profile_material_identity(
                IMAGE_TYPE_SCENE_MATERIAL,
                Some(&json!({ "profile_kind": "object", "profile_id": 8 })),
            ),
            None
        );
        assert_eq!(
            profile_material_identity(
                IMAGE_TYPE_OBJECT_MATERIAL,
                Some(&json!({ "profile_kind": "scene", "profile_id": 9 })),
            ),
            None
        );
    }

    #[test]
    fn project_reusable_material_excludes_video_frame_inputs() {
        let metadata = json!({
            "profile_kind": "object",
            "profile_id": 7,
            "video_frame_role": "start_frame"
        });
        assert!(!is_project_reusable_video_material_image(
            IMAGE_TYPE_OBJECT_MATERIAL,
            Some(&metadata),
        ));
    }
}
