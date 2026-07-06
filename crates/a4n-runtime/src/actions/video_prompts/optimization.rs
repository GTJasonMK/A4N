use super::{
    ChapterVideoPromptSummary, apply_video_prompt_downstream_deletions,
    prepare_video_prompt_mutation_plan, prompt_to_summary, validate_project_chapter_video_source,
};
use crate::AppState;
use crate::actions::access;
use crate::actions::dependency_conflict::DownstreamConflictStrategy;
use crate::actions::video_prompt_text::{
    MIN_FRAME_PROMPT_CHARS, MIN_PROVIDER_MOTION_PROMPT_CHARS, MIN_PROVIDER_VISUAL_PROMPT_CHARS,
    validate_provider_model_text,
};
use a4n_core::models::chapter::chapter_video_prompt;
use a4n_engine::llm::provider::{ChatRequest, LlmTraceContext, is_truncated_finish_reason};
use a4n_engine::prompt::engine::PromptEngine;
use a4n_services::generation::{
    GenerationRunCheckpointContext, RecordCompletedGenerationRunInput,
    RecordCompletedGenerationRunWithCheckpointInput, context_aware_chat_request_trace_json,
    generation_checkpoint_fingerprint_sha256,
};
use a4n_services::llm_checkpoint::{
    CompletedLlmCallCheckpointInput, llm_call_checkpoint_fingerprint_sha256,
    load_completed_llm_call_checkpoint, required_llm_call_checkpoint_json,
    save_completed_llm_call_checkpoint,
};
use a4n_services::llm_context::{
    COMPRESSION_FOCUSED_RAW, COMPRESSION_FULL_RAW, COMPRESSION_MINIMAL, LlmContextLedger,
    LlmContextLedgerPackDraft, context_aware_system_user_request,
};
use a4n_services::prompt_language::{
    prompt_output_language_from_instruction, with_prompt_output_language_metadata,
};
use a4n_services::source::source_content_sha256;
use a4n_services::video::{
    PromptOutputLanguage, VideoPromptUpdateInput, validate_direct_prompt_text,
};
use a4n_services::workflow::{
    UpsertWorkflowRunInput, WORKFLOW_STATUS_COMPLETED, WORKFLOW_STATUS_FAILED,
    WORKFLOW_STATUS_RUNNING,
};
use sea_orm::EntityTrait;
use serde::Deserialize;
use serde_json::{Value, json};
use std::collections::HashMap;

