use std::collections::BTreeSet;

use a4n_db::workflow;
use serde_json::{Map, Value, json};

use crate::video::prompt_generation::frame_prompts::{
    AppError, FINAL_VIDEO_PROMPT_OUTPUT_CONTRACT_VERSION, ProgressSpan,
    RenderOrRestoreJsonWithValidationInput, VideoPromptChunk, VideoPromptProgress,
    VideoPromptStepProgress, VideoPromptTargetedRepairResponse, build_final_video_context_manifest,
    render_or_restore_json_with_validation_with_context, video_stage_fingerprint,
};
use crate::workflow::workflow_checkpoint_contract_app_error;

use super::VideoPromptUnitCheckpointContext;

const TARGETED_REPAIR_PROMPT: &str = "chapter_video_prompt_targeted_repair";
const TARGETED_REPAIR_SCHEMA_VERSION: &str = "chapter_video_prompt_targeted_repair_v1";
const API_FIELDS: &[&str] = &[
    "title",
    "scene_summary",
    "visual_prompt",
    "motion_prompt",
    "negative_prompt",
    "dialogue_cues",
];
const CHAT_FIELDS: &[&str] = &["title", "scene_summary", "chat_prompt", "negative_prompt"];

pub(super) async fn repair_invalid_video_prompt_fields(
    context: VideoPromptUnitCheckpointContext<'_, '_>,
    invalid_value: &Value,
    validation_error: &AppError,
    progress_span: ProgressSpan,
    progress: &mut Option<VideoPromptProgress<'_>>,
    chunks: &mut Option<VideoPromptChunk<'_>>,
) -> Result<Value, AppError> {
    let allowed_fields = if context.is_chat_video {
        CHAT_FIELDS
    } else {
        API_FIELDS
    };
    let requested_fields = requested_fields(validation_error, allowed_fields);
    if requested_fields.is_empty() {
        return Err(AppError::InvalidArgument(format!(
            "{}结构校验失败，且错误无法安全归类到可局部修复字段；不会执行完整对象重写：{}",
            context.spec.video_prompt_label, validation_error
        )));
    }
    let repair_user = serde_json::to_string_pretty(&json!({
        "schema_version": "chapter_video_prompt_targeted_repair_request_v1",
        "invalid_output": invalid_value,
        "validation_error": validation_error.to_string(),
        "requested_fields": requested_fields,
        "allowed_fields": allowed_fields,
        "rules": [
            "repairs 只能返回 requested_fields，不能返回未请求字段",
            "不得重写 shots 根对象或其他已经合法的字段",
            "修复值必须保持原有业务事实、人物、动作、对白和镜头意图",
            "只输出严格 JSON 对象"
        ],
        "output_contract": {
            "schema_version": TARGETED_REPAIR_SCHEMA_VERSION,
            "repairs": "仅包含 requested_fields 对应的新值"
        }
    }))?;
    let manifest = build_final_video_context_manifest(
        context.prompt_renderer,
        context.project_id,
        context.vars,
        context.source,
        &[
            "chapter_video_segment_plan",
            context.spec.frame_prompt_name,
            context.spec.video_prompt_name,
            TARGETED_REPAIR_PROMPT,
        ],
        repair_user,
    )
    .await?;
    let call_key = format!(
        "{}:shot:{}:targeted_repair",
        context.spec.video_prompt_name, context.shot_number
    );
    let fingerprint = video_stage_fingerprint(
        TARGETED_REPAIR_PROMPT,
        &manifest,
        json!({
            "project_id": context.project_id,
            "chapter_id": context.chapter_id,
            "shot_number": context.shot_number,
            "requested_fields": requested_fields,
            "output_contract_version": FINAL_VIDEO_PROMPT_OUTPUT_CONTRACT_VERSION,
        }),
    );
    let workflow_run_id =
        workflow::required_llm_call_workflow_run_id(context.workflow_run_id, &call_key)
            .map_err(workflow_checkpoint_contract_app_error)?;
    let (_, repairs) = render_or_restore_json_with_validation_with_context(
        RenderOrRestoreJsonWithValidationInput {
            service: context.service,
            workflow_run_id,
            call_key,
            call_order: 800 + context.shot_number,
            input_fingerprint_json: fingerprint,
            prompt_renderer: context.prompt_renderer,
            prompt_name: TARGETED_REPAIR_PROMPT,
            project_id: context.project_id,
            variables: context.vars,
            context_manifest: manifest,
            temperature: 0.0,
            cancel_token: context.cancel_token,
            step: VideoPromptStepProgress::new(
                progress_span.second_half().start(),
                100,
                "正在局部修复视频提示词失败字段",
            )
            .with_span(progress_span.second_half()),
            progress,
            chunks,
            structured_progress: None,
            checkpoint_context: "视频提示词字段修复断点",
        },
        |raw| parse_targeted_repairs(raw, &requested_fields, allowed_fields),
        |repairs| serde_json::to_value(repairs).map_err(AppError::Serialization),
    )
    .await?;
    apply_targeted_repairs(invalid_value, repairs)
}

