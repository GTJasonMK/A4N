use a4n_api_contract::canvas::CanvasRunSummary;
use a4n_db::{
    agent::{AgentOwnedTask, AgentTaskExecutionInput},
    canvas::{CanvasPromptArtifactUpsertInput, FreeformCanvasGenerationSpecUpsertInput},
    generation::{
        GENERATION_SPEC_TARGET_ENTITY_KIND_CANVAS_NODE, GENERATION_SPEC_TARGET_KIND_CANVAS_PROMPT,
    },
};
use a4n_error::AppError;
use a4n_llm_provider_contract::provider::{
    ChatMessage, ChatRequest, ChatResponse, UsageInfo, is_truncated_finish_reason,
};
use a4n_llm_trace::context_aware_chat_request_trace_json;
use serde_json::{Value, json};

use crate::{
    OrchestratorContext,
    canvas::WORKFLOW_KIND_VIDEO_CANVAS_GRAPH,
    project::ProjectSourceMode,
    structured_llm::ensure_checkpoint_parsed_output_matches,
    workflow::{
        self, CompletedLlmCallCheckpoint, CompletedLlmCallCheckpointInput,
        WORKFLOW_STATUS_COMPLETED, WORKFLOW_STATUS_RUNNING, workflow_checkpoint_contract_app_error,
    },
};

use super::{
    contracts::{
        CanvasLlmTextInput, CanvasTextGenerationLlmCall, CanvasTextGenerationLlmPort,
        CanvasTextNodeExecutionInput,
    },
    input::{build_canvas_llm_text_input, build_canvas_rendered_prompt},
    request::build_canvas_llm_text_request,
};

struct ValidatedCanvasLlmText {
    transformed_text: String,
    usage: Option<UsageInfo>,
    finish_reason: Option<String>,
}

impl OrchestratorContext {
    pub async fn run_canvas_render_text_node_validated(
        &self,
        input: CanvasTextNodeExecutionInput<'_>,
    ) -> Result<CanvasRunSummary, AppError> {
        let checkpoints = self
            .canvas_text_source_checkpoints(input.shared_run.map(|run| run.workflow_run_id))
            .await?;
        let rendered =
            build_canvas_rendered_prompt(input.node, input.document, checkpoints.as_deref())
                .map_err(AppError::InvalidArgument)?;
        let step_key = crate::canvas::canvas_node_step_key(input.node.id)
            .map_err(AppError::InvalidArgument)?;
        let input_json = json!({
            "project_id": input.project_id,
            "graph_id": input.graph_id,
            "node_id": input.node.id,
            "prompt_kind": rendered.prompt_kind,
            "prompt_text": rendered.prompt_text,
            "canvas_text_asset_id": rendered.canvas_text_asset_id,
            "source_node_ids": rendered.source_node_ids,
        });
        let mut lifecycle = self
            .start_canvas_node_execution(
                input.project_id,
                input.graph_id,
                input.mode,
                step_key,
                input_json,
                input.shared_run,
            )
            .await?;

        let result = async {
            let spec = self
                .upsert_canvas_text_generation_spec(
                    &input,
                    rendered.canvas_text_asset_id,
                    WORKFLOW_STATUS_COMPLETED,
                    json!({
                        "prompt_text": rendered.prompt_text,
                        "source_node_ids": rendered.source_node_ids,
                    }),
                    json!({
                        "workflow": "render_prompt",
                        "prompt_kind": rendered.prompt_kind,
                        "node_config": input.node.config_json,
                    }),
                )
                .await?
                .ok_or_else(|| {
                    AppError::InvalidArgument(
                        "RenderTextNode 当前只支持 freeform 视频项目".to_string(),
                    )
                })?;
            lifecycle.generation_spec_id = Some(spec.id);
            let artifact = self
                .upsert_canvas_prompt_artifact_record(CanvasPromptArtifactUpsertInput {
                    generation_spec_id: spec.id,
                    prompt_kind: rendered.prompt_kind,
                    prompt_text: rendered.prompt_text,
                })
                .await?;
            self.finish_canvas_node_with_artifacts(
                &lifecycle,
                json!({
                    "data_type": "text",
                    "text": artifact.prompt_text,
                    "generation_spec_id": spec.id,
                    "prompt_artifact_id": artifact.id,
                    "prompt_kind": artifact.prompt_kind,
                }),
            )
            .await
        }
        .await;

        match result {
            Ok(summary) => Ok(summary),
            Err(error) => self.finish_canvas_node_error(lifecycle, error).await,
        }
    }

