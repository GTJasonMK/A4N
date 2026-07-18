use super::files::{persist_generated_image_file, required_filename_segment};
use super::metadata::{ImageGenerationTrace, attach_image_generation_trace};
use super::provider::test_image_request_aspect_ratio;
use a4n_core::models::ai::image_gen_config;
use serde_json::json;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

fn unique_temp_dir(prefix: &str) -> PathBuf {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock after UNIX_EPOCH")
        .as_nanos();
    std::env::temp_dir().join(format!("{prefix}_{}_{}", std::process::id(), stamp))
}

#[test]
fn generated_image_file_persistence_keeps_output_and_reports_actual_size() {
    let data_dir = unique_temp_dir("a4n_generated_image_persist");

    let (path, relative_path, file_size) = persist_generated_image_file(
        &data_dir,
        7,
        Some(3),
        "scene",
        Some("panel-1"),
        b"generated-image",
        "png",
        "Provider 图片文件",
    )
    .expect("persist generated image file");

    assert!(path.is_file());
    assert!(relative_path.starts_with("projects/7/images/chapter_3/scene_panel-1_"));
    assert!(relative_path.ends_with(".png"));
    assert_eq!(file_size, 15);
    let _ = std::fs::remove_dir_all(&data_dir);
}

fn image_config() -> image_gen_config::Model {
    let now = chrono::Utc::now().naive_utc();
    image_gen_config::Model {
        id: 1,
        name: "primary".to_string(),
        backend: "openai-compatible-image".to_string(),
        base_url: Some("http://localhost:3000".to_string()),
        api_key_encrypted: Some("encrypted".to_string()),
        model: "image-model".to_string(),
        max_concurrent: 1,
        default_width: 1024,
        default_height: 1024,
        provider_options_json: None,
        is_default: true,
        created_at: now,
        updated_at: now,
    }
}

fn trace<'a>(config: &'a image_gen_config::Model) -> ImageGenerationTrace<'a> {
    ImageGenerationTrace {
        command: "generate_image",
        config,
        image_type: "scene",
        prompt: "test prompt",
        provider_model: "provider-model",
        chapter_id: Some(7),
        panel_id: Some("panel-1"),
        width: 1024,
        height: 576,
        generated_at: chrono::Utc::now().naive_utc(),
    }
}

#[test]
fn filename_segment_rejects_empty_or_path_like_values() {
    let empty =
        required_filename_segment("  ", "kind").expect_err("empty filename segment should fail");
    assert!(empty.contains("不能为空"));

    let slash = required_filename_segment("../scene", "kind")
        .expect_err("path-like filename segment should fail");
    assert!(slash.contains("只能包含"));
}

#[test]
fn filename_segment_accepts_safe_ascii_values() {
    let segment = required_filename_segment("manga_page-01", "kind")
        .expect("safe filename segment should pass");

    assert_eq!(segment, "manga_page-01");
}

#[test]
fn image_request_aspect_ratio_reduces_dimensions() {
    assert_eq!(test_image_request_aspect_ratio(1024, 576), "16:9");
    assert_eq!(test_image_request_aspect_ratio(576, 1024), "9:16");
}

#[test]
fn attach_image_generation_trace_requires_object_metadata() {
    let config = image_config();
    let err = attach_image_generation_trace(Some(json!(["legacy"])), trace(&config))
        .expect_err("non-object image metadata should fail explicitly");

    assert!(err.contains("metadata_json 必须是对象"));
}

#[test]
fn attach_image_generation_trace_preserves_object_metadata() {
    let config = image_config();
    let metadata = attach_image_generation_trace(
        Some(json!({
            "reference_image_ids": [1, 2],
        })),
        trace(&config),
    )
    .expect("object image metadata should accept")
    .expect("image metadata should be present");

    assert_eq!(metadata["metadata_schema_version"], 1);
    assert_eq!(metadata["reference_image_ids"], json!([1, 2]));
    assert_eq!(metadata["generation"]["command"], "generate_image");
}
