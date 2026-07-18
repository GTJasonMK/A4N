use serde_json::json;

use super::dimensions::{batch_image_aspect_ratio, image_dimensions_for_aspect_ratio};
use super::references::{
    attach_video_frame_reference_image_ids, ensure_batch_reference_image_backend,
};
use super::validation::ensure_batch_image_item_shape;

#[test]
fn image_dimensions_follow_explicit_aspect_ratio() {
    assert_eq!(
        image_dimensions_for_aspect_ratio(1024, 1024, "16:9").unwrap(),
        (1024, 576)
    );
    assert_eq!(
        image_dimensions_for_aspect_ratio(1024, 1024, "9:16").unwrap(),
        (576, 1024)
    );
    assert_eq!(
        image_dimensions_for_aspect_ratio(768, 1024, "1：1").unwrap(),
        (1024, 1024)
    );
}

#[test]
fn image_dimensions_reject_invalid_aspect_ratio() {
    let err = image_dimensions_for_aspect_ratio(1024, 1024, "wide")
        .expect_err("invalid aspect ratio should fail");
    assert!(err.contains("aspect_ratio 无效"));

    let err = image_dimensions_for_aspect_ratio(1024, 1024, "16:0")
        .expect_err("zero aspect ratio should fail");
    assert!(err.contains("必须大于 0"));
}

#[test]
fn batch_image_aspect_ratio_reads_prompt_snapshot_and_top_level_override() {
    let metadata = json!({
        "prompt_snapshot": {
            "aspect_ratio": "16:9"
        }
    });
    assert_eq!(
        batch_image_aspect_ratio(&json!({}), Some(&metadata)).unwrap(),
        Some("16:9".to_string())
    );
    assert_eq!(
        batch_image_aspect_ratio(&json!({ "aspect_ratio": "9:16" }), Some(&metadata)).unwrap(),
        Some("9:16".to_string())
    );
}

#[test]
fn batch_image_aspect_ratio_rejects_wrong_types_without_metadata_fallback() {
    let metadata = json!({
        "prompt_snapshot": {
            "aspect_ratio": "16:9"
        }
    });
    let error = batch_image_aspect_ratio(&json!({ "aspect_ratio": 169 }), Some(&metadata))
        .expect_err("top-level aspect_ratio type errors must not fall back to metadata");
    assert_eq!(error, "aspect_ratio 必须是字符串");

    let error = batch_image_aspect_ratio(
        &json!({}),
        Some(&json!({ "generation_context": { "aspect_ratio": 169 } })),
    )
    .expect_err("metadata aspect_ratio type errors must not fall back to defaults");
    assert_eq!(
        error,
        "metadata_json.generation_context.aspect_ratio 必须是字符串"
    );
}

#[test]
fn batch_image_item_shape_rejects_unknown_fields() {
    let err = ensure_batch_image_item_shape(
        0,
        &json!({
            "prompt": "draw",
            "image_type": "video_key_frame",
            "legacy_config": 1
        }),
    )
    .expect_err("unknown batch item field must fail");

    assert_eq!(err, "第 1 项包含未知字段：legacy_config");
}

#[test]
fn batch_image_item_shape_rejects_wrong_optional_field_types() {
    let err = ensure_batch_image_item_shape(
        1,
        &json!({
            "prompt": "draw",
            "image_type": "video_key_frame",
            "chapter_id": "12"
        }),
    )
    .expect_err("chapter_id string must not become project-level image");
    assert_eq!(err, "chapter_id 必须是整数");

    let err = ensure_batch_image_item_shape(
        1,
        &json!({
            "prompt": "draw",
            "image_type": "video_key_frame",
            "config_id": "2"
        }),
    )
    .expect_err("config_id string must not fall back to default config");
    assert_eq!(err, "config_id 必须是整数");

    let err = ensure_batch_image_item_shape(
        1,
        &json!({
            "prompt": "draw",
            "image_type": "video_key_frame",
            "aspect_ratio": 169
        }),
    )
    .expect_err("aspect_ratio number must not fall back to default dimensions");
    assert_eq!(err, "aspect_ratio 必须是字符串");
}

#[test]
fn batch_image_item_shape_accepts_documented_and_video_frame_fields() {
    ensure_batch_image_item_shape(
        0,
        &json!({
            "prompt": "draw",
            "image_type": "video_key_frame",
            "chapter_id": 12,
            "panel_id": "shot-1",
            "metadata_json": { "generation_context": { "source": "video_first_frame_prompt", "prompt_id": 9 } },
            "config_id": 3,
            "aspect_ratio": "16:9",
            "reference_image_ids": [1, 2]
        }),
    )
    .expect("known batch item fields should pass");
}

#[test]
fn video_frame_reference_ids_are_attached_to_generation_context() {
    let metadata = attach_video_frame_reference_image_ids(
        Some(json!({
            "generation_context": {
                "source": "video_boundary_frame_prompt",
                "prompt_id": 9
            }
        })),
        &[3, 5],
    )
    .expect("metadata should accept reference ids")
    .expect("metadata should be present");

    assert_eq!(
        metadata["generation_context"]["reference_image_ids"],
        json!([3, 5])
    );
}

#[test]
fn video_frame_reference_ids_reject_non_object_generation_context() {
    let err = attach_video_frame_reference_image_ids(
        Some(json!({
            "generation_context": "invalid"
        })),
        &[3],
    )
    .expect_err("generation_context must stay structured");

    assert!(err.contains("metadata_json.generation_context 必须是对象"));
}

#[test]
fn batch_reference_image_backend_accepts_only_real_image_input_backends() {
    for backend in [
        "openai-compatible-image-edit",
        "openai-chat-image",
        "cliproxy-image",
    ] {
        ensure_batch_reference_image_backend(backend)
            .expect("image input backend should accept reference_image_ids");
    }

    for backend in ["openai-compatible-image", "stable-diffusion", "comfyui"] {
        let err = ensure_batch_reference_image_backend(backend)
            .expect_err("text-only backend must reject reference_image_ids");
        assert!(err.contains("不支持真实参考图输入"));
        assert!(err.contains("移除 reference_image_ids"));
    }
}
