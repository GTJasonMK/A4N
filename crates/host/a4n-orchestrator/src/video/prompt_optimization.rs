use std::collections::HashMap;

use a4n_api_contract::video::VideoPromptDownstreamCleanupPort;

use a4n_db::generation::{
    GenerationRunCheckpointContext, RecordCompletedGenerationRunInput,
    RecordCompletedGenerationRunWithCheckpointInput, generation_checkpoint_fingerprint_sha256,
};
use a4n_db_models::models::chapter::chapter_video_prompt;
use a4n_domain::video_prompt::ChapterVideoPromptPatch;
use a4n_domain::{
    prompt_language::{
        PromptOutputLanguage, prompt_output_language_from_instruction,
        with_prompt_output_language_metadata,
    },
    source_text_hash as source_content_sha256,
};
use a4n_error::AppError;
use a4n_llm_context::{
    COMPRESSION_FOCUSED_RAW, COMPRESSION_FULL_RAW, COMPRESSION_MINIMAL, LlmContextLedger,
    LlmContextLedgerPackDraft, context_aware_system_user_request,
};
use a4n_llm_provider_contract::provider::{
    ChatRequest, LlmTraceContext, is_truncated_finish_reason,
};
use a4n_llm_trace::context_aware_chat_request_trace_json;
use a4n_prompt::rendering::PromptRenderer;
use serde_json::{Value, json};

use crate::{
    OrchestratorContext,
    structured_llm::parse_strict_json_response,
    workflow::{self, CompletedLlmCallCheckpoint, CompletedLlmCallCheckpointInput},
};

use super::VideoPromptDownstreamDependencies;
use super::prompt_json::{validate_chat_video_prompt_text, validate_direct_prompt_text};
use super::prompt_optimization_checkpoint::{
    OptimizedVideoPromptFields, video_prompt_optimization_checkpoint_output,
};
use super::prompt_optimization_rules::{
    VideoPromptOptimizationKind, video_prompt_optimization_kind,
};
use super::provider_prompt_text_rules::{
    MIN_FRAME_PROMPT_CHARS, MIN_PROVIDER_MOTION_PROMPT_CHARS, validate_provider_model_text,
};
use super::reference_media_rules::MIN_PROVIDER_VISUAL_PROMPT_CHARS;

const OPTIMIZATION_PROMPT_NAME: &str = "chapter_video_prompt_optimization";
const OPTIMIZATION_USER_PROMPT_NAME: &str = "chapter_video_prompt_optimization_user";

pub(super) struct VideoPromptOptimizationInput {
    pub(super) prompt: chapter_video_prompt::Model,
    pub(super) instruction: String,
    pub(super) allow_downstream: bool,
    pub(super) delete_dependencies: Option<VideoPromptDownstreamDependencies>,
}

struct PreparedVideoPromptOptimization {
    prompt: chapter_video_prompt::Model,
    prompt_kind: VideoPromptOptimizationKind,
    allow_downstream: bool,
    delete_dependencies: Option<VideoPromptDownstreamDependencies>,
    prompt_output_language: PromptOutputLanguage,
    optimized_metadata: Value,
    request: ChatRequest,
    call_key: String,
    input_fingerprint_json: Value,
    messages_json: Value,
    input_variables_json: Value,
    source_refs_json: Value,
    media_refs_json: Option<Value>,
}

struct VideoPromptOptimizationCallResult {
    response_text: String,
    response_usage_json: Option<Value>,
    response_finish_reason: Option<String>,
    visual_prompt: String,
    motion_prompt: Option<String>,
    negative_prompt: Option<String>,
}

struct ValidatedVideoPromptOptimization {
    parsed: OptimizedVideoPromptFields,
    visual_prompt: String,
    motion_prompt: Option<String>,
    negative_prompt: Option<String>,
}

