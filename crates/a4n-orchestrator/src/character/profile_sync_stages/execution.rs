use a4n_core::error::AppError;
use a4n_llm::{provider::is_truncated_finish_reason, trace::context_aware_chat_request_trace_json};
use a4n_prompt::rendering::PromptRenderer;
use serde_json::{Value, json};

use crate::{
    OrchestratorContext,
    character::{
        CharacterProfileSyncProgress, optional_character_sync_checkpoint_string,
        required_character_sync_checkpoint_string, validate_contiguous_character_sync_range,
    },
    workflow::{self, CompletedLlmCallCheckpointInput, workflow_checkpoint_contract_app_error},
};

use super::{
    contracts::{
        CharacterProfileSyncExtraction, CharacterProfileSyncGenerationTrace,
        CharacterProfileSyncPreviewStep, CharacterProfileSyncStage,
        CharacterProfileSyncStageExchange, CharacterProfileSyncStageInput,
        CharacterProfileSyncStageOutput, CharacterProfileSyncStageOutputs,
        CharacterProfileSyncStageTraceInput, CharacterProfileSyncStageTracePort,
    },
    output::{
        parse_character_profile_sync_stage_output, validate_character_profile_sync_extraction,
    },
    request::{
        PreparedCharacterProfileSyncStageCall, build_character_profile_sync_stage_request,
        prepare_character_profile_sync_stage_call,
    },
};

impl OrchestratorContext {
    pub(crate) async fn prepare_character_profile_sync_initial_request_preview_validated(
        &self,
        input: CharacterProfileSyncStageInput,
        prompt_renderer: &dyn PromptRenderer,
    ) -> Result<CharacterProfileSyncPreviewStep, AppError> {
        validate_stage_input(&input, false)?;
        let stage = CharacterProfileSyncStage::EvidenceEvents;
        Ok(CharacterProfileSyncPreviewStep {
            id: format!("sync_{}_{}", stage.key(), input.profile_id),
            title: format!("{}：{}", input.profile_name, stage.title()),
            prompt_name: stage.prompt_name(),
            note: format!("同步第 {}-{} 章", input.start_chapter, input.target_chapter),
            request: build_character_profile_sync_stage_request(
                &input,
                stage,
                &json!({}),
                prompt_renderer,
            )
            .await?,
        })
    }