    pub async fn run_canvas_llm_text_node_validated(
        &self,
        input: CanvasTextNodeExecutionInput<'_>,
        llm_port: &dyn CanvasTextGenerationLlmPort,
    ) -> Result<CanvasRunSummary, AppError> {
        let checkpoints = self
            .canvas_text_source_checkpoints(input.shared_run.map(|run| run.workflow_run_id))
            .await?;
        let llm_input =
            build_canvas_llm_text_input(input.node, input.document, checkpoints.as_deref())
                .map_err(AppError::InvalidArgument)?;
        let step_key = crate::canvas::canvas_node_step_key(input.node.id)
            .map_err(AppError::InvalidArgument)?;
        let input_json = json!({
            "project_id": input.project_id,
            "graph_id": input.graph_id,
            "node_id": input.node.id,
            "output_kind": llm_input.output_kind,
            "config_id": llm_input.config_id,
            "temperature": llm_input.temperature,
            "max_tokens": llm_input.max_tokens,
            "canvas_text_asset_id": llm_input.canvas_text_asset_id,
            "source_node_ids": llm_input.source_node_ids,
            "messages": [
                { "role": "system", "content": llm_input.system_prompt },
                { "role": "user", "content": llm_input.user_prompt }
            ],
        });
        let mut lifecycle = self
            .start_canvas_node_execution(
                input.project_id,
                input.graph_id,
                input.mode,
                step_key,
                input_json,
                input.shared_run,
            )
            .await?;

        let result = async {
            let spec = self
                .upsert_canvas_text_generation_spec(
                    &input,
                    llm_input.canvas_text_asset_id,
                    WORKFLOW_STATUS_RUNNING,
                    json!({
                        "system_prompt": llm_input.system_prompt,
                        "user_prompt": llm_input.user_prompt,
                        "source_node_ids": llm_input.source_node_ids,
                    }),
                    json!({
                        "workflow": "llm_transform",
                        "output_kind": llm_input.output_kind,
                        "config_id": llm_input.config_id,
                        "node_config": input.node.config_json,
                    }),
                )
                .await?
                .ok_or_else(|| {
                    AppError::InvalidArgument(
                        "LlmTextNode 当前只支持 freeform 视频项目".to_string(),
                    )
                })?;
            lifecycle.generation_spec_id = Some(spec.id);
            let validated = self
                .execute_canvas_llm_text_call(
                    &input,
                    &llm_input,
                    lifecycle.run.workflow_run_id,
                    &lifecycle.step_key,
                    llm_port,
                )
                .await?;
            self.mark_canvas_generation_spec_status_record(spec.id, WORKFLOW_STATUS_COMPLETED)
                .await?;
            let artifact = self
                .upsert_canvas_prompt_artifact_record(CanvasPromptArtifactUpsertInput {
                    generation_spec_id: spec.id,
                    prompt_kind: llm_input.output_kind,
                    prompt_text: validated.transformed_text,
                })
                .await?;
            self.finish_canvas_node_with_artifacts(
                &lifecycle,
                json!({
                    "data_type": "text",
                    "text": artifact.prompt_text,
                    "generation_spec_id": spec.id,
                    "prompt_artifact_id": artifact.id,
                    "prompt_kind": artifact.prompt_kind,
                    "llm_finish_reason": validated.finish_reason,
                    "llm_usage": validated.usage,
                }),
            )
            .await
        }
        .await;

        match result {
            Ok(summary) => Ok(summary),
            Err(error) => self.finish_canvas_node_error(lifecycle, error).await,
        }
    }

    async fn canvas_text_source_checkpoints(
        &self,
        workflow_run_id: Option<i64>,
    ) -> Result<Option<Vec<a4n_db_models::models::generation::workflow_checkpoint::Model>>, AppError>
    {
        match workflow_run_id {
            Some(workflow_run_id) => self
                .workflow_checkpoint_models(workflow_run_id)
                .await
                .map(Some),
            None => Ok(None),
        }
    }

    async fn upsert_canvas_text_generation_spec(
        &self,
        input: &CanvasTextNodeExecutionInput<'_>,
        canvas_text_asset_id: Option<i64>,
        status: &str,
        source_material_json: Value,
        params_json: Value,
    ) -> Result<Option<a4n_db_models::models::generation::generation_spec::Model>, AppError> {
        let project = self.project_by_id(input.project_id).await?.ok_or_else(|| {
            AppError::InvalidArgument(format!("项目不存在: {}", input.project_id))
        })?;
        if project
            .source_mode()
            .map_err(|error| AppError::InvalidArgument(format!("项目 {}: {error}", project.id)))?
            != ProjectSourceMode::Freeform
        {
            return Ok(None);
        }
        self.upsert_freeform_canvas_generation_spec_record(
            FreeformCanvasGenerationSpecUpsertInput {
                project_id: input.project_id,
                target_entity_id: input.node.id,
                canvas_text_asset_id,
                target_kind: GENERATION_SPEC_TARGET_KIND_CANVAS_PROMPT.to_string(),
                target_entity_kind: GENERATION_SPEC_TARGET_ENTITY_KIND_CANVAS_NODE.to_string(),
                status: status.to_string(),
                params_json: json!({
                    "canvas_graph_id": input.graph_id,
                    "canvas_node_id": input.node.id,
                    "source_material": source_material_json,
                    "params": params_json,
                }),
            },
        )
        .await
        .map(Some)
    }