const CHAPTER_VIDEO_PROMPT_OPTIMIZATION_PROMPT_NAME: &str = "chapter_video_prompt_optimization";
const CHAPTER_VIDEO_PROMPT_OPTIMIZATION_USER_PROMPT_NAME: &str =
    "chapter_video_prompt_optimization_user";

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OptimizeChapterVideoPromptInput {
    pub instruction: String,
    #[serde(default)]
    pub downstream_strategy: DownstreamConflictStrategy,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct OptimizedVideoPromptFields {
    pub visual_prompt: String,
    pub motion_prompt: Option<String>,
    pub negative_prompt: Option<String>,
}

pub async fn optimize_chapter_video_prompt(
    state: &AppState,
    user_id: i64,
    prompt_id: i64,
    input: OptimizeChapterVideoPromptInput,
) -> Result<ChapterVideoPromptSummary, String> {
    let instruction = input.instruction.trim();
    if instruction.is_empty() {
        return Err("优化要求不能为空".to_string());
    }
    let prompt = chapter_video_prompt::Entity::find_by_id(prompt_id)
        .one(state.db.as_ref())
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "视频提示词不存在".to_string())?;
    access::verify_project_editor_string(state, user_id, prompt.project_id).await?;
    validate_project_chapter_video_source(
        state,
        prompt.project_id,
        prompt.chapter_id,
        "优化视频提示词",
        true,
    )
    .await?;
    let mutation_plan = prepare_video_prompt_mutation_plan(
        state,
        prompt_id,
        input.downstream_strategy,
        "优化视频提示词",
    )
    .await?;

    let current_language = PromptOutputLanguage::from_metadata(prompt.metadata_json.as_ref());
    let prompt_output_language =
        prompt_output_language_from_instruction(instruction, current_language);
    let mut request = build_video_prompt_optimization_request(
        state.engine.as_ref(),
        &prompt,
        instruction,
        prompt_output_language,
    )?;
    let llm_request = request.clone();
    let request_json = context_aware_chat_request_trace_json(&llm_request)
        .map_err(|err| format!("LLM 请求序列化失败: {err}"))?;
    let messages_json = serde_json::to_value(&llm_request.messages)
        .map_err(|err| format!("LLM 消息序列化失败: {err}"))?;
    let input_variables_json = json!({
        "instruction": instruction,
        "prompt_id": prompt.id,
        "prompt_type": &prompt.prompt_type,
        "workflow_mode": &prompt.workflow_mode,
        "shot_number": prompt.shot_number,
        "prompt_output_language": prompt_output_language.as_str(),
    });
    let source_refs_json = json!({
        "previous_prompt_id": prompt.id,
        "previous_visual_prompt": &prompt.visual_prompt,
        "previous_motion_prompt": &prompt.motion_prompt,
        "previous_negative_prompt": &prompt.negative_prompt,
    });
    let media_refs_json = prompt.reference_image_ids_json.clone();
    let input_fingerprint_json = json!({
        "prompt_names": [
            CHAPTER_VIDEO_PROMPT_OPTIMIZATION_PROMPT_NAME,
            CHAPTER_VIDEO_PROMPT_OPTIMIZATION_USER_PROMPT_NAME,
        ],
        "request": request_json.clone(),
        "input_variables": input_variables_json.clone(),
        "source_refs": source_refs_json.clone(),
        "media_refs": media_refs_json.clone(),
    });
    let workflow_run = state
        .workflow_run
        .upsert_run(UpsertWorkflowRunInput {
            business_key: format!(
                "video_prompt_optimization:{}:{prompt_id}",
                prompt.project_id
            ),
            workflow_kind: "video_prompt_optimization".to_string(),
            project_id: Some(prompt.project_id),
            root_project_id: None,
            source_snapshot_id: None,
            scope_table: Some("chapter_video_prompt".to_string()),
            scope_id: Some(prompt_id),
            chapter_id: Some(prompt.chapter_id),
            mode: Some(prompt.workflow_mode.clone()),
            input_fingerprint_json: Some(input_fingerprint_json.clone()),
            status: WORKFLOW_STATUS_RUNNING.to_string(),
            current_step_key: Some(format!("optimize_chapter_video_prompt:prompt:{prompt_id}")),
            active_task_id: None,
            can_resume: true,
            resume_block_reason: None,
            error_message: None,
        })
        .await
        .map_err(|err| err.to_string())?;
    let workflow_run_id = workflow_run.id;
    let generation_input_fingerprint_sha256 =
        generation_checkpoint_fingerprint_sha256(&input_fingerprint_json)
            .map_err(|err| err.to_string())?;
    let call_key = format!("optimize_chapter_video_prompt:prompt:{prompt_id}");
    let input_fingerprint_sha256 = llm_call_checkpoint_fingerprint_sha256(&input_fingerprint_json)
        .map_err(|err| err.to_string())?;
    if let Some(trace_context) = request.trace_context.as_mut() {
        trace_context.workflow_run_id = Some(workflow_run_id);
        trace_context.call_key = Some(call_key.clone());
        trace_context.input_fingerprint_sha256 = Some(input_fingerprint_sha256);
    }
    let request_audit_json = context_aware_chat_request_trace_json(&request)
        .map_err(|err| format!("LLM 请求序列化失败: {err}"))?;
    state
        .workflow_run
        .mark_current_step(workflow_run_id, &call_key)
        .await
        .map_err(|err| err.to_string())?;

    let result = async {
        let (response_text, response_usage_json, response_finish_reason, restored) =
            if let Some(checkpoint) = load_completed_llm_call_checkpoint(
                state.db.clone(),
                workflow_run_id,
                &call_key,
                &input_fingerprint_json,
            )
            .await
            .map_err(|err| err.to_string())?
            {
                let checkpoint_json = required_llm_call_checkpoint_json(checkpoint, &call_key)
                    .map_err(|err| err.to_string())?;
                let (response_text, response_usage_json, response_finish_reason) =
                    video_prompt_optimization_checkpoint_response(&checkpoint_json, &call_key)?;
                (
                    response_text,
                    response_usage_json,
                    response_finish_reason,
                    true,
                )
            } else {
                let response = state
                    .llm
                    .chat(request)
                    .await
                    .map_err(|e| format!("优化视频提示词失败: {e}"))?;
                if is_truncated_finish_reason(response.finish_reason.as_deref()) {
                    return Err("优化视频提示词输出达到当前 max_tokens 上限，返回 JSON 被截断；请在设置页调大“媒体提示词”输出档位，或缩短本次提示词后重试".to_string());
                }
                let response_text = response.message.content.clone();
                let response_usage_json = response
                    .usage
                    .as_ref()
                    .map(serde_json::to_value)
                    .transpose()
                    .map_err(|err| format!("LLM 用量序列化失败: {err}"))?;
                let response_finish_reason = response.finish_reason.clone();
                (
                    response_text,
                    response_usage_json,
                    response_finish_reason,
                    false,
                )
            };
        let optimized = parse_video_prompt_optimization_output(&response_text)?;
        let parsed_visual_prompt = optimized.visual_prompt.clone();
        let parsed_motion_prompt = optimized.motion_prompt.clone();
        let parsed_negative_prompt = optimized.negative_prompt.clone();
        let visual_prompt = required_optimized_text(&optimized.visual_prompt, "visual_prompt")?;
        let motion_prompt = optimized_motion_prompt(&prompt, optimized.motion_prompt)?;
        let negative_prompt = normalized_optimized_optional_text(optimized.negative_prompt)
            .or_else(|| normalized_existing_optional_text(prompt.negative_prompt.as_deref()));
        validate_optimized_prompt_fields(
            &prompt,
            &visual_prompt,
            motion_prompt.as_deref(),
            negative_prompt.as_deref(),
            prompt_output_language,
        )?;
        if !restored {
            save_completed_llm_call_checkpoint(
                state.db.clone(),
                CompletedLlmCallCheckpointInput {
                    workflow_run_id,
                    call_key: call_key.clone(),
                    call_order: 1,
                    input_fingerprint_json: input_fingerprint_json.clone(),
                    checkpoint_json: json!({
                        "output": {
                            "visual_prompt": visual_prompt.clone(),
                            "motion_prompt": motion_prompt.clone(),
                            "negative_prompt": negative_prompt.clone(),
                        },
                        "parsed_output": {
                            "visual_prompt": parsed_visual_prompt,
                            "motion_prompt": parsed_motion_prompt,
                            "negative_prompt": parsed_negative_prompt,
                        },
                        "response_content": response_text.clone(),
                        "usage": response_usage_json.clone(),
                        "finish_reason": response_finish_reason.clone(),
                    }),
                    artifacts_json: None,
                },
            )
            .await
            .map_err(|err| err.to_string())?;
        }
        let optimized_metadata = with_prompt_output_language_metadata(
            prompt.metadata_json.clone().unwrap_or_else(|| json!({})),
            prompt_output_language,
        );

        apply_video_prompt_downstream_deletions(state, user_id, &mutation_plan).await?;
        state
            .video
            .update_prompt_with_generation_run_checkpoint(
                prompt_id,
                VideoPromptUpdateInput {
                    visual_prompt: Some(&visual_prompt),
                    motion_prompt: Some(motion_prompt.as_deref()),
                    negative_prompt: Some(negative_prompt.as_deref()),
                    duration_secs: None,
                    reference_image_ids_json: None,
                },
                Some(optimized_metadata),
                mutation_plan.allow_downstream,
                RecordCompletedGenerationRunWithCheckpointInput {
                run: RecordCompletedGenerationRunInput {
                    project_id: prompt.project_id,
                    workflow: "video_prompt_generation".to_string(),
                    step_key: "optimize_chapter_video_prompt".to_string(),
                    title: "视频提示词优化".to_string(),
                    model_kind: "llm".to_string(),
                    provider: None,
                    model: None,
                    prompt_template_names_json: Some(json!([
                        CHAPTER_VIDEO_PROMPT_OPTIMIZATION_PROMPT_NAME,
                        CHAPTER_VIDEO_PROMPT_OPTIMIZATION_USER_PROMPT_NAME,
                    ])),
                    prompt_template_hashes_json: None,
                    rendered_messages_json: Some(messages_json),
                    rendered_prompts_json: None,
                    input_variables_json: Some(input_variables_json),
                    source_refs_json: Some(source_refs_json),
                    media_refs_json,
                    request_json: Some(request_audit_json),
                    response_summary_json: Some(json!({
                        "visual_prompt_chars": visual_prompt.chars().count(),
                        "motion_prompt_chars": motion_prompt.as_deref().map(|value| value.chars().count()),
                        "negative_prompt_chars": negative_prompt.as_deref().map(|value| value.chars().count()),
                    })),
                    targets: Vec::new(),
                },
                checkpoint: GenerationRunCheckpointContext {
                    workflow_run_id: Some(workflow_run_id),
                    call_key: Some(call_key.clone()),
                    input_fingerprint_sha256: Some(generation_input_fingerprint_sha256),
                    input_fingerprint_json: Some(input_fingerprint_json),
                    raw_response_json: Some(json!({
                        "text": response_text,
                        "usage": response_usage_json,
                        "finish_reason": response_finish_reason,
                    })),
                    parsed_output_json: Some(json!({
                        "optimized_fields": {
                            "visual_prompt": visual_prompt.clone(),
                            "motion_prompt": motion_prompt.clone(),
                            "negative_prompt": negative_prompt.clone(),
                        },
                    })),
                    validation_json: None,
                    artifact_refs_json: None,
                    attempt: None,
                },
            },
        )
        .await
        .map_err(|e| e.to_string())
    }
    .await;

    if let Err(status_err) =
        finalize_video_prompt_optimization_workflow(state, workflow_run_id, &result).await
    {
        return match result {
            Ok(_) => Err(format!(
                "视频提示词优化完成，但工作流状态写入失败: {status_err}"
            )),
            Err(err) => Err(format!("{err}；工作流失败状态写入失败: {status_err}")),
        };
    }
    result.map(|row| prompt_to_summary(&row))
}