    pub(crate) async fn run_character_profile_sync_stages_validated(
        &self,
        input: CharacterProfileSyncStageInput,
        prompt_renderer: &dyn PromptRenderer,
        progress: Option<CharacterProfileSyncProgress<'_>>,
        trace_port: Option<&dyn CharacterProfileSyncStageTracePort>,
    ) -> Result<CharacterProfileSyncStageOutput, AppError> {
        validate_stage_input(&input, true)?;
        let workflow_run_id = workflow::required_llm_call_workflow_run_id(
            input.workflow_run_id,
            "character_profile_sync_stages",
        )
        .map_err(workflow_checkpoint_contract_app_error)?;
        let mut outputs = CharacterProfileSyncStageOutputs::default();
        let mut previous_steps = json!({});
        let mut exchanges = Vec::new();
        let mut llm_trace_paths = Vec::new();

        for (index, stage) in CharacterProfileSyncStage::ALL.iter().copied().enumerate() {
            let current = u32::try_from(index)
                .map_err(|_| {
                    AppError::InvalidArgument(format!("角色同步阶段进度索引超过 u32 范围：{index}"))
                })?
                .checked_add(1)
                .ok_or_else(|| {
                    AppError::InvalidArgument(format!("角色同步阶段进度索引超过 u32 范围：{index}"))
                })?;
            report_progress(
                progress,
                current,
                &stage.progress_message(&input.profile_name),
            )?;
            let call = self
                .run_character_profile_sync_stage_call(
                    &input,
                    workflow_run_id,
                    stage,
                    &previous_steps,
                    prompt_renderer,
                )
                .await?;
            let parsed = parse_character_profile_sync_stage_output(
                stage,
                &call.response_content,
                &input,
                &mut outputs,
            );
            match parsed {
                Ok(stage_value) => {
                    if let Some(parsed_output) = call.restored_parsed_output {
                        if parsed_output != stage_value {
                            return Err(AppError::InvalidArgument(format!(
                                "{} checkpoint 的 parsed_output 与 response_content 重新解析结果不一致",
                                call.call_key
                            )));
                        }
                    } else {
                        let call_order = i32::try_from(index)
                            .map_err(|_| {
                                AppError::InvalidArgument(format!(
                                    "角色同步阶段索引超过 i32 范围：{index}"
                                ))
                            })?
                            .checked_add(1)
                            .ok_or_else(|| {
                                AppError::InvalidArgument(format!(
                                    "角色同步阶段 call_order 超过 i32 范围：{index}"
                                ))
                            })?;
                        self.save_completed_llm_call_checkpoint(CompletedLlmCallCheckpointInput {
                            workflow_run_id,
                            call_key: call.call_key.clone(),
                            call_order,
                            input_fingerprint_json: call.input_fingerprint_json.clone(),
                            checkpoint_json: json!({
                                "output": stage_value.clone(),
                                "parsed_output": stage_value.clone(),
                                "response_content": call.response_content.clone(),
                                "usage": call.response_usage_json.clone(),
                                "finish_reason": call.response_finish_reason.clone(),
                            }),
                            artifacts_json: None,
                        })
                        .await?;
                    }
                    record_trace(
                        trace_port,
                        &input.profile_name,
                        format!("{}_success", stage.key()),
                        &call.request,
                        &call.response_content,
                        stage.temperature(),
                        None,
                        &mut llm_trace_paths,
                    );
                    previous_steps[stage.key()] = stage_value;
                    exchanges.push(CharacterProfileSyncStageExchange {
                        stage,
                        request: call.request,
                        response_content: call.response_content,
                    });
                }
                Err(error) => {
                    record_trace(
                        trace_port,
                        &input.profile_name,
                        format!("{}_invalid", stage.key()),
                        &call.request,
                        &call.response_content,
                        stage.temperature(),
                        Some(error.clone()),
                        &mut llm_trace_paths,
                    );
                    let trace_summary = if llm_trace_paths.is_empty() {
                        String::new()
                    } else {
                        format!("；LLM 调试日志：{}", llm_trace_paths.join("，"))
                    };
                    return Err(AppError::InvalidArgument(format!(
                        "{}输出不符合角色同步阶段契约：{error}{trace_summary}",
                        stage.title()
                    )));
                }
            }
        }

        report_progress(
            progress,
            6,
            &format!("{}：正在组合并校验角色同步阶段结果", input.profile_name),
        )?;
        let extraction = validate_character_profile_sync_extraction(
            CharacterProfileSyncExtraction {
                changes: outputs.changes,
                behaviors: outputs.behaviors,
                deletion_candidates: outputs.deletion_candidates,
                chapter_states: outputs.chapter_states,
            },
            &input,
        )
        .map_err(AppError::InvalidArgument)?;
        let generation_trace = generation_trace_from_exchanges(&exchanges)?;
        Ok(CharacterProfileSyncStageOutput {
            extraction,
            generation_trace,
            llm_trace_paths,
        })
    }

    async fn run_character_profile_sync_stage_call(
        &self,
        input: &CharacterProfileSyncStageInput,
        workflow_run_id: i64,
        stage: CharacterProfileSyncStage,
        previous_steps: &Value,
        prompt_renderer: &dyn PromptRenderer,
    ) -> Result<CharacterProfileSyncStageCallResult, AppError> {
        let PreparedCharacterProfileSyncStageCall {
            request,
            call_key,
            input_fingerprint_json,
        } = prepare_character_profile_sync_stage_call(
            input,
            stage,
            previous_steps,
            prompt_renderer,
        )
        .await?;
        self.mark_workflow_current_step(workflow_run_id, &call_key)
            .await?;
        let (response_content, response_usage_json, response_finish_reason, restored_parsed_output) =
            if let Some(checkpoint) = self
                .load_completed_llm_call_checkpoint(
                    workflow_run_id,
                    &call_key,
                    &input_fingerprint_json,
                )
                .await?
            {
                let checkpoint_json =
                    workflow::required_llm_call_checkpoint_json(checkpoint, &call_key)
                        .map_err(workflow_checkpoint_contract_app_error)?;
                let parsed_output =
                    checkpoint_json
                        .get("parsed_output")
                        .cloned()
                        .ok_or_else(|| {
                            AppError::InvalidArgument(format!("{call_key}.parsed_output 缺失"))
                        })?;
                let response_content = required_character_sync_checkpoint_string(
                    &checkpoint_json,
                    &call_key,
                    "response_content",
                )
                .map_err(AppError::InvalidArgument)?;
                let finish_reason =
                    optional_character_sync_checkpoint_string(&checkpoint_json, "finish_reason")
                        .map_err(AppError::InvalidArgument)?;
                ensure_response_not_truncated(
                    finish_reason.as_deref(),
                    &input.profile_name,
                    stage,
                    true,
                )?;
                let usage = checkpoint_json
                    .get("usage")
                    .filter(|value| !value.is_null())
                    .cloned();
                (response_content, usage, finish_reason, Some(parsed_output))
            } else {
                let response = self.stream_complete_llm(request.clone()).await?;
                ensure_response_not_truncated(
                    response.finish_reason.as_deref(),
                    &input.profile_name,
                    stage,
                    false,
                )?;
                let usage = response
                    .usage
                    .as_ref()
                    .map(serde_json::to_value)
                    .transpose()?;
                (
                    response.message.content,
                    usage,
                    response.finish_reason,
                    None,
                )
            };
        Ok(CharacterProfileSyncStageCallResult {
            call_key,
            request,
            input_fingerprint_json,
            response_content,
            response_usage_json,
            response_finish_reason,
            restored_parsed_output,
        })
    }
}

