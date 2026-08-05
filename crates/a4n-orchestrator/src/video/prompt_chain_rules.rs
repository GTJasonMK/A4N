//! 视频提示词链身份与合同匹配规则（无 DB、无 Host）。

use a4n_core::models::chapter::chapter_video_prompt;
use serde_json::Value;

use super::{
    BOUNDARY_FRAME_PROMPT_CONTRACT_VERSION, CHAT_VIDEO_CONTRACT_VERSION,
    FIRST_FRAME_CHAIN_CONTRACT_VERSION, FRAME_SEGMENT_PLAN_CONTRACT_VERSION,
    STORYBOARD_PROMPT_CONTRACT_VERSION, STORYBOARD_VIDEO_PROMPT_CONTRACT_VERSION,
};

pub fn prompt_chain_matches(
    workflow_mode: &str,
    left: &chapter_video_prompt::Model,
    right: &chapter_video_prompt::Model,
) -> bool {
    prompt_workflow_matches_chain(left, workflow_mode)
        && prompt_workflow_matches_chain(right, workflow_mode)
        && current_chain_contract_versions_match(workflow_mode, left, right)
        && video_execution_identity_matches(left, right)
}

fn prompt_workflow_matches_chain(
    prompt: &chapter_video_prompt::Model,
    workflow_mode: &str,
) -> bool {
    prompt.workflow_mode == workflow_mode
        || (prompt.prompt_type == "frame_segment_plan"
            && prompt.workflow_mode == "shared_segment_plan")
}

pub fn prompt_source_type_matches(prompt: &chapter_video_prompt::Model, expected: &str) -> bool {
    metadata_text(prompt.metadata_json.as_ref(), "source_prompt_type") == Some(expected)
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
    let base_matches = identity_fields_match(
        left_identity,
        right_identity,
        &[
            "source_content_sha256",
            "style_sha256",
            "duration_secs",
            "aspect_ratio",
        ],
    );
    base_matches && segment_execution_scope_matches(left, right, left_identity, right_identity)
}

fn segment_execution_scope_matches(
    left: &chapter_video_prompt::Model,
    right: &chapter_video_prompt::Model,
    left_identity: &serde_json::Map<String, Value>,
    right_identity: &serde_json::Map<String, Value>,
) -> bool {
    let left_plan = left_identity
        .get("segment_plan_prompt_id")
        .and_then(Value::as_i64);
    let right_plan = right_identity
        .get("segment_plan_prompt_id")
        .and_then(Value::as_i64);
    if left.prompt_type == "frame_segment_plan" {
        return right_plan == Some(left.id)
            && segment_identity_exists_in_plan(left_identity, right_identity);
    }
    if right.prompt_type == "frame_segment_plan" {
        return left_plan == Some(right.id)
            && segment_identity_exists_in_plan(right_identity, left_identity);
    }
    let same_segment = left_plan.is_some()
        && left_plan == right_plan
        && identity_fields_match(
            left_identity,
            right_identity,
            &["source_shot_id", "segment_index_in_shot", "segment_sha256"],
        );
    same_segment
        && (video_and_frame_prompt_pair(left, right)
            || left_identity.get("segment_recipe_fingerprint")
                == right_identity.get("segment_recipe_fingerprint"))
}

fn segment_identity_exists_in_plan(
    plan_identity: &serde_json::Map<String, Value>,
    segment_identity: &serde_json::Map<String, Value>,
) -> bool {
    plan_identity
        .get("segment_identities")
        .and_then(Value::as_array)
        .is_some_and(|items| {
            items.iter().any(|item| {
                let Some(item) = item.as_object() else {
                    return false;
                };
                identity_fields_match(
                    item,
                    segment_identity,
                    &["source_shot_id", "segment_index_in_shot", "segment_sha256"],
                )
            })
        })
}

fn identity_fields_match(
    left: &serde_json::Map<String, Value>,
    right: &serde_json::Map<String, Value>,
    fields: &[&str],
) -> bool {
    fields.iter().all(|key| {
        left.get(*key)
            .is_some_and(|value| right.get(*key) == Some(value))
    })
}

fn video_and_frame_prompt_pair(
    left: &chapter_video_prompt::Model,
    right: &chapter_video_prompt::Model,
) -> bool {
    (left.prompt_type == "video_prompt" && is_frame_prompt(&right.prompt_type))
        || (right.prompt_type == "video_prompt" && is_frame_prompt(&left.prompt_type))
}

fn is_frame_prompt(prompt_type: &str) -> bool {
    matches!(
        prompt_type,
        "frame_prompt" | "first_frame_prompt" | "storyboard_prompt" | "chat_video_frame_prompt"
    )
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
        ("boundary_frames", "video_prompt") => Some(FRAME_SEGMENT_PLAN_CONTRACT_VERSION),
        ("shared_segment_plan", "frame_segment_plan") => Some(FRAME_SEGMENT_PLAN_CONTRACT_VERSION),
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