async fn finalize_video_prompt_optimization_workflow(
    state: &AppState,
    workflow_run_id: i64,
    result: &Result<chapter_video_prompt::Model, String>,
) -> Result<(), String> {
    let (status, can_resume, error_message) = match result {
        Ok(_) => (WORKFLOW_STATUS_COMPLETED, false, None),
        Err(message) => (WORKFLOW_STATUS_FAILED, true, Some(message.as_str())),
    };
    state
        .workflow_run
        .mark_status(workflow_run_id, status, can_resume, error_message)
        .await
        .map_err(|err| err.to_string())?;
    Ok(())
}

pub(super) fn video_prompt_optimization_checkpoint_response(
    checkpoint_json: &Value,
    call_key: &str,
) -> Result<(String, Option<Value>, Option<String>), String> {
    let response_text = checkpoint_json
        .get("response_content")
        .and_then(Value::as_str)
        .map(ToString::to_string)
        .ok_or_else(|| format!("{call_key}.response_content 必须是字符串"))?;
    let parsed = parse_video_prompt_optimization_output(&response_text)?;
    let parsed_output = checkpoint_json
        .get("parsed_output")
        .ok_or_else(|| format!("{call_key}.parsed_output 缺失"))?;
    let checkpoint_parsed: OptimizedVideoPromptFields =
        serde_json::from_value(parsed_output.clone())
            .map_err(|err| format!("{call_key}.parsed_output 字段不符合要求: {err}"))?;
    if checkpoint_parsed.visual_prompt != parsed.visual_prompt
        || checkpoint_parsed.motion_prompt != parsed.motion_prompt
        || checkpoint_parsed.negative_prompt != parsed.negative_prompt
    {
        return Err(format!(
            "{call_key} checkpoint 的 parsed_output 与 response_content 重新解析结果不一致"
        ));
    }
    let usage = checkpoint_json
        .get("usage")
        .filter(|value| !value.is_null())
        .cloned();
    let finish_reason = match checkpoint_json.get("finish_reason") {
        Some(Value::Null) | None => None,
        Some(value) => Some(
            value
                .as_str()
                .map(ToString::to_string)
                .ok_or_else(|| format!("{call_key}.finish_reason 必须是字符串或 null"))?,
        ),
    };
    if is_truncated_finish_reason(finish_reason.as_deref()) {
        return Err(format!(
            "{call_key} checkpoint 标记为截断响应，不能用于恢复"
        ));
    }
    Ok((response_text, usage, finish_reason))
}

