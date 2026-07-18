use super::{
    canvas_node_references_generated_image, canvas_node_references_generated_video,
    clear_reset_generated_ref_payload, reset_missing_canvas_ref_for_node,
};
use crate::{canvas::references::MissingCanvasRef, project::FreeformGeneratedResetTargets};
use a4n_core::models::canvas::canvas_node;
use chrono::Utc;
use serde_json::{Value, json};

fn node(ref_type: Option<&str>, ref_id: Option<i64>, config_json: Value) -> canvas_node::Model {
    let now = Utc::now().naive_utc();
    canvas_node::Model {
        id: 1,
        graph_id: 1,
        node_kind: "GenerateVideoNode".to_string(),
        ref_type: ref_type.map(str::to_string),
        ref_id,
        config_json,
        position_json: json!({}),
        state: "ready".to_string(),
        created_at: now,
        updated_at: now,
    }
}

#[test]
fn reset_detection_prioritizes_direct_ref_before_config_refs() {
    let node = node(
        Some("media_asset"),
        Some(7),
        json!({"generated_image_id": 10}),
    );
    let targets = FreeformGeneratedResetTargets {
        image_ids: vec![10],
        media_asset_ids: vec![7],
        ..Default::default()
    };

    assert_eq!(
        reset_missing_canvas_ref_for_node(&node, &targets),
        Some(MissingCanvasRef {
            ref_type: "media_asset",
            ref_id: 7,
            clear_node_ref: true,
        })
    );
}

#[test]
fn reset_detection_reports_config_ref_without_clearing_node_ref() {
    let node = node(None, None, json!({"reference_image_ids": [10, 12]}));
    let targets = FreeformGeneratedResetTargets {
        image_ids: vec![10],
        ..Default::default()
    };

    assert_eq!(
        reset_missing_canvas_ref_for_node(&node, &targets),
        Some(MissingCanvasRef {
            ref_type: "generated_image",
            ref_id: 10,
            clear_node_ref: false,
        })
    );
}

#[test]
fn reset_payload_removes_only_targeted_ids() {
    let mut config = json!({
        "generated_image_id": 10,
        "source_image_id": 11,
        "reference_image_ids": [10, 12],
        "generated_video_id": 20,
        "video_ids": [20, 21],
        "generation_spec_id": 30,
        "prompt_artifact_ids": [40, 41],
        "media_asset_id": 50,
        "title": "keep"
    });
    let targets = FreeformGeneratedResetTargets {
        image_ids: vec![10],
        video_ids: vec![20],
        generation_spec_ids: vec![30],
        prompt_artifact_ids: vec![40],
        media_asset_ids: vec![50],
        ..Default::default()
    };

    clear_reset_generated_ref_payload(config.as_object_mut().expect("object config"), &targets);

    assert!(config.get("generated_image_id").is_none());
    assert_eq!(config["source_image_id"], json!(11));
    assert_eq!(config["reference_image_ids"], json!([12]));
    assert!(config.get("generated_video_id").is_none());
    assert_eq!(config["video_ids"], json!([21]));
    assert!(config.get("generation_spec_id").is_none());
    assert_eq!(config["prompt_artifact_ids"], json!([41]));
    assert!(config.get("media_asset_id").is_none());
    assert_eq!(config["title"], json!("keep"));
}

#[test]
fn reference_predicates_accept_legacy_and_config_shapes() {
    let legacy_video = node(Some("legacy_generated_video"), Some(20), json!({}));
    let config_video = node(None, None, json!({"video_ids": [20]}));
    let legacy_image = node(Some("legacy_generated_image"), Some(10), json!({}));
    let config_image = node(None, None, json!({"reference_image_ids": [10]}));

    assert!(canvas_node_references_generated_video(&legacy_video, 20));
    assert!(canvas_node_references_generated_video(&config_video, 20));
    assert!(canvas_node_references_generated_image(&legacy_image, &[10]));
    assert!(canvas_node_references_generated_image(&config_image, &[10]));
}
