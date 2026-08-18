use crate::video_prompt::PromptStage;
use a4n_core::models::chapter::chapter_video_prompt;
use a4n_core::source_spans::metadata_source_content_sha256;
use serde_json::Value;

pub(in crate::video_prompt::dependencies) fn prompt_stage(
    row: &chapter_video_prompt::Model,
) -> Option<PromptStage> {
    match row.prompt_type.as_str() {
        chapter_video_prompt::PROMPT_TYPE_FRAME_SEGMENT_PLAN => Some(PromptStage::SegmentPlan),
        chapter_video_prompt::PROMPT_TYPE_FRAME
        | chapter_video_prompt::PROMPT_TYPE_FIRST_FRAME
        | chapter_video_prompt::PROMPT_TYPE_CHAT_VIDEO_FRAME
        | chapter_video_prompt::PROMPT_TYPE_STORYBOARD => Some(PromptStage::FramePrompt),
        chapter_video_prompt::PROMPT_TYPE_VIDEO => Some(PromptStage::Video),
        _ => None,
    }
}

fn prompt_workflow_mode(row: &chapter_video_prompt::Model) -> Option<&str> {
    match row.workflow_mode.as_str() {
        mode @ (chapter_video_prompt::WORKFLOW_MODE_BOUNDARY_FRAMES
        | chapter_video_prompt::WORKFLOW_MODE_FIRST_FRAME_CHAIN
        | chapter_video_prompt::WORKFLOW_MODE_CHAT_VIDEO
        | chapter_video_prompt::WORKFLOW_MODE_STORYBOARD) => Some(mode),
        _ => None,
    }
}

pub(in crate::video_prompt::dependencies) fn prompt_same_generation_chain(
    left: &chapter_video_prompt::Model,
    right: &chapter_video_prompt::Model,
) -> bool {
    if left.project_id != right.project_id || left.chapter_id != right.chapter_id {
        return false;
    }
    let left_stage = prompt_stage(left);
    let right_stage = prompt_stage(right);
    let shared_upstream = left_stage != right_stage
        && (prompt_is_shared_video_upstream(left) || prompt_is_shared_video_upstream(right));
    if !shared_upstream && prompt_workflow_mode(left) != prompt_workflow_mode(right) {
        return false;
    }
    if left.style != right.style || left.aspect_ratio != right.aspect_ratio {
        return false;
    }
    let source_matches = match (
        metadata_source_content_sha256(left.metadata_json.as_ref()),
        metadata_source_content_sha256(right.metadata_json.as_ref()),
    ) {
        (Some(left_hash), Some(right_hash)) => left_hash == right_hash,
        (None, None) => true,
        _ => false,
    };
    source_matches && prompt_execution_scope_matches(left, right)
}

fn prompt_execution_scope_matches(
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
    let base_matches = [
        "contract_epoch",
        "source_content_sha256",
        "style_sha256",
        "duration_secs",
        "aspect_ratio",
    ]
    .into_iter()
    .all(|field| left_identity.get(field) == right_identity.get(field));
    if !base_matches {
        return false;
    }
    let left_plan = left_identity
        .get("segment_plan_prompt_id")
        .and_then(Value::as_i64);
    let right_plan = right_identity
        .get("segment_plan_prompt_id")
        .and_then(Value::as_i64);
    if prompt_is_shared_video_upstream(left) {
        return right_plan == Some(left.id)
            && segment_identity_exists_in_plan(left_identity, right_identity);
    }
    if prompt_is_shared_video_upstream(right) {
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
                item.as_object().is_some_and(|item| {
                    identity_fields_match(
                        item,
                        segment_identity,
                        &["source_shot_id", "segment_index_in_shot", "segment_sha256"],
                    )
                })
            })
        })
}

fn identity_fields_match(
    left: &serde_json::Map<String, Value>,
    right: &serde_json::Map<String, Value>,
    fields: &[&str],
) -> bool {
    fields.iter().all(|field| {
        left.get(*field)
            .is_some_and(|value| right.get(*field) == Some(value))
    })
}

