use a4n_core::models::chapter::chapter_video_prompt;
use serde_json::Value;

use crate::video::{
    BOUNDARY_FRAME_PROMPT_CONTRACT_VERSION, CHAT_VIDEO_CONTRACT_VERSION,
    FIRST_FRAME_CHAIN_CONTRACT_VERSION, FRAME_SEGMENT_PLAN_CONTRACT_VERSION,
    STORYBOARD_PROMPT_CONTRACT_VERSION, STORYBOARD_VIDEO_PROMPT_CONTRACT_VERSION,
};
use a4n_core::prompt_language::metadata_matches_prompt_output_language;

#[cfg(test)]
use super::super::contracts::PromptGenerationContext;
use super::super::contracts::{PromptGenerationContextForWorkflow, VideoExecutionIdentityMatch};

const VIDEO_EXECUTION_BASE_IDENTITY_FIELDS: [&str; 9] = [
    "contract_epoch",
    "shooting_script_identity",
    "projection_sha256",
    "segment_plan_sha256",
    "unit_plan_sha256",
    "source_content_sha256",
    "style_sha256",
    "duration_secs",
    "aspect_ratio",
];

pub(in crate::video) fn video_execution_base_identity_matches(
    stored_metadata: Option<&Value>,
    expected_identity: &Value,
) -> bool {
    let Some(stored_identity) = stored_metadata
        .and_then(|metadata| metadata.get("video_execution_identity"))
        .and_then(Value::as_object)
    else {
        return false;
    };
    let Some(expected_identity) = expected_identity.as_object() else {
        return false;
    };
    VIDEO_EXECUTION_BASE_IDENTITY_FIELDS.iter().all(|field| {
        expected_identity
            .get(*field)
            .is_some_and(|expected| stored_identity.get(*field) == Some(expected))
    })
}

pub(in crate::video) fn reusable_rows_are_complete(
    rows: &[chapter_video_prompt::Model],
    expected_count: usize,
) -> bool {
    rows.len() == expected_count
        && rows.iter().enumerate().all(|(idx, row)| {
            let Ok(idx) = i32::try_from(idx) else {
                return false;
            };
            idx.checked_add(1)
                .is_some_and(|expected| row.shot_number == expected)
        })
}

#[cfg(test)]
pub(in crate::video::prompt_records) fn prompt_matches_generation_context(
    row: &chapter_video_prompt::Model,
    context: PromptGenerationContext<'_>,
) -> bool {
    let PromptGenerationContext {
        prompt_type,
        source_metadata,
        style,
        aspect_ratio,
        reference_ids,
        generation_asset_fingerprint,
        source_prompt_type,
        video_execution_identity,
        prompt_output_language,
    } = context;
    let Some(expected_workflow_mode) = expected_prompt_workflow_mode(prompt_type) else {
        return false;
    };
    prompt_matches_generation_context_for_workflow(
        row,
        PromptGenerationContextForWorkflow {
            expected_workflow_mode,
            prompt_type,
            source_metadata,
            style,
            aspect_ratio,
            reference_ids,
            generation_asset_fingerprint,
            source_prompt_type,
            video_execution_identity,
            prompt_output_language,
        },
    )
}

