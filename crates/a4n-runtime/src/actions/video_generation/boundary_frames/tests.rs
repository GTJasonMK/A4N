use a4n_core::models::chapter::chapter_video_prompt;
use a4n_orchestrator::video::{
    BOUNDARY_FRAME_PROMPT_CONTRACT_VERSION, FRAME_SEGMENT_PLAN_CONTRACT_VERSION,
};
use serde_json::{Value, json};

use super::lookup::frame_prompt_matches_video_prompt;
use crate::actions::video_generation::prompt_chain::image_generation_prompt_id;

fn test_time() -> chrono::NaiveDateTime {
    chrono::DateTime::from_timestamp(0, 0).unwrap().naive_utc()
}

fn prompt_model(
    id: i64,
    prompt_type: &str,
    workflow_mode: &str,
    shot_number: i32,
    mut metadata_json: Value,
) -> chapter_video_prompt::Model {
    metadata_json
        .as_object_mut()
        .expect("prompt metadata fixture must be an object")
        .entry("video_execution_identity")
        .or_insert_with(test_execution_identity);
    chapter_video_prompt::Model {
        id,
        project_id: 1,
        chapter_id: 1,
        style: Some("电影感写实".to_string()),
        prompt_type: prompt_type.to_string(),
        workflow_mode: workflow_mode.to_string(),
        shot_number,
        title: None,
        scene_summary: None,
        visual_prompt: "视觉提示词".to_string(),
        motion_prompt: None,
        negative_prompt: None,
        duration_secs: 6,
        aspect_ratio: Some("16:9".to_string()),
        reference_image_ids_json: None,
        status: "completed".to_string(),
        generated_video_id: None,
        metadata_json: Some(metadata_json),
        created_at: test_time(),
        updated_at: test_time(),
    }
}

fn test_execution_identity() -> Value {
    json!({
        "contract_epoch": a4n_core::video_execution::VIDEO_EXECUTION_CONTRACT_EPOCH,
        "shooting_script_identity": {"record_id": 1, "revision": 1},
        "projection_sha256": "projection",
        "segment_plan_sha256": "segments",
        "unit_plan_sha256": "units",
        "source_content_sha256": "source",
        "style_sha256": "style",
        "duration_secs": 6,
        "aspect_ratio": "16:9"
    })
}

#[test]
fn frame_prompt_match_uses_structural_chain_not_generation_settings() {
    let video = prompt_model(
        1,
        "video_prompt",
        "boundary_frames",
        1,
        json!({
            "contract_version": FRAME_SEGMENT_PLAN_CONTRACT_VERSION,
            "source_prompt_type": "frame_segment_plan",
            "source": {"content_sha256": "current"}
        }),
    );
    let matching = prompt_model(
        2,
        "frame_prompt",
        "boundary_frames",
        1,
        json!({
            "contract_version": BOUNDARY_FRAME_PROMPT_CONTRACT_VERSION,
            "source_prompt_type": "frame_segment_plan",
            "source": {"content_sha256": "current"}
        }),
    );
    let old_source_hash = prompt_model(
        3,
        "frame_prompt",
        "boundary_frames",
        1,
        json!({
            "contract_version": BOUNDARY_FRAME_PROMPT_CONTRACT_VERSION,
            "source_prompt_type": "frame_segment_plan",
            "source": {"content_sha256": "old"}
        }),
    );
    let wrong_workflow = prompt_model(
        4,
        "frame_prompt",
        "first_frame_chain",
        1,
        json!({
            "contract_version": BOUNDARY_FRAME_PROMPT_CONTRACT_VERSION,
            "source_prompt_type": "frame_segment_plan",
            "source": {"content_sha256": "current"}
        }),
    );
    let mut changed_generation_settings = matching.clone();
    changed_generation_settings.duration_secs = 10;
    changed_generation_settings.style = Some("另一种风格".to_string());
    changed_generation_settings.aspect_ratio = Some("9:16".to_string());
    let wrong_video_source_type = prompt_model(
        5,
        "video_prompt",
        "boundary_frames",
        1,
        json!({
            "contract_version": FRAME_SEGMENT_PLAN_CONTRACT_VERSION,
            "source_prompt_type": "unknown_segment_plan",
            "source": {"content_sha256": "current"}
        }),
    );

    assert!(frame_prompt_matches_video_prompt(&video, &matching));
    assert!(frame_prompt_matches_video_prompt(
        &video,
        &changed_generation_settings
    ));
    assert!(frame_prompt_matches_video_prompt(&video, &old_source_hash));
    assert!(!frame_prompt_matches_video_prompt(&video, &wrong_workflow));
    assert!(!frame_prompt_matches_video_prompt(
        &wrong_video_source_type,
        &matching
    ));
}

#[test]
fn frame_prompt_match_requires_contract_but_not_source_hash() {
    let video = prompt_model(
        1,
        "video_prompt",
        "boundary_frames",
        1,
        json!({
            "contract_version": FRAME_SEGMENT_PLAN_CONTRACT_VERSION,
            "source_prompt_type": "frame_segment_plan",
            "source": {"content_sha256": "current"}
        }),
    );
    let missing_contract = prompt_model(
        2,
        "frame_prompt",
        "boundary_frames",
        1,
        json!({
            "source_prompt_type": "frame_segment_plan",
            "source": {"content_sha256": "current"}
        }),
    );
    let missing_source_hash = prompt_model(
        3,
        "frame_prompt",
        "boundary_frames",
        1,
        json!({
            "contract_version": BOUNDARY_FRAME_PROMPT_CONTRACT_VERSION,
            "source_prompt_type": "frame_segment_plan"
        }),
    );

    assert!(!frame_prompt_matches_video_prompt(
        &video,
        &missing_contract
    ));
    assert!(frame_prompt_matches_video_prompt(
        &video,
        &missing_source_hash
    ));
}

#[test]
fn frame_prompt_match_rejects_changed_video_execution_identity() {
    let video = prompt_model(
        1,
        "video_prompt",
        "boundary_frames",
        1,
        json!({
            "contract_version": FRAME_SEGMENT_PLAN_CONTRACT_VERSION,
            "source_prompt_type": "frame_segment_plan"
        }),
    );
    let mut changed_identity = test_execution_identity();
    changed_identity["unit_plan_sha256"] = json!("changed-units");
    let frame = prompt_model(
        2,
        "frame_prompt",
        "boundary_frames",
        1,
        json!({
            "contract_version": BOUNDARY_FRAME_PROMPT_CONTRACT_VERSION,
            "source_prompt_type": "frame_segment_plan",
            "video_execution_identity": changed_identity
        }),
    );

    assert!(!frame_prompt_matches_video_prompt(&video, &frame));
}

#[test]
fn image_generation_prompt_id_requires_json_integer() {
    assert_eq!(
        image_generation_prompt_id(Some(&json!({
            "generation_context": {"prompt_id": 12}
        }))),
        Some(12)
    );
    assert_eq!(
        image_generation_prompt_id(Some(&json!({
            "generation_context": {"prompt_id": "12"}
        }))),
        None
    );
}