impl OrchestratorContext {
    pub(super) async fn optimize_video_prompt_validated(
        &self,
        input: VideoPromptOptimizationInput,
        prompt_renderer: &dyn PromptRenderer,
        cleanup_port: &dyn VideoPromptDownstreamCleanupPort,
    ) -> Result<chapter_video_prompt::Model, AppError> {
        let prepared = self
            .prepare_video_prompt_optimization(input, prompt_renderer)
            .await?;
        let workflow_run = self
            .begin_video_prompt_optimization_workflow_validated(
                prepared.prompt.project_id,
                prepared.prompt.id,
                prepared.prompt.chapter_id,
                &prepared.prompt.workflow_mode,
                &prepared.call_key,
                prepared.input_fingerprint_json.clone(),
            )
            .await?;
        let result = self
            .execute_video_prompt_optimization(workflow_run.id, prepared, cleanup_port)
            .await;
        self.finalize_workflow_result(workflow_run.id, result, "视频提示词优化")
            .await
    }

    async fn prepare_video_prompt_optimization(
        &self,
        input: VideoPromptOptimizationInput,
        prompt_renderer: &dyn PromptRenderer,
    ) -> Result<PreparedVideoPromptOptimization, AppError> {
        if input.prompt.id <= 0 {
            return Err(AppError::InvalidArgument(
                "视频提示词 ID 必须大于 0".to_string(),
            ));
        }
        let instruction = input.instruction.trim();
        if instruction.is_empty() {
            return Err(AppError::InvalidArgument("优化要求不能为空".to_string()));
        }
        let prompt_kind = video_prompt_optimization_kind(&input.prompt)?;
        let current_metadata = input.prompt.metadata_json.clone().ok_or_else(|| {
            AppError::InvalidArgument(format!(
                "视频提示词 #{} 缺少 metadata_json，不能优化",
                input.prompt.id
            ))
        })?;
        let current_language = PromptOutputLanguage::from_metadata(Some(&current_metadata))?;
        self.project_chapter_video_source(
            input.prompt.project_id,
            input.prompt.chapter_id,
            "优化视频提示词",
            true,
        )
        .await?;

        let prompt_output_language = if input.prompt.timeline_item_id.is_some() {
            let recipe_language = current_metadata
                .get("segment_recipe")
                .and_then(|recipe| recipe.get("prompt_output_language"))
                .and_then(Value::as_str)
                .ok_or_else(|| {
                    AppError::InvalidArgument(
                        "插入片段提示词缺少 segment_recipe.prompt_output_language，不能优化"
                            .to_string(),
                    )
                })?;
            let recipe_language = PromptOutputLanguage::from_code(recipe_language)?;
            let requested = prompt_output_language_from_instruction(instruction, current_language);
            if requested != recipe_language {
                return Err(AppError::InvalidArgument(
                    "插入片段输出语言由 segment_recipe 管理，请修改执行方案后重新生成提示词"
                        .to_string(),
                ));
            }
            recipe_language
        } else {
            prompt_output_language_from_instruction(instruction, current_language)
        };
        let request = build_video_prompt_optimization_request(
            prompt_renderer,
            &input.prompt,
            instruction,
            prompt_output_language,
            prompt_kind,
        )
        .await?;
        let request_json = context_aware_chat_request_trace_json(&request)
            .map_err(|err| AppError::Internal(anyhow::anyhow!("LLM 请求序列化失败: {err}")))?;
        let messages_json = serde_json::to_value(&request.messages)?;
        let input_variables_json = json!({
            "instruction": instruction,
            "prompt_id": input.prompt.id,
            "prompt_type": &input.prompt.prompt_type,
            "workflow_mode": &input.prompt.workflow_mode,
            "shot_number": input.prompt.shot_number,
            "prompt_output_language": prompt_output_language.as_str(),
        });
        let source_refs_json = json!({
            "previous_prompt_id": input.prompt.id,
            "previous_visual_prompt": &input.prompt.visual_prompt,
            "previous_motion_prompt": &input.prompt.motion_prompt,
            "previous_negative_prompt": &input.prompt.negative_prompt,
        });
        let media_refs_json = input.prompt.reference_image_ids_json.clone();
        let input_fingerprint_json = json!({
            "prompt_names": [OPTIMIZATION_PROMPT_NAME, OPTIMIZATION_USER_PROMPT_NAME],
            "request": request_json,
            "input_variables": input_variables_json.clone(),
            "source_refs": source_refs_json.clone(),
            "media_refs": media_refs_json.clone(),
        });
        let optimized_metadata =
            with_prompt_output_language_metadata(current_metadata, prompt_output_language);

        Ok(PreparedVideoPromptOptimization {
            call_key: video_prompt_optimization_call_key(input.prompt.id),
            prompt: input.prompt,
            prompt_kind,
            allow_downstream: input.allow_downstream,
            delete_dependencies: input.delete_dependencies,
            prompt_output_language,
            optimized_metadata,
            request,
            input_fingerprint_json,
            messages_json,
            input_variables_json,
            source_refs_json,
            media_refs_json,
        })
    }

