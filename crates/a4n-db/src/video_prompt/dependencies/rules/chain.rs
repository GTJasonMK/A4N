use crate::video_prompt::PromptStage;
use a4n_core::models::chapter::chapter_video_prompt;
use serde_json::Value;

pub(in crate::video_prompt::dependencies) fn prompt_stage(
    row: &chapter_video_prompt::Model,
) -> Option<PromptStage> {
    match row.prompt_type.as_str() {
        "frame_segment_plan" => Some(PromptStage::SegmentPlan),
        "frame_prompt" | "first_frame_prompt" | "chat_video_frame_prompt" | "storyboard_prompt" => {
            Some(PromptStage::FramePrompt)
        }
        "video_prompt" => Some(PromptStage::Video),
        _ => None,
    }
}

fn prompt_workflow_mode(row: &chapter_video_prompt::Model) -> Option<&str> {
    match row.workflow_mode.as_str() {
        mode @ ("boundary_frames" | "first_frame_chain" | "chat_video" | "storyboard") => {
            Some(mode)
        }
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
    let source_matches = match (prompt_source_hash(left), prompt_source_hash(right)) {
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
        return right_plan == Some(left.id);
    }
    if prompt_is_shared_video_upstream(right) {
        return left_plan == Some(right.id);
    }
    left_plan.is_some()
        && left_plan == right_plan
        && left_identity.get("segment_number") == right_identity.get("segment_number")
        && left_identity.get("segment_recipe_fingerprint")
            == right_identity.get("segment_recipe_fingerprint")
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
        "video_prompt"
            | "first_frame_prompt"
            | "chat_video_frame_prompt"
            | "storyboard_prompt"
            | "frame_prompt"
    )
}

fn prompt_is_shared_video_upstream(row: &chapter_video_prompt::Model) -> bool {
    row.workflow_mode == "shared_segment_plan"
        && matches!(row.prompt_type.as_str(), "frame_segment_plan")
}

pub(in crate::video_prompt::dependencies) fn prompt_source_hash(
    row: &chapter_video_prompt::Model,
) -> Option<&str> {
    row.metadata_json
        .as_ref()
        .and_then(|metadata| metadata.get("source"))
        .and_then(|source| source.get("content_sha256"))
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
}

pub(in crate::video_prompt::dependencies) fn prompt_stage_label(
    row: &chapter_video_prompt::Model,
) -> &'static str {
    match row.prompt_type.as_str() {
        "frame_segment_plan" => "视频片段规划",
        "frame_prompt" => "边界帧提示词",
        "first_frame_prompt" => "首帧提示词",
        "chat_video_frame_prompt" => "Chat 首尾帧提示词",
        "storyboard_prompt" => "分镜提示词",
        "video_prompt" => match prompt_workflow_mode(row) {
            Some("first_frame_chain") => "首帧串联视频提示词",
            Some("chat_video") => "Chat 视频提示词",
            Some("storyboard") => "分镜视频提示词",
            _ => "视频提示词",
        },
        _ => "视频提示词",
    }
}
