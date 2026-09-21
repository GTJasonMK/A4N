use a4n_domain::source_text_hash as source_content_sha256;
use a4n_error::AppError;
use a4n_video_contract::{VideoSegmentExecutionRecipeV1, video_segment_content_fingerprint};
use serde_json::{Value, json};

use a4n_llm_context::LlmContextManifest;

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
        "schema_version": 2,
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
    let segment_identities = segment_plan
        .iter()
        .map(|segment| {
            Ok(json!({
                "source_shot_id": segment.source_shot_id,
                "segment_index_in_shot": segment.segment_index_in_shot,
                "segment_sha256": video_segment_content_fingerprint(segment)
                    .map_err(AppError::InvalidArgument)?,
            }))
        })
        .collect::<Result<Vec<_>, AppError>>()?;
    Ok(json!({
        "contract_epoch": a4n_video_contract::VIDEO_EXECUTION_CONTRACT_EPOCH,
        "shooting_script_identity": projection.shooting_script_identity,
        "projection_sha256": source_content_sha256(&projection_json),
        "segment_plan_sha256": source_content_sha256(&segment_plan_json),
        "unit_plan_sha256": source_content_sha256(&unit_plan_json),
        "segment_identities": segment_identities,
        "source_content_sha256": projection.shooting_script_identity.source_content_sha256,
        "style_sha256": source_content_sha256(style),
        "duration_secs": duration_secs,
        "aspect_ratio": aspect_ratio,
    }))
}

pub(in crate::video) fn video_segment_execution_identity(
    mut identity: Value,
    segment_plan_prompt_id: Option<i64>,
    recipe: Option<&VideoSegmentExecutionRecipeV1>,
    target: (&str, i32),
) -> Result<Value, AppError> {
    video_segment_execution_identity_with(
        &mut identity,
        segment_plan_prompt_id,
        recipe,
        target,
        VideoSegmentExecutionRecipeV1::generation_fingerprint,
    )?;
    Ok(identity)
}

pub(in crate::video) fn video_segment_frame_execution_identity(
    mut identity: Value,
    segment_plan_prompt_id: Option<i64>,
    recipe: Option<&VideoSegmentExecutionRecipeV1>,
    target: (&str, i32),
) -> Result<Value, AppError> {
    video_segment_execution_identity_with(
        &mut identity,
        segment_plan_prompt_id,
        recipe,
        target,
        VideoSegmentExecutionRecipeV1::frame_generation_fingerprint,
    )?;
    Ok(identity)
}

fn video_segment_execution_identity_with(
    identity: &mut Value,
    segment_plan_prompt_id: Option<i64>,
    recipe: Option<&VideoSegmentExecutionRecipeV1>,
    target: (&str, i32),
    fingerprint: fn(&VideoSegmentExecutionRecipeV1) -> Result<String, String>,
) -> Result<(), AppError> {
    let plan_id = segment_plan_prompt_id.ok_or_else(|| {
        AppError::InvalidArgument("片段执行缺少 segment_plan_prompt_id".to_string())
    })?;
    let recipe = recipe
        .ok_or_else(|| AppError::InvalidArgument("片段执行缺少 segment_recipe".to_string()))?;
    let segment_identity = identity
        .get("segment_identities")
        .and_then(Value::as_array)
        .and_then(|items| {
            items.iter().find(|item| {
                item.get("source_shot_id").and_then(Value::as_str) == Some(target.0)
                    && item.get("segment_index_in_shot").and_then(Value::as_i64)
                        == Some(i64::from(target.1))
            })
        })
        .cloned()
        .ok_or_else(|| {
            AppError::InvalidArgument(format!(
                "视频执行身份不存在镜头 {} 的第 {} 个片段",
                target.0, target.1
            ))
        })?;
    let object = identity
        .as_object_mut()
        .ok_or_else(|| AppError::InvalidArgument("视频执行身份必须是 JSON 对象".to_string()))?;
    object.insert("segment_plan_prompt_id".to_string(), json!(plan_id));
    object.insert("segment_number".to_string(), json!(recipe.segment_number));
    object.insert("source_shot_id".to_string(), json!(target.0));
    object.insert("segment_index_in_shot".to_string(), json!(target.1));
    object.insert(
        "segment_sha256".to_string(),
        segment_identity
            .get("segment_sha256")
            .cloned()
            .ok_or_else(|| AppError::InvalidArgument("片段执行身份缺少内容指纹".to_string()))?,
    );
    object.insert(
        "segment_recipe_fingerprint".to_string(),
        json!(fingerprint(recipe).map_err(AppError::InvalidArgument)?),
    );
    Ok(())
}
