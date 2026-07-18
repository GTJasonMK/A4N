use a4n_core::{error::AppError, source_spans::source_text_hash as source_content_sha256};
use serde_json::{Value, json};

use a4n_llm::context::LlmContextManifest;

use super::super::ChapterVideoSourceContext;
use super::super::{VideoSegmentPlan, VideoShootingScriptProjection, video_units_for_segment_plan};

pub(in crate::video::prompt_generation::frame_prompts) fn serialize_video_value<
    T: serde::Serialize,
>(
    value: &T,
    context: &str,
) -> Result<Value, AppError> {
    serde_json::to_value(value)
        .map_err(|err| AppError::InvalidArgument(format!("{context} 序列化失败: {err}")))
}

pub(in crate::video::prompt_generation::frame_prompts) fn video_stage_fingerprint(
    prompt_name: &str,
    context_manifest: &LlmContextManifest,
    extra: Value,
) -> Value {
    json!({
        "schema_version": 1,
        "prompt_name": prompt_name,
        "context_manifest": context_manifest,
        "extra": extra,
    })
}

pub(in crate::video::prompt_generation::frame_prompts) fn video_final_segment_plan_fingerprint(
    project_id: i64,
    source: ChapterVideoSourceContext<'_>,
    shooting_script_projection_json: &Value,
    style: &str,
    duration_secs: i32,
    aspect_ratio: &str,
) -> Value {
    json!({
        "schema_version": 1,
        "kind": "video_segment_plan.final",
        "project_id": project_id,
        "source_sha256": source_content_sha256(source.content),
        "source_summary": source.summary,
        "source_analysis": source.analysis,
        "shooting_script_projection": shooting_script_projection_json,
        "style": style,
        "duration_secs": duration_secs,
        "aspect_ratio": aspect_ratio,
    })
}

pub(crate) fn video_execution_chain_identity(
    projection: &VideoShootingScriptProjection,
    segment_plan: &[VideoSegmentPlan],
    style: &str,
    duration_secs: i32,
    aspect_ratio: &str,
) -> Result<Value, AppError> {
    let projection_json = serde_json::to_string(projection)?;
    let segment_plan_json = serde_json::to_string(segment_plan)?;
    let unit_plan = video_units_for_segment_plan(segment_plan, duration_secs)?;
    let unit_plan_json = serde_json::to_string(&unit_plan)?;
    Ok(json!({
        "contract_epoch": a4n_core::video_execution::VIDEO_EXECUTION_CONTRACT_EPOCH,
        "shooting_script_identity": projection.shooting_script_identity,
        "projection_sha256": source_content_sha256(&projection_json),
        "segment_plan_sha256": source_content_sha256(&segment_plan_json),
        "unit_plan_sha256": source_content_sha256(&unit_plan_json),
        "source_content_sha256": projection.shooting_script_identity.source_content_sha256,
        "style_sha256": source_content_sha256(style),
        "duration_secs": duration_secs,
        "aspect_ratio": aspect_ratio,
    }))
}
