use std::error::Error;

use sea_orm::DbErr;
use serde_json::json;

use super::{
    contracts::ProfileMaterialDbError,
    identity::{
        IMAGE_TYPE_OBJECT_MATERIAL, IMAGE_TYPE_SCENE_MATERIAL, PROFILE_KIND_OBJECT,
        is_project_reusable_video_material_image, profile_material_identity,
    },
    validation::{require_positive, require_text},
};

#[test]
fn profile_material_input_validation_rejects_non_positive_ids_and_blank_kind() {
    assert!(matches!(
        require_positive(0, "project_id"),
        Err(ProfileMaterialDbError::InvalidArgument(message))
            if message == "project_id 必须大于 0"
    ));
    assert!(matches!(
        require_text("   ", "profile_kind"),
        Err(ProfileMaterialDbError::InvalidArgument(message))
            if message == "profile_kind 不能为空"
    ));
}

#[test]
fn profile_material_database_error_keeps_context_and_source() {
    let error = ProfileMaterialDbError::Database {
        operation: "读取档案素材图片失败",
        source: DbErr::Custom("query failed".to_string()),
    };

    assert_eq!(
        error.to_string(),
        "读取档案素材图片失败: Custom Error: query failed"
    );
    assert!(error.source().is_some());
}

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