    async fn execute_video_prompt_optimization(
        &self,
        workflow_run_id: i64,
        mut prepared: PreparedVideoPromptOptimization,
        cleanup_port: &dyn VideoPromptDownstreamCleanupPort,
    ) -> Result<chapter_video_prompt::Model, AppError> {
        let generation_input_fingerprint_sha256 =
            generation_checkpoint_fingerprint_sha256(&prepared.input_fingerprint_json)?;
        workflow::attach_llm_call_checkpoint_trace(
            &mut prepared.request,
            workflow_run_id,
            &prepared.call_key,
            &prepared.input_fingerprint_json,
        )?;
        let request_audit_json = context_aware_chat_request_trace_json(&prepared.request)
            .map_err(|err| AppError::Internal(anyhow::anyhow!("LLM 请求序列化失败: {err}")))?;
        self.mark_workflow_current_step(workflow_run_id, &prepared.call_key)
            .await?;
        let call_result = self
            .run_video_prompt_optimization_call(
                &prepared.prompt,
                prepared.prompt_kind,
                prepared.prompt_output_language,
                prepared.request,
                workflow_run_id,
                &prepared.call_key,
                &prepared.input_fingerprint_json,
            )
            .await?;

        if let Some(dependencies) = prepared.delete_dependencies.as_ref() {
            cleanup_port
                .delete_downstream_media(&dependencies.image_ids, &dependencies.video_ids)
                .await?;
            if !dependencies.prompt_ids.is_empty() {
                self.delete_chapter_video_prompts_checked(&dependencies.prompt_ids, true)
                    .await?;
            }
        }

        let VideoPromptOptimizationCallResult {
            response_text,
            response_usage_json,
            response_finish_reason,
            visual_prompt,
            motion_prompt,
            negative_prompt,
        } = call_result;
        self.update_chapter_video_prompt_with_generation_run_checkpoint(
            prepared.prompt.id,
            ChapterVideoPromptPatch {
                visual_prompt: Some(visual_prompt.clone()),
                motion_prompt: Some(motion_prompt.clone()),
                negative_prompt: Some(negative_prompt.clone()),
                duration_secs: None,
                reference_image_ids_json: None,
            },
            Some(prepared.optimized_metadata),
            prepared.allow_downstream,
            RecordCompletedGenerationRunWithCheckpointInput {
                run: RecordCompletedGenerationRunInput {
                    project_id: prepared.prompt.project_id,
                    workflow: "video_prompt_generation".to_string(),
                    step_key: "optimize_chapter_video_prompt".to_string(),
                    title: "视频提示词优化".to_string(),
                    model_kind: "llm".to_string(),
                    provider: None,
                    model: None,
                    prompt_template_names_json: Some(json!([
                        OPTIMIZATION_PROMPT_NAME,
                        OPTIMIZATION_USER_PROMPT_NAME,
                    ])),
                    prompt_template_hashes_json: None,
                    rendered_messages_json: Some(prepared.messages_json),
                    rendered_prompts_json: None,
                    input_variables_json: Some(prepared.input_variables_json),
                    source_refs_json: Some(prepared.source_refs_json),
                    media_refs_json: prepared.media_refs_json,
                    request_json: Some(request_audit_json),
                    response_summary_json: Some(json!({
                        "visual_prompt_chars": visual_prompt.chars().count(),
                        "motion_prompt_chars": motion_prompt
                            .as_deref()
                            .map(|value| value.chars().count()),
                        "negative_prompt_chars": negative_prompt
                            .as_deref()
                            .map(|value| value.chars().count()),
                    })),
                    targets: Vec::new(),
                },
                checkpoint: GenerationRunCheckpointContext {
                    workflow_run_id: Some(workflow_run_id),
                    call_key: Some(prepared.call_key),
                    input_fingerprint_sha256: Some(generation_input_fingerprint_sha256),
                    input_fingerprint_json: Some(prepared.input_fingerprint_json),
                    raw_response_json: Some(json!({
                        "text": response_text,
                        "usage": response_usage_json,
                        "finish_reason": response_finish_reason,
                    })),
                    parsed_output_json: Some(json!({
                        "optimized_fields": {
                            "visual_prompt": visual_prompt,
                            "motion_prompt": motion_prompt,
                            "negative_prompt": negative_prompt,
                        },
                    })),
                    validation_json: None,
                    artifact_refs_json: None,
                    attempt: None,
                },
            },
        )
        .await
    }

