use a4n_db_models::models::ai::generated_image;
use a4n_domain::image::{
    IMAGE_TYPE_OBJECT_MATERIAL, IMAGE_TYPE_SCENE_MATERIAL, IMAGE_TYPE_VIDEO_TAIL_FRAME,
};
use serde_json::json;

use super::super::references::validate_generated_image_ai_score;
use super::validation::{
    validate_generated_image_update_chapter_binding, validate_generated_image_update_clear_flags,
    validate_generated_image_update_image_type, validate_generated_image_update_panel_binding,
};
use crate::video::UpdateGeneratedImageRecordInput;

fn image_model() -> generated_image::Model {
    let now = chrono::NaiveDate::from_ymd_opt(2026, 1, 1)
        .unwrap()
        .and_hms_opt(0, 0, 0)
        .unwrap();
    generated_image::Model {
        id: 9,
        project_id: 1,
        image_type: "scene".to_string(),
        file_path: "images/scene.png".to_string(),
        prompt: "draw scene".to_string(),
        model: "image-model".to_string(),
        width: 1024,
        height: 576,
        file_size: Some(1234),
        ai_score: None,
        is_favorite: false,
        chapter_id: Some(2),
        panel_id: Some("panel-1".to_string()),
        metadata_json: Some(json!({"source": "test"})),
        created_at: now,
    }
}

#[test]
fn validate_finite_f32_rejects_non_finite_scores() {
    assert_eq!(validate_generated_image_ai_score(0.8).unwrap(), 0.8);
    for value in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
        let err = validate_generated_image_ai_score(value)
            .expect_err("non-finite score must be rejected");
        assert_eq!(err.to_string(), "ai_score 必须是有限数");
    }
}

#[test]
fn update_record_clear_flags_reject_field_value_conflicts() {
    let cases = [
        (
            UpdateGeneratedImageRecordInput {
                chapter_id: Some(1),
                clear_chapter_id: true,
                ..Default::default()
            },
            "clear_chapter_id 不能与 chapter_id 同时使用",
        ),
        (
            UpdateGeneratedImageRecordInput {
                panel_id: Some("panel-1".to_string()),
                clear_panel_id: true,
                ..Default::default()
            },
            "clear_panel_id 不能与 panel_id 同时使用",
        ),
        (
            UpdateGeneratedImageRecordInput {
                ai_score: Some(0.8),
                clear_ai_score: true,
                ..Default::default()
            },
            "clear_ai_score 不能与 ai_score 同时使用",
        ),
    ];

    for (input, expected) in cases {
        let err = validate_generated_image_update_clear_flags(&input)
            .expect_err("conflicting update input must be rejected");
        assert_eq!(err.to_string(), expected);
    }
}

#[test]
fn update_record_image_type_rejects_system_managed_type_transitions() {
    validate_generated_image_update_image_type("manga_page", " manga_page ")
        .expect("same managed image type should be allowed");
    validate_generated_image_update_image_type("custom_reference", "custom_board")
        .expect("custom image type edits should be allowed");

    let cases = [
        ("manga_page", "custom_reference"),
        ("custom_reference", "manga_page"),
        (IMAGE_TYPE_SCENE_MATERIAL, IMAGE_TYPE_OBJECT_MATERIAL),
        (IMAGE_TYPE_VIDEO_TAIL_FRAME, "project_reference"),
    ];
    for (current, next) in cases {
        let err = validate_generated_image_update_image_type(current, next)
            .expect_err("managed image type transition must be rejected");
        assert_eq!(
            err.to_string(),
            format!("系统管理图片类型不能通过通用 update_image 修改：{current} -> {next}")
        );
    }
}

#[test]
fn update_record_binding_rejects_system_managed_field_changes() {
    let image = image_model();
    validate_generated_image_update_chapter_binding(&image, Some(2))
        .expect("same managed chapter binding should be allowed");
    validate_generated_image_update_panel_binding(&image, Some("panel-1"))
        .expect("same managed panel binding should be allowed");

    let chapter_err = validate_generated_image_update_chapter_binding(&image, Some(3))
        .expect_err("managed chapter binding change must be rejected");
    assert_eq!(
        chapter_err.to_string(),
        "系统管理图片的 chapter_id 不能通过通用 update_image 修改：image_id=9 Some(2) -> Some(3)"
    );

    let clear_chapter_err = validate_generated_image_update_chapter_binding(&image, None)
        .expect_err("managed chapter binding clear must be rejected");
    assert_eq!(
        clear_chapter_err.to_string(),
        "系统管理图片的 chapter_id 不能通过通用 update_image 修改：image_id=9 Some(2) -> None"
    );

    let panel_err = validate_generated_image_update_panel_binding(&image, Some("panel-2"))
        .expect_err("managed panel binding change must be rejected");
    assert_eq!(
        panel_err.to_string(),
        "系统管理图片的 panel_id 不能通过通用 update_image 修改：image_id=9 Some(\"panel-1\") -> Some(\"panel-2\")"
    );

    let clear_panel_err = validate_generated_image_update_panel_binding(&image, None)
        .expect_err("managed panel binding clear must be rejected");
    assert_eq!(
        clear_panel_err.to_string(),
        "系统管理图片的 panel_id 不能通过通用 update_image 修改：image_id=9 Some(\"panel-1\") -> None"
    );
}

#[test]
fn update_record_binding_allows_custom_field_changes() {
    let mut image = image_model();
    image.image_type = "custom_reference".to_string();

    validate_generated_image_update_chapter_binding(&image, Some(3))
        .expect("custom chapter binding change should be allowed");
    validate_generated_image_update_chapter_binding(&image, None)
        .expect("custom chapter binding clear should be allowed");
    validate_generated_image_update_panel_binding(&image, Some("panel-2"))
        .expect("custom panel binding change should be allowed");
    validate_generated_image_update_panel_binding(&image, None)
        .expect("custom panel binding clear should be allowed");
}