pub(super) fn build_video_prompt_optimization_request(
    prompt_engine: &dyn PromptEngine,
    prompt: &chapter_video_prompt::Model,
    instruction: &str,
    prompt_output_language: PromptOutputLanguage,
) -> Result<ChatRequest, String> {
    let dynamic_payload = build_video_prompt_optimization_dynamic_payload(
        prompt,
        instruction,
        prompt_output_language,
    );
    let payload = serde_json::to_string_pretty(&dynamic_payload)
        .map_err(|err| format!("视频提示词优化请求 JSON 序列化失败：{err}"))?;
    let variables = HashMap::from([("payload".to_string(), json!(payload))]);
    let system_prompt = render_video_prompt_template(
        prompt_engine,
        CHAPTER_VIDEO_PROMPT_OPTIMIZATION_PROMPT_NAME,
        prompt.project_id,
        &variables,
    )?;
    let user_prompt = render_video_prompt_template(
        prompt_engine,
        CHAPTER_VIDEO_PROMPT_OPTIMIZATION_USER_PROMPT_NAME,
        prompt.project_id,
        &variables,
    )?;
    let current_prompt = video_prompt_optimization_source_artifact(prompt);
    let source_hash = source_content_sha256(
        &serde_json::to_string(&current_prompt)
            .map_err(|err| format!("视频提示词优化 source artifact 序列化失败：{err}"))?,
    );
    let mut ledger = LlmContextLedger::new();
    ledger
        .add_pack(a4n_services::llm_context::LlmContextLedgerPackInput {
            scope_kind: ("prompt_optimization").into(),
            pack_kind: ("source_artifact").into(),
            source_refs_json: Some(json!({
                "source": "chapter_video_prompt",
                "prompt_id": prompt.id,
                "content_sha256": source_hash,
            })),
            config_identity_json: json!({
                "ledger_version": "video_prompt_optimization_context_v1",
                "artifact_kind": "chapter_video_prompt",
            }),
            required: true,
            stable_prefix: true,
            variants: vec![
                LlmContextLedgerPackDraft::new(COMPRESSION_FULL_RAW, current_prompt.clone()),
                LlmContextLedgerPackDraft::new(COMPRESSION_FOCUSED_RAW, current_prompt.clone()),
                LlmContextLedgerPackDraft::new(
                    COMPRESSION_MINIMAL,
                    json!({
                        "artifact_kind": "chapter_video_prompt",
                        "prompt_id": prompt.id,
                        "prompt_type": prompt.prompt_type.as_str(),
                        "workflow_mode": prompt.workflow_mode.as_str(),
                        "shot_number": prompt.shot_number,
                        "title": prompt.title.as_deref(),
                        "visual_prompt": &prompt.visual_prompt,
                        "motion_prompt": prompt.motion_prompt.as_deref(),
                        "negative_prompt": prompt.negative_prompt.as_deref(),
                    }),
                ),
            ],
        })
        .map_err(|err| err.to_string())?;
    let manifest = ledger.into_manifest(
        user_prompt,
        Some("视频提示词优化必须保留稳定上下文包 source_artifact 中的事实、结构和素材约束，只按动态输入中的 instruction 修改表达。".to_string()),
    );
    let fingerprint = source_content_sha256(
        &serde_json::to_string(&json!({
            "prompt_id": prompt.id,
            "source_artifact_sha256": source_hash,
            "instruction": instruction,
            "prompt_output_language": prompt_output_language.as_str(),
            "prompt_names": [
                CHAPTER_VIDEO_PROMPT_OPTIMIZATION_PROMPT_NAME,
                CHAPTER_VIDEO_PROMPT_OPTIMIZATION_USER_PROMPT_NAME,
            ],
        }))
        .map_err(|err| format!("视频提示词优化 fingerprint 序列化失败：{err}"))?,
    );
    let request = context_aware_system_user_request(
        system_prompt,
        manifest,
        Some(0.25),
        None,
        Some("media_prompt".to_string()),
    )
    .map_err(|err| err.to_string())?
    .with_json_response_format()
    .with_trace_context(LlmTraceContext {
        project_id: Some(prompt.project_id),
        root_project_id: None,
        workflow: "video_prompt_optimization".to_string(),
        step_key: "optimize_chapter_video_prompt".to_string(),
        generation_run_id: None,
        workflow_run_id: None,
        call_key: Some(format!(
            "optimize_chapter_video_prompt:prompt:{}",
            prompt.id
        )),
        input_fingerprint_sha256: Some(fingerprint),
        context_plan_json: None,
        context_pack_refs_json: None,
    });
    Ok(request)
}