    async fn run_video_prompt_optimization_call(
        &self,
        prompt: &chapter_video_prompt::Model,
        prompt_kind: VideoPromptOptimizationKind,
        prompt_output_language: PromptOutputLanguage,
        request: ChatRequest,
        workflow_run_id: i64,
        call_key: &str,
        input_fingerprint_json: &Value,
    ) -> Result<VideoPromptOptimizationCallResult, AppError> {
        let (
            validated,
            response_text,
            save_checkpoint,
            response_usage_json,
            response_finish_reason,
        ) = if let Some(checkpoint) = self
            .load_completed_llm_call_checkpoint(workflow_run_id, call_key, input_fingerprint_json)
            .await?
        {
            let optimized = video_prompt_optimization_checkpoint_output(
                &checkpoint.parsed_output,
                &checkpoint.response_content,
                call_key,
            )
            .map_err(AppError::InvalidArgument)?;
            let validated = validate_video_prompt_optimization(
                prompt,
                prompt_kind,
                optimized,
                prompt_output_language,
            )?;
            (
                validated,
                checkpoint.response_content,
                false,
                checkpoint.usage,
                checkpoint.finish_reason,
            )
        } else {
            let response = self
                .stream_complete_llm(request)
                .await
                .map_err(|err| super::video_llm_step_error(OPTIMIZATION_PROMPT_NAME, err))?;
            if is_truncated_finish_reason(response.finish_reason.as_deref()) {
                return Err(AppError::InvalidArgument(
                    "优化视频提示词输出达到模型 max_output_tokens 上限，返回 JSON 被截断；请提高模型输出上限，或缩短本次提示词后重试".to_string(),
                ));
            }
            let response_usage_json = response
                .usage
                .as_ref()
                .map(serde_json::to_value)
                .transpose()?;
            let response_finish_reason = response.finish_reason.clone();
            let response_text = response.message.content;
            let optimized = parse_strict_json_response::<OptimizedVideoPromptFields>(
                &response_text,
                "视频提示词优化",
            )?;
            let validated = validate_video_prompt_optimization(
                prompt,
                prompt_kind,
                optimized,
                prompt_output_language,
            )?;
            (
                validated,
                response_text,
                true,
                response_usage_json,
                response_finish_reason,
            )
        };

        let ValidatedVideoPromptOptimization {
            parsed,
            visual_prompt,
            motion_prompt,
            negative_prompt,
        } = validated;
        if save_checkpoint {
            self.save_completed_llm_call_checkpoint(CompletedLlmCallCheckpointInput {
                workflow_run_id,
                call_key: call_key.to_string(),
                call_order: 1,
                input_fingerprint_json: input_fingerprint_json.clone(),
                checkpoint: CompletedLlmCallCheckpoint {
                    parsed_output: json!({
                        "visual_prompt": parsed.visual_prompt,
                        "motion_prompt": parsed.motion_prompt,
                        "negative_prompt": parsed.negative_prompt,
                    }),
                    response_content: response_text.clone(),
                    usage: response_usage_json.clone(),
                    finish_reason: response_finish_reason.clone(),
                    artifacts_json: None,
                },
            })
            .await?;
        }

        Ok(VideoPromptOptimizationCallResult {
            response_text,
            response_usage_json,
            response_finish_reason,
            visual_prompt,
            motion_prompt,
            negative_prompt,
        })
    }
}