fn requested_fields(error: &AppError, allowed_fields: &[&str]) -> Vec<String> {
    let message = error.to_string();
    allowed_fields
        .iter()
        .filter(|field| message.contains(**field))
        .map(|field| (*field).to_string())
        .collect()
}

fn parse_targeted_repairs(
    raw: &str,
    requested_fields: &[String],
    allowed_fields: &[&str],
) -> Result<VideoPromptTargetedRepairResponse, AppError> {
    let response: VideoPromptTargetedRepairResponse =
        crate::structured_llm::parse_strict_json_response(raw, "视频提示词字段修复")?;
    if response.schema_version != TARGETED_REPAIR_SCHEMA_VERSION {
        return Err(AppError::InvalidArgument(format!(
            "视频提示词字段修复 schema_version 必须是 {TARGETED_REPAIR_SCHEMA_VERSION}"
        )));
    }
    let value = serde_json::to_value(&response.repairs)?;
    let returned = value
        .as_object()
        .expect("targeted repair fields serialize as object")
        .keys()
        .cloned()
        .collect::<BTreeSet<_>>();
    let requested = requested_fields.iter().cloned().collect::<BTreeSet<_>>();
    if returned != requested {
        return Err(AppError::InvalidArgument(format!(
            "视频提示词字段修复必须且只能返回 {:?}，实际返回 {:?}",
            requested, returned
        )));
    }
    if returned
        .iter()
        .any(|field| !allowed_fields.contains(&field.as_str()))
    {
        return Err(AppError::InvalidArgument(
            "视频提示词字段修复返回了当前工作流不允许的字段".to_string(),
        ));
    }
    Ok(response)
}

fn apply_targeted_repairs(
    invalid_value: &Value,
    response: VideoPromptTargetedRepairResponse,
) -> Result<Value, AppError> {
    let repairs = serde_json::to_value(response.repairs)?;
    let repairs = repairs
        .as_object()
        .ok_or_else(|| AppError::InvalidArgument("视频提示词 repairs 必须是对象".to_string()))?;
    let mut patched = invalid_value.clone();
    let shot = first_shot_object_mut(&mut patched)?;
    for (field, value) in repairs {
        shot.insert(field.clone(), value.clone());
    }
    Ok(patched)
}

fn first_shot_object_mut(value: &mut Value) -> Result<&mut Map<String, Value>, AppError> {
    value
        .get_mut("shots")
        .and_then(Value::as_array_mut)
        .and_then(|shots| shots.first_mut())
        .and_then(Value::as_object_mut)
        .ok_or_else(|| {
            AppError::InvalidArgument(
                "视频提示词字段修复要求原响应包含 shots[0] JSON 对象".to_string(),
            )
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn requested_fields_only_selects_named_allowed_fields() {
        let error = AppError::InvalidArgument(
            "shots[0].visual_prompt 缺少镜头分镜；dialogue_cues 必须是数组".to_string(),
        );
        assert_eq!(
            requested_fields(&error, API_FIELDS),
            vec!["visual_prompt".to_string(), "dialogue_cues".to_string()]
        );
    }

    #[test]
    fn targeted_repair_rejects_unrequested_fields() {
        let error = parse_targeted_repairs(
            r#"{
                "schema_version":"chapter_video_prompt_targeted_repair_v1",
                "repairs":{"visual_prompt":"修复后的提示词","negative_prompt":"多余字段"}
            }"#,
            &["visual_prompt".to_string()],
            API_FIELDS,
        )
        .expect_err("repair must not return fields that were not requested");
        assert!(error.to_string().contains("必须且只能返回"));
    }

    #[test]
    fn targeted_repair_only_patches_requested_shot_fields() {
        let original = json!({
            "shots": [{
                "title": "保留标题",
                "scene_summary": "保留摘要",
                "visual_prompt": "无效提示词",
                "motion_prompt": "保留运动提示词",
                "negative_prompt": "",
                "dialogue_cues": []
            }]
        });
        let response = parse_targeted_repairs(
            r#"{
                "schema_version":"chapter_video_prompt_targeted_repair_v1",
                "repairs":{"visual_prompt":"修复后的提示词"}
            }"#,
            &["visual_prompt".to_string()],
            API_FIELDS,
        )
        .expect("requested field repair should parse");
        let patched = apply_targeted_repairs(&original, response).expect("repair should apply");
        assert_eq!(patched["shots"][0]["visual_prompt"], "修复后的提示词");
        assert_eq!(patched["shots"][0]["title"], "保留标题");
        assert_eq!(patched["shots"][0]["motion_prompt"], "保留运动提示词");
    }
}
