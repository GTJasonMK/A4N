use super::chain::{prompt_same_frame_reference_chain, prompt_same_generation_chain, prompt_stage};
use super::frame_generation::{
    ensure_frame_prompt_generation_source_matches, ensure_frame_prompt_workflow_matches_type,
    frame_image_generation_source_for_prompt,
};
use crate::video_prompt::PromptStage;
use a4n_db_models::models::chapter::chapter_video_prompt;
use a4n_domain::image::{
    IMAGE_GENERATION_SOURCE_VIDEO_BOUNDARY_FRAME_PROMPT,
    IMAGE_GENERATION_SOURCE_VIDEO_FIRST_FRAME_PROMPT, image_generation_prompt_id,
    image_generation_shot_number, image_generation_source, image_generation_source_hash,
};
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
        timeline_item_id: None,
        metadata_json,
        created_at: now,
        updated_at: now,
    }
}

fn source_metadata(
    source_hash: &str,
    segment_plan_prompt_id: Option<i64>,
    segment_number: Option<i64>,
    recipe_fingerprint: Option<&str>,
) -> Value {
    let mut identity = json!({
        "contract_epoch": 5,
        "shooting_script_identity": "script-1",
        "projection_sha256": "projection-1",
        "segment_plan_sha256": "plan-1",
        "unit_plan_sha256": "unit-1",
        "source_content_sha256": source_hash,
        "style_sha256": "style-1",
        "duration_secs": 6,
        "aspect_ratio": "16:9"
    });
    if let Some(object) = identity.as_object_mut() {
        if let Some(value) = segment_plan_prompt_id {
            object.insert("segment_plan_prompt_id".to_string(), Value::from(value));
        }
        if let Some(value) = segment_number {
            object.insert("segment_number".to_string(), Value::from(value));
            object.insert(
                "source_shot_id".to_string(),
                Value::from(format!("shot-{value}")),
            );
            object.insert("segment_index_in_shot".to_string(), Value::from(1));
            object.insert(
                "segment_sha256".to_string(),
                Value::from(format!("segment-{value}")),
            );
        }
        if let Some(value) = recipe_fingerprint {
            object.insert("segment_recipe_fingerprint".to_string(), Value::from(value));
        }
        object.insert(
            "segment_identities".to_string(),
            json!([{
                "source_shot_id": "shot-1",
                "segment_index_in_shot": 1,
                "segment_sha256": "segment-1"
            }]),
        );
    }
    json!({
        "contract_version": "video_prompt_v2",
        "source_prompt_type": "frame_segment_plan",
        "generation_asset_fingerprint": "asset-fingerprint",
        "prompt_output_language": "zh-CN",
        "source": {"content_sha256": source_hash},
        "video_execution_identity": identity
    })
}

#[test]
fn generation_chain_requires_scope_style_and_source_identity() {
    let segment_plan = prompt(
        1,
        "frame_segment_plan",
        "shared_segment_plan",
        Some(source_metadata("same-source", None, None, None)),
    );
    let video_prompt = prompt(
        2,
        "video_prompt",
        "first_frame_chain",
        Some(source_metadata(
            "same-source",
            Some(1),
            Some(1),
            Some("recipe-1"),
        )),
    );

    assert_eq!(prompt_stage(&segment_plan), Some(PromptStage::SegmentPlan));
    assert_eq!(prompt_stage(&video_prompt), Some(PromptStage::Video));
    assert!(prompt_same_generation_chain(&segment_plan, &video_prompt));

    let mut different_source = video_prompt.clone();
    different_source.metadata_json = Some(source_metadata(
        "other-source",
        Some(1),
        Some(1),
        Some("recipe-1"),
    ));
    assert!(!prompt_same_generation_chain(
        &segment_plan,
        &different_source
    ));

    let different_segment = prompt(
        3,
        "video_prompt",
        "first_frame_chain",
        Some(source_metadata(
            "same-source",
            Some(1),
            Some(2),
            Some("recipe-1"),
        )),
    );
    assert!(!prompt_same_generation_chain(
        &video_prompt,
        &different_segment
    ));

    let different_recipe = prompt(
        4,
        "video_prompt",
        "first_frame_chain",
        Some(source_metadata(
            "same-source",
            Some(1),
            Some(1),
            Some("recipe-2"),
        )),
    );
    assert!(!prompt_same_generation_chain(
        &video_prompt,
        &different_recipe
    ));

    let mut renumbered = video_prompt.clone();
    renumbered.shot_number = 8;
    renumbered.metadata_json.as_mut().unwrap()["video_execution_identity"]["segment_number"] =
        json!(8);
    assert!(prompt_same_generation_chain(&video_prompt, &renumbered));

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
        Some(source_metadata(
            "same-source",
            Some(1),
            Some(1),
            Some("recipe-1"),
        )),
    );
    let previous = prompt(
        1,
        "frame_prompt",
        "boundary_frames",
        Some(source_metadata(
            "same-source",
            Some(1),
            Some(1),
            Some("recipe-1"),
        )),
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
        Some(IMAGE_GENERATION_SOURCE_VIDEO_BOUNDARY_FRAME_PROMPT)
    );
    assert_eq!(
        ensure_frame_prompt_generation_source_matches(
            &frame_prompt,
            IMAGE_GENERATION_SOURCE_VIDEO_BOUNDARY_FRAME_PROMPT,
        )
        .expect("matching source must pass"),
        IMAGE_GENERATION_SOURCE_VIDEO_BOUNDARY_FRAME_PROMPT
    );
    ensure_frame_prompt_workflow_matches_type(&frame_prompt).expect("matching workflow must pass");

    let wrong_workflow = prompt(2, "frame_prompt", "chat_video", None);
    assert!(ensure_frame_prompt_workflow_matches_type(&wrong_workflow).is_err());
    assert!(
        ensure_frame_prompt_generation_source_matches(
            &frame_prompt,
            IMAGE_GENERATION_SOURCE_VIDEO_FIRST_FRAME_PROMPT,
        )
        .is_err()
    );
}

#[test]
fn image_generation_metadata_requires_positive_typed_fields() {
    let metadata = json!({
        "generation_context": {
            "prompt_id": 12,
            "source": IMAGE_GENERATION_SOURCE_VIDEO_BOUNDARY_FRAME_PROMPT,
            "shot_number": 3,
            "source_content_sha256": "source-hash"
        }
    });
    assert_eq!(image_generation_prompt_id(Some(&metadata)), Some(12));
    assert_eq!(
        image_generation_source(Some(&metadata)),
        Some(IMAGE_GENERATION_SOURCE_VIDEO_BOUNDARY_FRAME_PROMPT)
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
