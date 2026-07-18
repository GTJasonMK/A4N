use a4n_core::models::chapter::chapter_video_prompt;
use a4n_orchestrator::video::{
    BOUNDARY_FRAME_PROMPT_CONTRACT_VERSION, CHAT_VIDEO_CONTRACT_VERSION,
    FIRST_FRAME_CHAIN_CONTRACT_VERSION, FRAME_SEGMENT_PLAN_CONTRACT_VERSION,
    STORYBOARD_PROMPT_CONTRACT_VERSION, STORYBOARD_VIDEO_PROMPT_CONTRACT_VERSION,
};
use serde_json::Value;

pub(super) fn prompt_chain_matches(
    workflow_mode: &str,
    left: &chapter_video_prompt::Model,
    right: &chapter_video_prompt::Model,
) -> bool {
    left.workflow_mode == workflow_mode
        && right.workflow_mode == workflow_mode
        && current_chain_contract_versions_match(workflow_mode, left, right)
        && video_execution_identity_matches(left, right)
}

fn video_execution_identity_matches(
    left: &chapter_video_prompt::Model,
    right: &chapter_video_prompt::Model,
) -> bool {
    let Some(left_identity) = left
        .metadata_json
        .as_ref()
        .and_then(|metadata| metadata.get("video_execution_identity"))
        .and_then(Value::as_object)
    else {
        return false;
    };
    let Some(right_identity) = right
        .metadata_json
        .as_ref()
        .and_then(|metadata| metadata.get("video_execution_identity"))
        .and_then(Value::as_object)
    else {
        return false;
    };
    if left_identity.get("contract_epoch").and_then(Value::as_u64)
        != Some(a4n_core::video_execution::VIDEO_EXECUTION_CONTRACT_EPOCH as u64)
        || right_identity.get("contract_epoch").and_then(Value::as_u64)
            != Some(a4n_core::video_execution::VIDEO_EXECUTION_CONTRACT_EPOCH as u64)
    {
        return false;
    }
    [
        "shooting_script_identity",
        "projection_sha256",
        "segment_plan_sha256",
        "unit_plan_sha256",
        "source_content_sha256",
        "style_sha256",
        "duration_secs",
        "aspect_ratio",
    ]
    .into_iter()
    .all(|key| left_identity.get(key) == right_identity.get(key))
}

pub(super) fn prompt_source_type_matches(
    prompt: &chapter_video_prompt::Model,
    expected: &str,
) -> bool {
    metadata_text(prompt.metadata_json.as_ref(), "source_prompt_type") == Some(expected)
}

pub(super) fn image_generation_prompt_id(metadata: Option<&Value>) -> Option<i64> {
    let raw = metadata
        .and_then(|metadata| metadata.get("generation_context"))
        .and_then(|context| context.get("prompt_id"))?;
    raw.as_i64().filter(|id| *id > 0)
}

pub(super) fn image_generation_shot_number(metadata: Option<&Value>) -> Option<i32> {
    let raw = metadata
        .and_then(|metadata| metadata.get("generation_context"))
        .and_then(|context| context.get("shot_number"))?;
    raw.as_i64()
        .and_then(|value| i32::try_from(value).ok())
        .filter(|value| *value > 0)
}

pub(super) fn image_generation_source(metadata: Option<&Value>) -> Option<&str> {
    metadata
        .and_then(|metadata| metadata.get("generation_context"))
        .and_then(|context| context.get("source"))
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
}

fn metadata_text<'a>(metadata: Option<&'a Value>, key: &str) -> Option<&'a str> {
    metadata
        .and_then(|metadata| metadata.get(key))
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
}

fn current_chain_contract_versions_match(
    workflow_mode: &str,
    left: &chapter_video_prompt::Model,
    right: &chapter_video_prompt::Model,
) -> bool {
    let Some(left_expected) = expected_chain_contract_version(workflow_mode, &left.prompt_type)
    else {
        return false;
    };
    let Some(right_expected) = expected_chain_contract_version(workflow_mode, &right.prompt_type)
    else {
        return false;
    };
    metadata_text(left.metadata_json.as_ref(), "contract_version") == Some(left_expected)
        && metadata_text(right.metadata_json.as_ref(), "contract_version") == Some(right_expected)
}

fn expected_chain_contract_version(workflow_mode: &str, prompt_type: &str) -> Option<&'static str> {
    match (workflow_mode, prompt_type) {
        ("boundary_frames", "video_prompt" | "frame_segment_plan") => {
            Some(FRAME_SEGMENT_PLAN_CONTRACT_VERSION)
        }
        ("boundary_frames", "frame_prompt") => Some(BOUNDARY_FRAME_PROMPT_CONTRACT_VERSION),
        ("first_frame_chain", "video_prompt" | "first_frame_prompt") => {
            Some(FIRST_FRAME_CHAIN_CONTRACT_VERSION)
        }
        ("storyboard", "storyboard_prompt") => Some(STORYBOARD_PROMPT_CONTRACT_VERSION),
        ("storyboard", "video_prompt") => Some(STORYBOARD_VIDEO_PROMPT_CONTRACT_VERSION),
        ("chat_video", "chat_video_frame_prompt") => Some(BOUNDARY_FRAME_PROMPT_CONTRACT_VERSION),
        ("chat_video", "video_prompt") => Some(CHAT_VIDEO_CONTRACT_VERSION),
        _ => None,
    }
}