async fn build_video_prompt_optimization_request(
    prompt_renderer: &dyn PromptRenderer,
    prompt: &chapter_video_prompt::Model,
    instruction: &str,
    prompt_output_language: PromptOutputLanguage,
    prompt_kind: VideoPromptOptimizationKind,
) -> Result<ChatRequest, AppError> {
    let dynamic_payload = build_video_prompt_optimization_dynamic_payload(
        instruction,
        prompt_output_language,
        prompt_kind,
    );
    let payload = serde_json::to_string_pretty(&dynamic_payload)?;
    let variables = HashMap::from([("payload".to_string(), json!(payload))]);
    let system_prompt = prompt_renderer
        .render_prompt(
            OPTIMIZATION_PROMPT_NAME,
            &variables,
            Some(prompt.project_id),
        )
        .await?;
    let user_prompt = prompt_renderer
        .render_prompt(
            OPTIMIZATION_USER_PROMPT_NAME,
            &variables,
            Some(prompt.project_id),
        )
        .await?;
    build_video_prompt_optimization_request_with_prompts(
        system_prompt.trim().to_string(),
        user_prompt.trim().to_string(),
        prompt,
        instruction,
        prompt_output_language,
    )
}

fn build_video_prompt_optimization_request_with_prompts(
    system_prompt: String,
    user_prompt: String,
    prompt: &chapter_video_prompt::Model,
    instruction: &str,
    prompt_output_language: PromptOutputLanguage,
) -> Result<ChatRequest, AppError> {
    let current_prompt = video_prompt_optimization_source_artifact(prompt);
    let source_hash = source_content_sha256(&serde_json::to_string(&current_prompt)?);
    let mut ledger = LlmContextLedger::new();
    ledger.add_pack(a4n_llm_context::LlmContextLedgerPackInput {
        scope_kind: "prompt_optimization".into(),
        pack_kind: "source_artifact".into(),
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
    })?;
    let manifest = ledger.into_manifest(
        user_prompt,
        Some("视频提示词优化必须保留稳定上下文包 source_artifact 中的事实、结构和素材约束，只按动态输入中的 instruction 修改表达。".to_string()),
    );
    let fingerprint = source_content_sha256(&serde_json::to_string(&json!({
        "prompt_id": prompt.id,
        "source_artifact_sha256": source_hash,
        "instruction": instruction,
        "prompt_output_language": prompt_output_language.as_str(),
        "prompt_names": [OPTIMIZATION_PROMPT_NAME, OPTIMIZATION_USER_PROMPT_NAME],
    }))?);
    Ok(
        context_aware_system_user_request(system_prompt, manifest, Some(0.25), None)?
            .with_json_response_format()
            .with_trace_context(LlmTraceContext {
                project_id: Some(prompt.project_id),
                root_project_id: None,
                workflow: "video_prompt_optimization".to_string(),
                step_key: "optimize_chapter_video_prompt".to_string(),
                generation_run_id: None,
                workflow_run_id: None,
                call_key: Some(video_prompt_optimization_call_key(prompt.id)),
                input_fingerprint_sha256: Some(fingerprint),
                context_plan_json: None,
                context_pack_refs_json: None,
            }),
    )
}

fn video_prompt_optimization_call_key(prompt_id: i64) -> String {
    format!("optimize_chapter_video_prompt:prompt:{prompt_id}")
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
            "negative_prompt": prompt.negative_prompt.as_deref(),
        },
    })
}

fn build_video_prompt_optimization_dynamic_payload(
    instruction: &str,
    prompt_output_language: PromptOutputLanguage,
    prompt_kind: VideoPromptOptimizationKind,
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
            "asset_contract": optimized_prompt_asset_contract(prompt_kind, prompt_output_language),
            "field_rules": {
                "visual_prompt": "优化后的画面/主体提示词，不能为空；自然语言必须遵守 prompt_output_language_instruction；原文对白、引号内台词和角色实际说出口的短句必须保留原文写法，不要把中文对白翻成英文",
                "motion_prompt": "provider_video_prompt 必须返回完整非空字符串；chat_video_prompt 和 frame_prompt 必须返回 null；自然语言必须遵守 prompt_output_language_instruction；原文对白、引号内台词和角色实际说出口的短句必须保留原文写法，不要把中文对白翻成英文",
                "negative_prompt": "最终负面提示词；需要保留原值时必须原样返回，最终不需要负面提示词时返回 null；不得用 null 表示沿用旧值；自然语言必须遵守 prompt_output_language_instruction",
            },
            "forbidden": [
                "markdown",
                "code_fence",
                "extra_fields",
                "json_inside_prompt_text",
                "internal_image_id",
                "upload_status_text",
                "motion_timeline_duplicate",
            ],
        },
    })
}

