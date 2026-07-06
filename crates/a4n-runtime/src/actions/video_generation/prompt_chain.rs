use a4n_core::models::chapter::chapter_video_prompt;
use a4n_services::video::{
    BOUNDARY_FRAME_PROMPT_CONTRACT_VERSION, FIRST_FRAME_CHAIN_CONTRACT_VERSION,
    FRAME_SEGMENT_PLAN_CONTRACT_VERSION, STORYBOARD_PROMPT_CONTRACT_VERSION,
    STORYBOARD_VIDEO_PROMPT_CONTRACT_VERSION,
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
        _ => None,
    }
}
