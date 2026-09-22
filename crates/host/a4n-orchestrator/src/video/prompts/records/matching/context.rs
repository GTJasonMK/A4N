use a4n_db_models::models::chapter::chapter_video_prompt;
use serde_json::Value;

use crate::video::{
    BOUNDARY_FRAME_PROMPT_CONTRACT_VERSION, CHAT_VIDEO_CONTRACT_VERSION,
    FIRST_FRAME_CHAIN_CONTRACT_VERSION, FRAME_SEGMENT_PLAN_CONTRACT_VERSION,
    STORYBOARD_PROMPT_CONTRACT_VERSION, STORYBOARD_VIDEO_PROMPT_CONTRACT_VERSION,
};
use a4n_domain::prompt_language::metadata_matches_prompt_output_language;

use super::super::contracts::PromptGenerationContextForWorkflow;

const VIDEO_EXECUTION_BASE_IDENTITY_FIELDS: [&str; 5] = [
    "contract_epoch",
    "source_content_sha256",
    "style_sha256",
    "duration_secs",
    "aspect_ratio",
];
const VIDEO_EXECUTION_LINEAGE_BASE_IDENTITY_FIELDS: [&str; 4] = [
    "contract_epoch",
    "source_content_sha256",
    "style_sha256",
    "aspect_ratio",
];
const VIDEO_EXECUTION_SEGMENT_IDENTITY_FIELDS: [&str; 5] = [
    "segment_plan_prompt_id",
    "source_shot_id",
    "segment_index_in_shot",
    "segment_sha256",
    "segment_recipe_fingerprint",
];

pub(in crate::video) fn video_execution_segment_identity_matches(
    stored_metadata: Option<&Value>,
    expected_identity: &Value,
) -> bool {
    video_execution_identity_fields_match(
        stored_metadata,
        expected_identity,
        &VIDEO_EXECUTION_BASE_IDENTITY_FIELDS,
    ) && video_execution_identity_fields_match(
        stored_metadata,
        expected_identity,
        &VIDEO_EXECUTION_SEGMENT_IDENTITY_FIELDS,
    )
}

pub(in crate::video) fn video_execution_segment_lineage_matches(
    stored_metadata: Option<&Value>,
    expected_identity: &Value,
) -> bool {
    video_execution_identity_fields_match(
        stored_metadata,
        expected_identity,
        &VIDEO_EXECUTION_LINEAGE_BASE_IDENTITY_FIELDS,
    ) && video_execution_identity_fields_match(
        stored_metadata,
        expected_identity,
        &VIDEO_EXECUTION_SEGMENT_IDENTITY_FIELDS,
    )
}