fn video_prompt_optimization_source_artifact(prompt: &chapter_video_prompt::Model) -> Value {
    json!({
        "artifact_kind": "chapter_video_prompt",
        "current_prompt": {
            "prompt_id": prompt.id,
            "prompt_type": prompt.prompt_type.as_str(),
            "workflow_mode": prompt.workflow_mode.as_str(),
            "shot_number": prompt.shot_number,
            "title": prompt.title.as_deref(),
            "scene_summary": prompt.scene_summary.as_deref(),
            "duration_secs": prompt.duration_secs,
            "aspect_ratio": prompt.aspect_ratio.as_deref(),
            "visual_prompt": &prompt.visual_prompt,
            "motion_prompt": prompt.motion_prompt.as_deref(),
            "negative_prompt": prompt.negative_prompt.as_deref()
        }
    })
}

fn build_video_prompt_optimization_dynamic_payload(
    prompt: &chapter_video_prompt::Model,
    instruction: &str,
    prompt_output_language: PromptOutputLanguage,
) -> Value {
    json!({
        "context_refs": {
            "current_prompt": "稳定上下文包 source_artifact.current_prompt",
        },
        "instruction": instruction,
        "prompt_output_language": prompt_output_language.to_prompt_json(),
        "prompt_output_language_instruction": prompt_output_language.instruction(),
        "output_contract": {
            "format": "json_object",
            "required_fields": ["visual_prompt", "motion_prompt", "negative_prompt"],
            "asset_contract": optimized_prompt_asset_contract(prompt, prompt_output_language),
            "field_rules": {
                "visual_prompt": "优化后的画面/主体提示词，不能为空；自然语言必须遵守 prompt_output_language_instruction；原文对白、引号内台词和角色实际说出口的短句必须保留原文写法，不要把中文对白翻成英文",
                "motion_prompt": "最终视频提示词必须返回完整 motion_prompt，不要写 null；帧提示词不使用 motion_prompt，可返回 null；自然语言必须遵守 prompt_output_language_instruction；原文对白、引号内台词和角色实际说出口的短句必须保留原文写法，不要把中文对白翻成英文",
                "negative_prompt": "优化后的负面提示词；如果不需要修改，返回 null 表示保留原值；自然语言必须遵守 prompt_output_language_instruction"
            },
            "forbidden": ["markdown", "code_fence", "extra_fields", "json_inside_prompt_text", "internal_image_id", "upload_status_text", "motion_timeline_duplicate"]
        }
    })
}