fn optimized_prompt_asset_contract(
    prompt_kind: VideoPromptOptimizationKind,
    prompt_output_language: PromptOutputLanguage,
) -> Value {
    match prompt_kind {
        VideoPromptOptimizationKind::ProviderVideo => json!({
            "kind": "provider_video_prompt",
            "visual_prompt_required_sections": prompt_output_language.visual_sections(),
            "motion_prompt_required_sections": prompt_output_language.motion_sections(),
            "motion_prompt_required": true,
            "motion_prompt_must_not_repeat_timing_breakdown": true,
            "minimum_lengths": {
                "visual_prompt_chars": MIN_PROVIDER_VISUAL_PROMPT_CHARS,
                "motion_prompt_chars": MIN_PROVIDER_MOTION_PROMPT_CHARS,
            },
            "forbidden_terms": {
                "upload_status_examples": ["无输入首帧", "没有输入尾帧"],
                "internal_workflow_examples": ["首帧串联", "provider", "image 字段", "generated_image #"],
            },
        }),
        VideoPromptOptimizationKind::ChatVideo => json!({
            "kind": "chat_video_prompt",
            "visual_prompt_is_chat_prompt": true,
            "visual_prompt_required_sections": prompt_output_language.visual_sections(),
            "motion_prompt_must_be_null": true,
            "required_avoidance_rule": "限制与避免项必须明确禁止背景音乐/配乐/BGM",
            "forbidden_terms": {
                "upload_status_examples": ["无输入首帧", "没有输入尾帧"],
                "internal_workflow_examples": ["首帧串联", "provider", "image 字段", "generated_image #"],
            },
        }),
        VideoPromptOptimizationKind::Frame => json!({
            "kind": "frame_prompt",
            "visual_prompt_is_frame_prompt": true,
            "visual_prompt_required_sections": prompt_output_language.frame_sections(),
            "motion_prompt_must_be_null": true,
            "minimum_lengths": {"visual_prompt_chars": MIN_FRAME_PROMPT_CHARS},
            "forbidden_terms": {
                "internal_workflow_examples": ["首帧串联", "provider", "image 字段", "generated_image #"],
            },
        }),
    }
}

fn required_optimized_text(value: &str, field: &str) -> Result<String, AppError> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(AppError::InvalidArgument(format!(
            "优化结果 {field} 不能为空"
        )));
    }
    Ok(trimmed.to_string())
}