struct CharacterProfileSyncStageCallResult {
    call_key: String,
    request: a4n_llm::provider::ChatRequest,
    input_fingerprint_json: Value,
    response_content: String,
    response_usage_json: Option<Value>,
    response_finish_reason: Option<String>,
    restored_parsed_output: Option<Value>,
}

fn validate_stage_input(
    input: &CharacterProfileSyncStageInput,
    require_workflow: bool,
) -> Result<(), AppError> {
    if input.project_id <= 0 {
        return Err(AppError::InvalidArgument(
            "角色同步 project_id 必须大于 0".to_string(),
        ));
    }
    if input.profile_id <= 0 {
        return Err(AppError::InvalidArgument(
            "角色档案 ID 必须大于 0".to_string(),
        ));
    }
    if input.profile_name.trim().is_empty() {
        return Err(AppError::InvalidArgument(
            "角色档案名称不能为空".to_string(),
        ));
    }
    if require_workflow && input.workflow_run_id.is_none_or(|id| id <= 0) {
        return Err(AppError::InvalidArgument(
            "角色同步 workflow_run_id 必须大于 0".to_string(),
        ));
    }
    validate_contiguous_character_sync_range(
        input.start_chapter,
        input.target_chapter,
        &input.selected_chapter_numbers,
    )
    .map_err(AppError::InvalidArgument)
}

fn report_progress(
    progress: Option<CharacterProfileSyncProgress<'_>>,
    current: u32,
    message: &str,
) -> Result<(), AppError> {
    if let Some(progress) = progress {
        progress.report(current, message)?;
    }
    Ok(())
}

fn ensure_response_not_truncated(
    finish_reason: Option<&str>,
    profile_name: &str,
    stage: CharacterProfileSyncStage,
    restored: bool,
) -> Result<(), AppError> {
    if !is_truncated_finish_reason(finish_reason) {
        return Ok(());
    }
    let scope = format!("{profile_name} {}", stage.title());
    let suffix = if restored {
        "checkpoint 标记为截断响应，不能用于恢复"
    } else {
        "输出达到当前 max_tokens 上限，返回 JSON 被截断；请在设置页调大“角色同步”输出档位，或减少本次同步章节后重试"
    };
    Err(AppError::InvalidArgument(format!("{scope} {suffix}")))
}

fn record_trace(
    trace_port: Option<&dyn CharacterProfileSyncStageTracePort>,
    profile_name: &str,
    phase: String,
    request: &a4n_llm::provider::ChatRequest,
    response_content: &str,
    temperature: f32,
    parse_error: Option<String>,
    llm_trace_paths: &mut Vec<String>,
) {
    let Some(trace_port) = trace_port else {
        return;
    };
    if let Some(path) = trace_port.record_stage_trace(CharacterProfileSyncStageTraceInput {
        profile_name: profile_name.to_string(),
        phase,
        messages: request.messages.clone(),
        response_content: response_content.to_string(),
        temperature: Some(temperature),
        parse_error,
    }) {
        llm_trace_paths.push(path);
    }
}

fn generation_trace_from_exchanges(
    exchanges: &[CharacterProfileSyncStageExchange],
) -> Result<CharacterProfileSyncGenerationTrace, AppError> {
    let stage_requests = exchanges
        .iter()
        .map(|exchange| {
            context_aware_chat_request_trace_json(&exchange.request).map(|request| {
                json!({
                    "prompt_name": exchange.stage.prompt_name(),
                    "request": request,
                })
            })
        })
        .collect::<Result<Vec<_>, _>>()?;
    let stage_messages = exchanges
        .iter()
        .map(|exchange| {
            serde_json::to_value(&exchange.request.messages).map(|messages| {
                json!({
                    "prompt_name": exchange.stage.prompt_name(),
                    "messages": messages,
                })
            })
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok(CharacterProfileSyncGenerationTrace {
        request_json: json!({ "stages": stage_requests }),
        rendered_messages_json: json!({ "stages": stage_messages }),
        prompt_template_names_json: json!(
            exchanges
                .iter()
                .map(|exchange| exchange.stage.prompt_name())
                .collect::<Vec<_>>()
        ),
        response_chars: exchanges
            .iter()
            .map(|exchange| exchange.response_content.chars().count())
            .sum(),
    })
}