fn optimized_prompt_asset_contract(
    prompt: &chapter_video_prompt::Model,
    prompt_output_language: PromptOutputLanguage,
) -> Value {
    match prompt.prompt_type.as_str() {
        "video_prompt" => json!({
            "kind": "video_prompt",
            "visual_prompt_required_sections": prompt_output_language.visual_sections(),
            "motion_prompt_required_sections": prompt_output_language.motion_sections(),
            "motion_prompt_required": true,
            "motion_prompt_must_not_repeat_timing_breakdown": true,
            "minimum_lengths": {
                "visual_prompt_chars": MIN_PROVIDER_VISUAL_PROMPT_CHARS,
                "motion_prompt_chars": MIN_PROVIDER_MOTION_PROMPT_CHARS
            },
            "forbidden_terms": {
                "upload_status_examples": ["无输入首帧", "没有输入尾帧"],
                "internal_workflow_examples": ["首帧串联", "provider", "image 字段", "generated_image #"]
            }
        }),
        "frame_prompt" | "first_frame_prompt" | "chat_video_frame_prompt" => json!({
            "kind": "frame_prompt",
            "visual_prompt_is_frame_prompt": true,
            "visual_prompt_required_sections": prompt_output_language.frame_sections(),
            "motion_prompt_required": false,
            "minimum_lengths": {
                "visual_prompt_chars": MIN_FRAME_PROMPT_CHARS
            },
            "forbidden_terms": {
                "internal_workflow_examples": ["首帧串联", "provider", "image 字段", "generated_image #"]
            }
        }),
        _ => json!({
            "kind": "text_prompt",
            "motion_prompt_required": false
        }),
    }
}

