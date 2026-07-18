use super::*;
use serde_json::json;

#[test]
fn profile_image_inputs_reject_unknown_fields() {
    let object_err = serde_json::from_value::<ObjectProfileImageInput>(json!({
        "object_profile_id": 1,
        "style": "写实",
        "set_primary": true
    }))
    .expect_err("object profile image input must reject unknown fields");
    assert!(object_err.to_string().contains("unknown field"));

    let scene_err = serde_json::from_value::<SceneProfileImageInput>(json!({
        "scene_profile_id": 1,
        "style": "写实",
        "legacy_style_id": "old"
    }))
    .expect_err("scene profile image input must reject unknown fields");
    assert!(scene_err.to_string().contains("unknown field"));
}

#[test]
fn profile_image_inputs_accept_documented_fields() {
    let object = serde_json::from_value::<ObjectProfileImageInput>(json!({
        "object_profile_id": 1,
        "chapter_id": 2,
        "style": "写实",
        "config_id": 3,
        "style_template_id": "template",
        "style_template_name": "模板",
        "style_template_version": "1",
        "custom_style": null
    }))
    .expect("object profile image input should accept documented fields");
    assert_eq!(object.object_profile_id, 1);
    assert_eq!(object.chapter_id, Some(2));

    let scene = serde_json::from_value::<SceneProfileImageInput>(json!({
        "scene_profile_id": 4,
        "chapter_id": null,
        "style": null,
        "config_id": null,
        "style_template_id": null,
        "style_template_name": null,
        "style_template_version": null,
        "custom_style": "手绘"
    }))
    .expect("scene profile image input should accept documented fields");
    assert_eq!(scene.scene_profile_id, 4);
    assert_eq!(scene.custom_style.as_deref(), Some("手绘"));
}