pub(in crate::video::prompt_records) fn prompt_matches_generation_context_for_workflow(
    row: &chapter_video_prompt::Model,
    context: PromptGenerationContextForWorkflow<'_>,
) -> bool {
    if row.prompt_type != context.prompt_type
        || row.workflow_mode != context.expected_workflow_mode
        || row.style.as_deref() != Some(context.style)
        || row.aspect_ratio.as_deref() != Some(context.aspect_ratio)
    {
        return false;
    }
    let Some(metadata) = row.metadata_json.as_ref() else {
        return false;
    };
    let matches_source = metadata
        .get("source")
        .and_then(|source| source.get("content_sha256"))
        == context.source_metadata.get("content_sha256");
    if !matches_source {
        return false;
    }
    let Some(expected_contract) =
        expected_contract_version(context.expected_workflow_mode, context.prompt_type)
    else {
        return false;
    };
    if metadata.get("contract_version").and_then(Value::as_str) != Some(expected_contract) {
        return false;
    }
    if prompt_type_uses_prompt_output_language(context.prompt_type)
        && !metadata_matches_prompt_output_language(
            row.metadata_json.as_ref(),
            context.prompt_output_language,
        )
    {
        return false;
    }
    if prompt_type_uses_generation_assets(context.prompt_type) {
        let Some(expected_fingerprint) = context.generation_asset_fingerprint else {
            return false;
        };
        if metadata.get("generation_asset_fingerprint") != Some(expected_fingerprint) {
            return false;
        }
    }
    if context.prompt_type == "video_prompt"
        && context.expected_workflow_mode == "chat_video"
        && !reference_ids_match(
            metadata.get("generation_reference_image_ids"),
            context.reference_ids,
        )
    {
        return false;
    }
    match context.video_execution_identity {
        #[cfg(test)]
        VideoExecutionIdentityMatch::NotApplicable => {}
        VideoExecutionIdentityMatch::Exact(expected_identity) => {
            if metadata.get("video_execution_identity") != Some(expected_identity) {
                return false;
            }
        }
        VideoExecutionIdentityMatch::Base(expected_identity) => {
            if !video_execution_base_identity_matches(Some(metadata), expected_identity) {
                return false;
            }
        }
    }

    context.source_prompt_type.is_none_or(|expected| {
        metadata.get("source_prompt_type").and_then(Value::as_str) == Some(expected)
    })
}

fn prompt_type_uses_prompt_output_language(prompt_type: &str) -> bool {
    a4n_core::prompt_language::prompt_type_uses_prompt_output_language(prompt_type)
}

fn prompt_type_uses_generation_assets(prompt_type: &str) -> bool {
    matches!(
        prompt_type,
        "frame_prompt"
            | "first_frame_prompt"
            | "chat_video_frame_prompt"
            | "storyboard_prompt"
            | "video_prompt"
    )
}

pub(super) fn expected_contract_version(
    expected_workflow_mode: &str,
    prompt_type: &str,
) -> Option<&'static str> {
    match (expected_workflow_mode, prompt_type) {
        ("boundary_frames", "frame_segment_plan" | "video_prompt") => {
            Some(FRAME_SEGMENT_PLAN_CONTRACT_VERSION)
        }
        ("boundary_frames", "frame_prompt") | ("chat_video", "chat_video_frame_prompt") => {
            Some(BOUNDARY_FRAME_PROMPT_CONTRACT_VERSION)
        }
        ("first_frame_chain", "first_frame_prompt" | "video_prompt") => {
            Some(FIRST_FRAME_CHAIN_CONTRACT_VERSION)
        }
        ("chat_video", "video_prompt") => Some(CHAT_VIDEO_CONTRACT_VERSION),
        ("storyboard", "storyboard_prompt") => Some(STORYBOARD_PROMPT_CONTRACT_VERSION),
        ("storyboard", "video_prompt") => Some(STORYBOARD_VIDEO_PROMPT_CONTRACT_VERSION),
        _ => None,
    }
}

#[cfg(test)]
pub(in crate::video::prompt_records) fn expected_prompt_workflow_mode(
    prompt_type: &str,
) -> Option<&'static str> {
    match prompt_type {
        "frame_segment_plan" | "frame_prompt" => Some("boundary_frames"),
        "first_frame_prompt" => Some("first_frame_chain"),
        "chat_video_frame_prompt" => Some("chat_video"),
        "storyboard_prompt" => Some("storyboard"),
        "video_prompt" => None,
        _ => None,
    }
}

pub(super) fn reference_ids_match(stored: Option<&Value>, expected: &Value) -> bool {
    reference_ids_from_value(stored) == reference_ids_from_value(Some(expected))
}

fn reference_ids_from_value(value: Option<&Value>) -> Option<Vec<i64>> {
    value?
        .as_array()?
        .iter()
        .map(Value::as_i64)
        .collect::<Option<Vec<_>>>()
}
