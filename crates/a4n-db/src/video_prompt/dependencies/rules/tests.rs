use super::chain::{prompt_same_frame_reference_chain, prompt_same_generation_chain, prompt_stage};
use super::frame_generation::{
    ensure_frame_prompt_generation_source_matches, ensure_frame_prompt_workflow_matches_type,
    frame_image_generation_source_for_prompt,
};
use super::image_metadata::{
    image_generation_prompt_id, image_generation_shot_number, image_generation_source,
    image_generation_source_hash,
};
use crate::video_prompt::PromptStage;
use a4n_core::models::chapter::chapter_video_prompt;
use serde_json::{Value, json};

fn prompt(
    id: i64,
    prompt_type: &str,
    workflow_mode: &str,
    metadata_json: Option<Value>,
) -> chapter_video_prompt::Model {
    let now = chrono::Utc::now().naive_utc();
    chapter_video_prompt::Model {
        id,
        project_id: 3,
        chapter_id: 7,
        style: Some("电影感写实".to_string()),
        prompt_type: prompt_type.to_string(),
        workflow_mode: workflow_mode.to_string(),
        shot_number: 1,
        title: None,
        scene_summary: None,
        visual_prompt: "测试提示词".to_string(),
        motion_prompt: None,
        negative_prompt: None,
        duration_secs: 6,
        aspect_ratio: Some("16:9".to_string()),
        reference_image_ids_json: None,
        status: "pending".to_string(),
        generated_video_id: None,
        metadata_json,
        created_at: now,
        updated_at: now,
    }
}

fn source_metadata(source_hash: &str) -> Value {
    json!({
        "contract_version": "video_prompt_v2",
        "source_prompt_type": "frame_segment_plan",
        "generation_asset_fingerprint": "asset-fingerprint",
        "prompt_output_language": "zh-CN",
        "source": {"content_sha256": source_hash}
    })
}

#[test]
fn generation_chain_requires_scope_style_and_source_identity() {
    let segment_plan = prompt(
        1,
        "frame_segment_plan",
        "boundary_frames",
        Some(source_metadata("same-source")),
    );
    let video_prompt = prompt(
        2,
        "video_prompt",
        "first_frame_chain",
        Some(source_metadata("same-source")),
    );

    assert_eq!(prompt_stage(&segment_plan), Some(PromptStage::SegmentPlan));
    assert_eq!(prompt_stage(&video_prompt), Some(PromptStage::Video));
    assert!(prompt_same_generation_chain(&segment_plan, &video_prompt));

    let mut different_source = video_prompt.clone();
    different_source.metadata_json = Some(source_metadata("other-source"));
    assert!(!prompt_same_generation_chain(
        &segment_plan,
        &different_source
    ));

    let mut different_project = video_prompt;
    different_project.project_id = 4;
    assert!(!prompt_same_generation_chain(
        &segment_plan,
        &different_project
    ));
}

#[test]
fn frame_reference_chain_requires_generation_asset_fingerprint() {
    let current = prompt(
        2,
        "frame_prompt",
        "boundary_frames",
        Some(source_metadata("same-source")),
    );
    let previous = prompt(
        1,
        "frame_prompt",
        "boundary_frames",
        Some(source_metadata("same-source")),
    );
    assert!(prompt_same_frame_reference_chain(&current, &previous));

    let mut missing_fingerprint = previous;
    missing_fingerprint
        .metadata_json
        .as_mut()
        .and_then(Value::as_object_mut)
        .expect("test metadata must be an object")
        .remove("generation_asset_fingerprint");
    assert!(!prompt_same_frame_reference_chain(
        &current,
        &missing_fingerprint
    ));
}

#[test]
fn frame_generation_contract_matches_prompt_type_and_workflow() {
    let frame_prompt = prompt(1, "frame_prompt", "boundary_frames", None);
    assert_eq!(
        frame_image_generation_source_for_prompt(&frame_prompt),
        Some("video_boundary_frame_prompt")
    );
    assert_eq!(
        ensure_frame_prompt_generation_source_matches(&frame_prompt, "video_boundary_frame_prompt")
            .expect("matching source must pass"),
        "video_boundary_frame_prompt"
    );
    ensure_frame_prompt_workflow_matches_type(&frame_prompt).expect("matching workflow must pass");

    let wrong_workflow = prompt(2, "frame_prompt", "chat_video", None);
    assert!(ensure_frame_prompt_workflow_matches_type(&wrong_workflow).is_err());
    assert!(
        ensure_frame_prompt_generation_source_matches(&frame_prompt, "video_first_frame_prompt")
            .is_err()
    );
}

#[test]
fn image_generation_metadata_requires_positive_typed_fields() {
    let metadata = json!({
        "generation_context": {
            "prompt_id": 12,
            "source": "video_boundary_frame_prompt",
            "shot_number": 3,
            "source_content_sha256": " source-hash "
        }
    });
    assert_eq!(image_generation_prompt_id(Some(&metadata)), Some(12));
    assert_eq!(
        image_generation_source(Some(&metadata)),
        Some("video_boundary_frame_prompt")
    );
    assert_eq!(image_generation_shot_number(Some(&metadata)), Some(3));
    assert_eq!(
        image_generation_source_hash(Some(&metadata)),
        Some("source-hash")
    );

    let invalid = json!({
        "generation_context": {
            "prompt_id": "12",
            "shot_number": 0,
            "source_content_sha256": "   "
        }
    });
    assert_eq!(image_generation_prompt_id(Some(&invalid)), None);
    assert_eq!(image_generation_shot_number(Some(&invalid)), None);
    assert_eq!(image_generation_source_hash(Some(&invalid)), None);
}
