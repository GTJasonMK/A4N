use super::trace::{image_generation_parsed_output, image_generation_response_summary};
use a4n_db_models::models::ai::generated_image;
use serde_json::json;

fn image_model() -> generated_image::Model {
    generated_image::Model {
        id: 17,
        project_id: 3,
        image_type: "scene".to_string(),
        file_path: "images/scene.png".to_string(),
        prompt: "draw scene".to_string(),
        model: "image-model".to_string(),
        width: 1024,
        height: 576,
        file_size: Some(2048),
        ai_score: None,
        is_favorite: false,
        chapter_id: Some(5),
        panel_id: Some("panel-1".to_string()),
        metadata_json: Some(json!({"source": "test"})),
        created_at: chrono::NaiveDate::from_ymd_opt(2026, 1, 1)
            .expect("valid fixture date")
            .and_hms_opt(0, 0, 0)
            .expect("valid fixture time"),
    }
}

#[test]
fn image_generation_summary_merges_generated_fields() {
    let summary = image_generation_response_summary(
        Some(json!({"provider": "test", "generated_image_id": 1})),
        &image_model(),
    )
    .expect("object summary should be accepted");

    assert_eq!(
        summary,
        Some(json!({
            "provider": "test",
            "generated_image_id": 17,
            "file_path": "images/scene.png",
            "model": "image-model",
            "file_size": 2048
        }))
    );
}

#[test]
fn image_generation_summary_rejects_non_object() {
    let error = image_generation_response_summary(Some(json!([])), &image_model())
        .expect_err("non-object summary must be rejected");
    assert_eq!(
        error.to_string(),
        "图片生成运行记录 response_summary_json 必须是对象"
    );
}

#[test]
fn image_generation_parsed_output_preserves_existing_fields() {
    let output = image_generation_parsed_output(
        Some(json!({"provider_output": {"request_id": "req-1"}})),
        &image_model(),
    )
    .expect("object parsed output should be accepted");

    assert_eq!(
        output,
        Some(json!({
            "provider_output": {"request_id": "req-1"},
            "generated_image": {
                "generated_image_id": 17,
                "project_id": 3,
                "image_type": "scene",
                "file_path": "images/scene.png",
                "prompt": "draw scene",
                "model": "image-model",
                "width": 1024,
                "height": 576,
                "file_size": 2048,
                "chapter_id": 5,
                "panel_id": "panel-1",
                "metadata": {"source": "test"}
            }
        }))
    );
}

#[test]
fn image_generation_parsed_output_rejects_non_object() {
    let error = image_generation_parsed_output(Some(json!("invalid")), &image_model())
        .expect_err("non-object parsed output must be rejected");
    assert_eq!(
        error.to_string(),
        "图片生成运行记录 parsed_output_json 必须是对象"
    );
}