fn video_execution_identity_fields_match(
    stored_metadata: Option<&Value>,
    expected_identity: &Value,
    fields: &[&str],
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
    fields.iter().all(|field| {
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

pub(in crate::video::prompts::records) fn prompt_matches_generation_context_for_workflow(
    row: &chapter_video_prompt::Model,
    context: PromptGenerationContextForWorkflow<'_>,
) -> bool {
    let expected_storage_workflow_mode =
        if context.prompt_type == chapter_video_prompt::PROMPT_TYPE_FRAME_SEGMENT_PLAN {
            chapter_video_prompt::WORKFLOW_MODE_SHARED_SEGMENT_PLAN
        } else {
            context.expected_workflow_mode
        };
    if row.prompt_type != context.prompt_type
        || row.workflow_mode != expected_storage_workflow_mode
        || row.style.as_deref() != Some(context.style)
        || row.aspect_ratio.as_deref() != Some(context.aspect_ratio)
    {
        return false;
    }
    let Some(metadata) = row.metadata_json.as_ref() else {
        return false;
    };
    let Some(expected_source) = context
        .source_metadata
        .get("content_sha256")
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
    else {
        return false;
    };
    let matches_source = metadata
        .get("source")
        .and_then(|source| source.get("content_sha256"))
        .and_then(Value::as_str)
        == Some(expected_source);
    if !matches_source {
        return false;
    }
    let Some(expected_contract) =
        expected_contract_version(expected_storage_workflow_mode, context.prompt_type)
    else {
        return false;
    };
    if metadata.get("contract_version").and_then(Value::as_str) != Some(expected_contract) {
        return false;
    }
    if a4n_domain::prompt_language::prompt_type_uses_prompt_output_language(context.prompt_type)
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
    if context.prompt_type == chapter_video_prompt::PROMPT_TYPE_VIDEO
        && context.expected_workflow_mode == chapter_video_prompt::WORKFLOW_MODE_CHAT_VIDEO
        && !reference_ids_match(
            metadata.get("generation_reference_image_ids"),
            context.reference_ids,
        )
    {
        return false;
    }
    if !video_execution_segment_identity_matches(Some(metadata), context.video_execution_identity) {
        return false;
    }

    context.source_prompt_type.is_none_or(|expected| {
        metadata.get("source_prompt_type").and_then(Value::as_str) == Some(expected)
    })
}

fn prompt_type_uses_generation_assets(prompt_type: &str) -> bool {
    matches!(
        prompt_type,
        chapter_video_prompt::PROMPT_TYPE_FRAME
            | chapter_video_prompt::PROMPT_TYPE_FIRST_FRAME
            | chapter_video_prompt::PROMPT_TYPE_CHAT_VIDEO_FRAME
            | chapter_video_prompt::PROMPT_TYPE_STORYBOARD
            | chapter_video_prompt::PROMPT_TYPE_VIDEO
    )
}

pub(in crate::video) fn expected_contract_version(
    expected_workflow_mode: &str,
    prompt_type: &str,
) -> Option<&'static str> {
    match (expected_workflow_mode, prompt_type) {
        (
            chapter_video_prompt::WORKFLOW_MODE_SHARED_SEGMENT_PLAN,
            chapter_video_prompt::PROMPT_TYPE_FRAME_SEGMENT_PLAN,
        )
        | (
            chapter_video_prompt::WORKFLOW_MODE_BOUNDARY_FRAMES,
            chapter_video_prompt::PROMPT_TYPE_VIDEO,
        ) => Some(FRAME_SEGMENT_PLAN_CONTRACT_VERSION),
        (
            chapter_video_prompt::WORKFLOW_MODE_BOUNDARY_FRAMES,
            chapter_video_prompt::PROMPT_TYPE_FRAME,
        )
        | (
            chapter_video_prompt::WORKFLOW_MODE_CHAT_VIDEO,
            chapter_video_prompt::PROMPT_TYPE_CHAT_VIDEO_FRAME,
        ) => Some(BOUNDARY_FRAME_PROMPT_CONTRACT_VERSION),
        (
            chapter_video_prompt::WORKFLOW_MODE_FIRST_FRAME_CHAIN,
            chapter_video_prompt::PROMPT_TYPE_FIRST_FRAME | chapter_video_prompt::PROMPT_TYPE_VIDEO,
        ) => Some(FIRST_FRAME_CHAIN_CONTRACT_VERSION),
        (
            chapter_video_prompt::WORKFLOW_MODE_CHAT_VIDEO,
            chapter_video_prompt::PROMPT_TYPE_VIDEO,
        ) => Some(CHAT_VIDEO_CONTRACT_VERSION),
        (
            chapter_video_prompt::WORKFLOW_MODE_STORYBOARD,
            chapter_video_prompt::PROMPT_TYPE_STORYBOARD,
        ) => Some(STORYBOARD_PROMPT_CONTRACT_VERSION),
        (
            chapter_video_prompt::WORKFLOW_MODE_STORYBOARD,
            chapter_video_prompt::PROMPT_TYPE_VIDEO,
        ) => Some(STORYBOARD_VIDEO_PROMPT_CONTRACT_VERSION),
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