    async fn execute_canvas_llm_text_call(
        &self,
        input: &CanvasTextNodeExecutionInput<'_>,
        llm_input: &CanvasLlmTextInput,
        workflow_run_id: i64,
        step_key: &str,
        llm_port: &dyn CanvasTextGenerationLlmPort,
    ) -> Result<ValidatedCanvasLlmText, AppError> {
        let mut request =
            build_canvas_llm_text_request(input, llm_input, Some(workflow_run_id), step_key)
                .map_err(AppError::InvalidArgument)?;
        let call_key = format!("{step_key}:llm_transform");
        let call_order = i32::try_from(input.node.id).map_err(|_| {
            AppError::InvalidArgument(format!(
                "画布节点 ID 超出调用级 checkpoint 顺序范围: {}",
                input.node.id
            ))
        })?;
        let request_json = context_aware_chat_request_trace_json(&request).map_err(|error| {
            AppError::InvalidArgument(format!("LlmTextNode 请求序列化失败: {error}"))
        })?;
        let input_fingerprint_json = json!({
            "workflow": WORKFLOW_KIND_VIDEO_CANVAS_GRAPH,
            "call_kind": "canvas_llm_transform",
            "project_id": input.project_id,
            "graph_id": input.graph_id,
            "node_id": input.node.id,
            "mode": input.mode,
            "request": request_json,
        });
        let input_fingerprint_sha256 =
            workflow::llm_call_checkpoint_fingerprint_sha256(&input_fingerprint_json)
                .map_err(workflow_checkpoint_contract_app_error)?;
        if let Some(trace_context) = request.trace_context.as_mut() {
            trace_context.call_key = Some(call_key.clone());
            trace_context.input_fingerprint_sha256 = Some(input_fingerprint_sha256);
        }

        let agent = self.agent();
        let task = AgentOwnedTask::Workflow(workflow_run_id);
        let model_execution = if let Some(owner) = agent.workflow_owner(workflow_run_id).await? {
            let key = task.execution_key(true, &call_key);
            if let Some(execution) = agent.execution_by_key(owner.id, &key).await? {
                let original = ChatRequest::from_snapshot_value(
                    execution.arguments_json.clone().ok_or_else(|| {
                        AppError::InvalidArgument("Canvas 模型缺少持久请求".into())
                    })?,
                )?;
                if original
                    .trace_context
                    .as_ref()
                    .and_then(|t| t.input_fingerprint_sha256.as_ref())
                    != request
                        .trace_context
                        .as_ref()
                        .and_then(|t| t.input_fingerprint_sha256.as_ref())
                {
                    return Err(AppError::InvalidArgument(
                        "Canvas 模型恢复请求与原输入不一致".into(),
                    ));
                }
                request = original;
                Some(execution)
            } else {
                agent
                    .bound_model_request(
                        owner.id,
                        &mut request,
                        llm_port.max_output_tokens(llm_input.config_id).await?,
                    )
                    .await?;
                agent
                    .prepare_task_execution(AgentTaskExecutionInput {
                        task: task.clone(),
                        execution_key: call_key.clone(),
                        tool_name: None,
                        arguments: request.snapshot_value()?,
                        input_message_id: None,
                        base_input_fingerprint: None,
                    })
                    .await?
            }
        } else {
            None
        };

        if let Some(checkpoint) = self
            .load_completed_llm_call_checkpoint(workflow_run_id, &call_key, &input_fingerprint_json)
            .await?
        {
            if let Some(execution) = &model_execution {
                let response = ChatResponse {
                    message: ChatMessage::assistant(checkpoint.response_content.clone()),
                    usage: checkpoint
                        .usage
                        .clone()
                        .map(serde_json::from_value)
                        .transpose()?,
                    finish_reason: checkpoint.finish_reason.clone(),
                    reasoning_content: None,
                };
                if execution.status != "succeeded"
                    || execution.result_json.as_ref() != Some(&serde_json::to_value(response)?)
                {
                    return Err(AppError::InvalidArgument(
                        "Canvas checkpoint 与 Agent 模型回执不一致".into(),
                    ));
                }
            }
            return validate_restored_canvas_llm_text_checkpoint(
                &checkpoint.parsed_output,
                &checkpoint.response_content,
                checkpoint.usage,
                checkpoint.finish_reason,
                &call_key,
            );
        }

        let restored = model_execution
            .as_ref()
            .filter(|e| e.status == "succeeded")
            .map(|e| {
                serde_json::from_value::<ChatResponse>(
                    e.result_json
                        .clone()
                        .ok_or_else(|| AppError::InvalidArgument("Canvas 模型缺少回执".into()))?,
                )
                .map_err(AppError::from)
            })
            .transpose()?;
        let response = if let Some(response) = restored {
            response
        } else {
            if let Some(execution) = &model_execution {
                if execution.status != "prepared" {
                    return Err(AppError::InvalidArgument(
                        "Canvas 模型上次调用结果不明，禁止重复派发".into(),
                    ));
                }
                agent
                    .record_task_execution(&task, execution.id, None, None, None)
                    .await?;
            }
            let response = llm_port
                .stream_complete(CanvasTextGenerationLlmCall {
                    project_id: input.project_id,
                    workflow_run_id,
                    config_id: llm_input.config_id,
                    request,
                })
                .await;
            if let Some(execution) = &model_execution {
                agent
                    .record_task_execution(
                        &task,
                        execution.id,
                        response
                            .as_ref()
                            .ok()
                            .map(serde_json::to_value)
                            .transpose()?,
                        None,
                        response.as_ref().err().map(ToString::to_string),
                    )
                    .await?;
            }
            response.map_err(map_canvas_llm_error)?
        };
        if let Some(execution) = &model_execution
            && response.usage.is_none()
            && agent
                .run_budget_snapshot(execution.run_id)
                .await?
                .remaining
                .max_tokens
                .is_some()
        {
            return Err(AppError::InvalidArgument(
                "Canvas 模型缺少 usage，无法核实 Agent token 预算".into(),
            ));
        }
        if is_truncated_finish_reason(response.finish_reason.as_deref()) {
            return Err(AppError::InvalidArgument(
                "LlmTextNode 输出达到当前 max_tokens 上限，文本可能被截断；请提高模型配置 max_output_tokens 或减少上游文本后重试"
                    .to_string(),
            ));
        }
        let transformed_text = response.message.content.trim().to_string();
        if transformed_text.is_empty() {
            return Err(AppError::InvalidArgument(
                "LlmTextNode 返回空文本".to_string(),
            ));
        }
        let usage_json = response
            .usage
            .as_ref()
            .map(serde_json::to_value)
            .transpose()?;
        self.save_completed_llm_call_checkpoint(CompletedLlmCallCheckpointInput {
            workflow_run_id,
            call_key,
            call_order,
            input_fingerprint_json,
            checkpoint: CompletedLlmCallCheckpoint {
                parsed_output: json!({"text": transformed_text}),
                response_content: response.message.content,
                usage: usage_json,
                finish_reason: response.finish_reason.clone(),
                artifacts_json: None,
            },
        })
        .await?;
        Ok(ValidatedCanvasLlmText {
            transformed_text,
            usage: response.usage,
            finish_reason: response.finish_reason,
        })
    }
}