fn render_video_prompt_template(
    prompt_engine: &dyn PromptEngine,
    prompt_name: &str,
    project_id: i64,
    variables: &HashMap<String, Value>,
) -> Result<String, String> {
    prompt_engine
        .render(prompt_name, variables, Some(project_id))
        .map(|value| value.trim().to_string())
        .map_err(|err| format!("视频提示词模板 {prompt_name} 渲染失败：{err}"))
}

pub(super) fn parse_video_prompt_optimization_output(
    raw: &str,
) -> Result<OptimizedVideoPromptFields, String> {
    let value = crate::actions::json_contract::parse_llm_json_object(raw, "优化结果")?;
    serde_json::from_value(value).map_err(|e| format!("优化结果字段不符合要求: {e}"))
}

fn required_optimized_text(value: &str, field: &str) -> Result<String, String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(format!("优化结果 {field} 不能为空"));
    }
    Ok(trimmed.to_string())
}

fn normalized_optimized_optional_text(value: Option<String>) -> Option<String> {
    value.and_then(|text| {
        let trimmed = text.trim();
        (!trimmed.is_empty()).then(|| trimmed.to_string())
    })
}

fn normalized_existing_optional_text(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|text| !text.is_empty())
        .map(ToString::to_string)
}

pub(super) fn optimized_motion_prompt(
    prompt: &chapter_video_prompt::Model,
    optimized: Option<String>,
) -> Result<Option<String>, String> {
    let optimized = normalized_optimized_optional_text(optimized);
    if prompt.prompt_type == "video_prompt" {
        return Ok(optimized
            .or_else(|| normalized_existing_optional_text(prompt.motion_prompt.as_deref())));
    }
    Ok(normalized_existing_optional_text(
        prompt.motion_prompt.as_deref(),
    ))
}

pub(super) fn validate_optimized_prompt_fields(
    prompt: &chapter_video_prompt::Model,
    visual_prompt: &str,
    motion_prompt: Option<&str>,
    negative_prompt: Option<&str>,
    prompt_output_language: PromptOutputLanguage,
) -> Result<(), String> {
    match prompt.prompt_type.as_str() {
        "video_prompt" => {
            validate_provider_model_text(
                visual_prompt,
                "visual_prompt",
                prompt.id,
                prompt_output_language,
                MIN_PROVIDER_VISUAL_PROMPT_CHARS,
                false,
            )?;
            let motion_prompt = motion_prompt.ok_or_else(|| {
                format!(
                    "优化结果 motion_prompt 不能为空；视频提示词 #{} 生成视频需要运镜与动作提示词",
                    prompt.id
                )
            })?;
            validate_provider_model_text(
                motion_prompt,
                "motion_prompt",
                prompt.id,
                prompt_output_language,
                MIN_PROVIDER_MOTION_PROMPT_CHARS,
                false,
            )?;
        }
        "frame_prompt" | "first_frame_prompt" | "chat_video_frame_prompt" => {
            validate_provider_model_text(
                visual_prompt,
                "frame_prompt",
                prompt.id,
                prompt_output_language,
                MIN_FRAME_PROMPT_CHARS,
                false,
            )?;
        }
        _ => {
            validate_direct_prompt_text(visual_prompt, "视频提示词 visual_prompt")
                .map_err(|err| err.to_string())?;
        }
    }
    if let Some(negative_prompt) = negative_prompt {
        validate_direct_prompt_text(negative_prompt, "视频提示词 negative_prompt")
            .map_err(|err| err.to_string())?;
    }
    Ok(())
}