fn validate_video_prompt_optimization(
    prompt: &chapter_video_prompt::Model,
    prompt_kind: VideoPromptOptimizationKind,
    parsed: OptimizedVideoPromptFields,
    prompt_output_language: PromptOutputLanguage,
) -> Result<ValidatedVideoPromptOptimization, AppError> {
    let negative_prompt = parsed
        .negative_prompt
        .as_deref()
        .map(|value| required_optimized_text(value, "negative_prompt"))
        .transpose()?;
    let (visual_prompt, motion_prompt) = match prompt_kind {
        VideoPromptOptimizationKind::ProviderVideo => {
            let visual_prompt = validate_provider_model_text(
                &parsed.visual_prompt,
                "visual_prompt",
                prompt.id,
                prompt_output_language,
                MIN_PROVIDER_VISUAL_PROMPT_CHARS,
            )
            .map(str::to_string)
            .map_err(AppError::InvalidArgument)?;
            let motion_prompt = parsed.motion_prompt.as_deref().ok_or_else(|| {
                AppError::InvalidArgument(format!(
                    "优化结果 motion_prompt 不能为空；视频提示词 #{} 生成视频需要运镜与动作提示词",
                    prompt.id
                ))
            })?;
            let motion_prompt = validate_provider_model_text(
                motion_prompt,
                "motion_prompt",
                prompt.id,
                prompt_output_language,
                MIN_PROVIDER_MOTION_PROMPT_CHARS,
            )
            .map(str::to_string)
            .map_err(AppError::InvalidArgument)?;
            (visual_prompt, Some(motion_prompt))
        }
        VideoPromptOptimizationKind::ChatVideo => {
            if parsed.motion_prompt.is_some() {
                return Err(AppError::InvalidArgument(format!(
                    "Chat 视频提示词 #{} 的优化结果 motion_prompt 必须为 null",
                    prompt.id
                )));
            }
            let visual_prompt = required_optimized_text(&parsed.visual_prompt, "visual_prompt")?;
            validate_chat_video_prompt_text(&visual_prompt, prompt_output_language)?;
            (visual_prompt, None)
        }
        VideoPromptOptimizationKind::Frame => {
            if parsed.motion_prompt.is_some() {
                return Err(AppError::InvalidArgument(format!(
                    "帧提示词 #{} 的优化结果 motion_prompt 必须为 null",
                    prompt.id
                )));
            }
            let visual_prompt = validate_provider_model_text(
                &parsed.visual_prompt,
                "frame_prompt",
                prompt.id,
                prompt_output_language,
                MIN_FRAME_PROMPT_CHARS,
            )
            .map(str::to_string)
            .map_err(AppError::InvalidArgument)?;
            (visual_prompt, None)
        }
    };
    if let Some(negative_prompt) = negative_prompt.as_deref() {
        validate_direct_prompt_text(negative_prompt, "视频提示词 negative_prompt")?;
    }
    Ok(ValidatedVideoPromptOptimization {
        parsed,
        visual_prompt,
        motion_prompt,
        negative_prompt,
    })
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use a4n_prompt::tera_engine::TeraPromptEngine;
    use chrono::Utc;

    use super::*;

    fn test_prompt(prompt_type: &str, workflow_mode: &str) -> chapter_video_prompt::Model {
        let now = Utc::now().naive_utc();
        chapter_video_prompt::Model {
            id: 9,
            project_id: 1,
            chapter_id: 2,
            style: Some("电影感写实".to_string()),
            prompt_type: prompt_type.to_string(),
            workflow_mode: workflow_mode.to_string(),
            shot_number: 3,
            title: Some("穿过走廊".to_string()),
            scene_summary: Some("主角沿走廊前进并停在门前".to_string()),
            visual_prompt: "现有画面提示词".to_string(),
            motion_prompt: Some("现有运镜提示词".to_string()),
            negative_prompt: Some("现有负面提示词".to_string()),
            duration_secs: 5,
            aspect_ratio: Some("16:9".to_string()),
            reference_image_ids_json: None,
            status: chapter_video_prompt::STATUS_COMPLETED.to_string(),
            generated_video_id: None,
            timeline_item_id: None,
            metadata_json: None,
            created_at: now,
            updated_at: now,
        }
    }

    fn prompt_with_sections(sections: &[&str], body: &str) -> String {
        sections
            .iter()
            .map(|section| format!("{section}\n{body}"))
            .collect::<Vec<_>>()
            .join("\n")
    }

    #[test]
    fn optimization_kind_requires_an_exact_type_and_workflow_pair() {
        assert_eq!(
            video_prompt_optimization_kind(&test_prompt(
                chapter_video_prompt::PROMPT_TYPE_VIDEO,
                chapter_video_prompt::WORKFLOW_MODE_BOUNDARY_FRAMES,
            ))
            .expect("boundary video prompt should be optimizable"),
            VideoPromptOptimizationKind::ProviderVideo
        );
        assert_eq!(
            video_prompt_optimization_kind(&test_prompt(
                chapter_video_prompt::PROMPT_TYPE_VIDEO,
                chapter_video_prompt::WORKFLOW_MODE_CHAT_VIDEO,
            ))
            .expect("chat video prompt should be optimizable"),
            VideoPromptOptimizationKind::ChatVideo
        );
        assert_eq!(
            video_prompt_optimization_kind(&test_prompt(
                chapter_video_prompt::PROMPT_TYPE_STORYBOARD,
                chapter_video_prompt::WORKFLOW_MODE_STORYBOARD,
            ))
            .expect("storyboard frame prompt should be optimizable"),
            VideoPromptOptimizationKind::Frame
        );
        assert!(
            video_prompt_optimization_kind(&test_prompt(
                chapter_video_prompt::PROMPT_TYPE_STORYBOARD,
                chapter_video_prompt::WORKFLOW_MODE_CHAT_VIDEO,
            ))
            .is_err()
        );
    }

    #[test]
    fn optimization_output_enforces_kind_specific_nullable_fields() {
        let language = PromptOutputLanguage::ZhCn;
        let visual = prompt_with_sections(
            language.visual_sections(),
            "角色身份、服装、空间位置、动作、光线和道具关系保持清晰稳定，不增加片段外事件。",
        );
        let motion = prompt_with_sections(
            language.motion_sections(),
            "镜头平稳跟随角色向前，动作节奏清楚，视线、手部和停顿均与当前表演连续。",
        );
        let frame = prompt_with_sections(
            language.frame_sections(),
            "角色外观、空间锚点、道具位置、构图、光线与画面连续性均清晰稳定。",
        );
        let chat = prompt_with_sections(
            language.visual_sections(),
            "角色身份、空间关系、镜头动作和稳定尾态均清楚；镜头 1：角色从门口向前移动并停在画面中央；不要配背景音乐。",
        );

        let provider = validate_video_prompt_optimization(
            &test_prompt(
                chapter_video_prompt::PROMPT_TYPE_VIDEO,
                chapter_video_prompt::WORKFLOW_MODE_BOUNDARY_FRAMES,
            ),
            VideoPromptOptimizationKind::ProviderVideo,
            OptimizedVideoPromptFields {
                visual_prompt: visual,
                motion_prompt: Some(motion),
                negative_prompt: None,
            },
            language,
        )
        .expect("provider prompt should require and retain motion_prompt");
        assert!(provider.motion_prompt.is_some());
        assert!(provider.negative_prompt.is_none());

        validate_video_prompt_optimization(
            &test_prompt(
                chapter_video_prompt::PROMPT_TYPE_VIDEO,
                chapter_video_prompt::WORKFLOW_MODE_CHAT_VIDEO,
            ),
            VideoPromptOptimizationKind::ChatVideo,
            OptimizedVideoPromptFields {
                visual_prompt: chat,
                motion_prompt: None,
                negative_prompt: None,
            },
            language,
        )
        .expect("chat prompt should require a null motion_prompt");

        validate_video_prompt_optimization(
            &test_prompt(
                chapter_video_prompt::PROMPT_TYPE_STORYBOARD,
                chapter_video_prompt::WORKFLOW_MODE_STORYBOARD,
            ),
            VideoPromptOptimizationKind::Frame,
            OptimizedVideoPromptFields {
                visual_prompt: frame,
                motion_prompt: None,
                negative_prompt: None,
            },
            language,
        )
        .expect("frame prompt should require a null motion_prompt");

        assert!(
            serde_json::from_value::<OptimizedVideoPromptFields>(json!({
                "visual_prompt": "完整提示词",
                "negative_prompt": null,
            }))
            .is_err()
        );
        assert!(
            serde_json::from_value::<OptimizedVideoPromptFields>(json!({
                "visual_prompt": "完整提示词",
                "motion_prompt": null,
            }))
            .is_err()
        );
    }

    #[tokio::test]
    async fn request_uses_registry_and_stable_source_pack() {
        let engine = TeraPromptEngine::from_directory(
            &Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../prompts"),
        )
        .expect("prompts should load");
        let prompt = test_prompt(
            chapter_video_prompt::PROMPT_TYPE_VIDEO,
            chapter_video_prompt::WORKFLOW_MODE_BOUNDARY_FRAMES,
        );

        let request = build_video_prompt_optimization_request(
            &engine,
            &prompt,
            "让镜头动作更具体，但不要改变空间关系",
            PromptOutputLanguage::ZhCn,
            VideoPromptOptimizationKind::ProviderVideo,
        )
        .await
        .expect("optimization prompt templates should render");

        assert_eq!(request.messages.len(), 2);
        assert!(request.messages[1].content.contains("让镜头动作更具体"));
        assert!(request.messages[1].content.contains("\"output_contract\""));
        assert_eq!(
            request.context_manifest_json.as_ref().expect("manifest")["packs"][0]["source_refs_json"]
                ["prompt_id"],
            json!(9)
        );
    }
}