fn video_and_frame_prompt_pair(
    left: &chapter_video_prompt::Model,
    right: &chapter_video_prompt::Model,
) -> bool {
    (left.prompt_type == chapter_video_prompt::PROMPT_TYPE_VIDEO
        && is_frame_prompt(&right.prompt_type))
        || (right.prompt_type == chapter_video_prompt::PROMPT_TYPE_VIDEO
            && is_frame_prompt(&left.prompt_type))
}

fn is_frame_prompt(prompt_type: &str) -> bool {
    matches!(
        prompt_type,
        chapter_video_prompt::PROMPT_TYPE_FRAME
            | chapter_video_prompt::PROMPT_TYPE_FIRST_FRAME
            | chapter_video_prompt::PROMPT_TYPE_STORYBOARD
            | chapter_video_prompt::PROMPT_TYPE_CHAT_VIDEO_FRAME
    )
}

pub(in crate::video_prompt::dependencies) fn prompt_same_frame_reference_chain(
    current: &chapter_video_prompt::Model,
    previous: &chapter_video_prompt::Model,
) -> bool {
    if !prompt_same_generation_chain(current, previous)
        || current.prompt_type != previous.prompt_type
        || current.workflow_mode != previous.workflow_mode
    {
        return false;
    }
    let Some(current_metadata) = current.metadata_json.as_ref() else {
        return false;
    };
    let Some(previous_metadata) = previous.metadata_json.as_ref() else {
        return false;
    };
    if current_metadata.get("contract_version") != previous_metadata.get("contract_version") {
        return false;
    }
    if current_metadata.get("source_prompt_type") != previous_metadata.get("source_prompt_type") {
        return false;
    }
    if prompt_type_uses_generation_assets(&current.prompt_type)
        && current_metadata
            .get("generation_asset_fingerprint")
            .is_none()
    {
        return false;
    }
    if current_metadata.get("generation_asset_fingerprint")
        != previous_metadata.get("generation_asset_fingerprint")
    {
        return false;
    }
    current_metadata.get("prompt_output_language")
        == previous_metadata.get("prompt_output_language")
}

fn prompt_type_uses_generation_assets(prompt_type: &str) -> bool {
    matches!(
        prompt_type,
        chapter_video_prompt::PROMPT_TYPE_VIDEO
            | chapter_video_prompt::PROMPT_TYPE_FIRST_FRAME
            | chapter_video_prompt::PROMPT_TYPE_CHAT_VIDEO_FRAME
            | chapter_video_prompt::PROMPT_TYPE_STORYBOARD
            | chapter_video_prompt::PROMPT_TYPE_FRAME
    )
}

fn prompt_is_shared_video_upstream(row: &chapter_video_prompt::Model) -> bool {
    row.workflow_mode == chapter_video_prompt::WORKFLOW_MODE_SHARED_SEGMENT_PLAN
        && row.prompt_type == chapter_video_prompt::PROMPT_TYPE_FRAME_SEGMENT_PLAN
}

pub(in crate::video_prompt::dependencies) fn prompt_stage_label(
    row: &chapter_video_prompt::Model,
) -> &'static str {
    match row.prompt_type.as_str() {
        chapter_video_prompt::PROMPT_TYPE_FRAME_SEGMENT_PLAN => "视频片段规划",
        chapter_video_prompt::PROMPT_TYPE_FRAME => "边界帧提示词",
        chapter_video_prompt::PROMPT_TYPE_FIRST_FRAME => "首帧提示词",
        chapter_video_prompt::PROMPT_TYPE_CHAT_VIDEO_FRAME => "Chat 首尾帧提示词",
        chapter_video_prompt::PROMPT_TYPE_STORYBOARD => "分镜提示词",
        chapter_video_prompt::PROMPT_TYPE_VIDEO => match prompt_workflow_mode(row) {
            Some(chapter_video_prompt::WORKFLOW_MODE_FIRST_FRAME_CHAIN) => {
                "首帧串联视频提示词"
            }
            Some(chapter_video_prompt::WORKFLOW_MODE_CHAT_VIDEO) => "Chat 视频提示词",
            Some(chapter_video_prompt::WORKFLOW_MODE_STORYBOARD) => "分镜视频提示词",
            _ => "视频提示词",
        },
        _ => "视频提示词",
    }
}