pub(super) fn map_canvas_llm_error(error: AppError) -> AppError {
    match error {
        AppError::Cancelled => AppError::Cancelled,
        error => AppError::LlmError(format!("LlmTextNode 调用 LLM 失败: {error}")),
    }
}

fn validate_restored_canvas_llm_text_checkpoint(
    parsed_output: &Value,
    response_content: &str,
    usage: Option<Value>,
    finish_reason: Option<String>,
    call_key: &str,
) -> Result<ValidatedCanvasLlmText, AppError> {
    let parsed_text = parsed_output
        .get("text")
        .and_then(Value::as_str)
        .ok_or_else(|| {
            AppError::InvalidArgument(format!("{call_key}.parsed_output.text 必须是字符串"))
        })?;
    let transformed_text = response_content.trim().to_string();
    ensure_checkpoint_parsed_output_matches(
        parsed_text,
        transformed_text.as_str(),
        &format!("画布文本生成 {call_key}"),
    )?;
    let usage = usage
        .map(serde_json::from_value)
        .transpose()
        .map_err(|error| {
            AppError::InvalidArgument(format!("{call_key}.usage 反序列化失败: {error}"))
        })?;
    Ok(ValidatedCanvasLlmText {
        transformed_text,
        usage,
        finish_reason,
    })
}

#[cfg(test)]
pub(super) fn validate_restored_checkpoint_for_test(
    parsed_output: Value,
    response_content: &str,
    call_key: &str,
) -> Result<String, AppError> {
    validate_restored_canvas_llm_text_checkpoint(
        &parsed_output,
        response_content,
        None,
        None,
        call_key,
    )
    .map(|validated| validated.transformed_text)
}
