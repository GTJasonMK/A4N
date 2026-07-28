use a4n_core::{error::AppError, source_spans::source_text_hash};
use a4n_db::canvas::{CanvasPromptArtifactUpsertInput, FreeformCanvasGenerationSpecUpsertInput};
use a4n_llm::{
    provider::{UsageInfo, is_truncated_finish_reason},
    trace::context_aware_chat_request_trace_json,
};
use serde_json::{Value, json};

use crate::{
    OrchestratorContext,
    canvas::WORKFLOW_KIND_VIDEO_CANVAS_GRAPH,
    project::SOURCE_MODE_FREEFORM,
    workflow::{
        self, CompletedLlmCallCheckpointInput, WORKFLOW_STATUS_COMPLETED, WORKFLOW_STATUS_RUNNING,
        workflow_checkpoint_contract_app_error,
    },
};

use super::{
    contracts::{
        CanvasLlmTextInput, CanvasTextGenerationLlmCall, CanvasTextGenerationLlmPort,
        CanvasTextNodeExecutionInput, CanvasTextNodeRunSummary,
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
    pub async fn prepare_canvas_llm_text_input_validated(
        &self,
        workflow_run_id: Option<i64>,
        node: &a4n_core::models::canvas::canvas_node::Model,
        document: &a4n_db::canvas::CanvasGraphRecordDocument,
    ) -> Result<CanvasLlmTextInput, AppError> {
        let checkpoints = self.canvas_text_source_checkpoints(workflow_run_id).await?;
        build_canvas_llm_text_input(node, document, checkpoints.as_deref())
            .map_err(AppError::InvalidArgument)
    }

    pub async fn run_canvas_render_text_node_validated(
        &self,
        input: CanvasTextNodeExecutionInput<'_>,
    ) -> Result<CanvasTextNodeRunSummary, AppError> {
        let checkpoints = self
            .canvas_text_source_checkpoints(input.shared_run.map(|run| run.workflow_run_id))
            .await?;
        let rendered =
            build_canvas_rendered_prompt(input.node, input.document, checkpoints.as_deref())
                .map_err(AppError::InvalidArgument)?;
        let step_key = format!("canvas_node:{}", input.node.id);
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
            self.record_canvas_node_checkpoint(
                lifecycle.run.workflow_run_id,
                lifecycle.step_key.clone(),
                WORKFLOW_STATUS_COMPLETED,
                lifecycle.input_json.clone(),
                Some(json!({
                    "data_type": "text",
                    "text": artifact.prompt_text,
                    "generation_spec_id": spec.id,
                    "prompt_artifact_id": artifact.id,
                    "prompt_kind": artifact.prompt_kind,
                })),
                None,
                true,
            )
            .await?;
            self.finish_canvas_node_success(&lifecycle).await
        }
        .await;

        match result {
            Ok(summary) => Ok(summary),
            Err(error) => self.fail_canvas_node(lifecycle, error).await,
        }
    }

    pub async fn run_canvas_llm_text_node_validated(
        &self,
        input: CanvasTextNodeExecutionInput<'_>,
        llm_port: &dyn CanvasTextGenerationLlmPort,
    ) -> Result<CanvasTextNodeRunSummary, AppError> {
        let checkpoints = self
            .canvas_text_source_checkpoints(input.shared_run.map(|run| run.workflow_run_id))
            .await?;
        let llm_input =
            build_canvas_llm_text_input(input.node, input.document, checkpoints.as_deref())
                .map_err(AppError::InvalidArgument)?;
        let step_key = format!("canvas_node:{}", input.node.id);
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
            self.record_canvas_node_checkpoint(
                lifecycle.run.workflow_run_id,
                lifecycle.step_key.clone(),
                WORKFLOW_STATUS_COMPLETED,
                lifecycle.input_json.clone(),
                Some(json!({
                    "data_type": "text",
                    "text": artifact.prompt_text,
                    "generation_spec_id": spec.id,
                    "prompt_artifact_id": artifact.id,
                    "prompt_kind": artifact.prompt_kind,
                    "llm_finish_reason": validated.finish_reason,
                    "llm_usage": validated.usage,
                })),
                None,
                true,
            )
            .await?;
            self.finish_canvas_node_success(&lifecycle).await
        }
        .await;

        match result {
            Ok(summary) => Ok(summary),
            Err(error) => self.fail_canvas_node(lifecycle, error).await,
        }
    }

    async fn canvas_text_source_checkpoints(
        &self,
        workflow_run_id: Option<i64>,
    ) -> Result<Option<Vec<a4n_core::models::generation::workflow_checkpoint::Model>>, AppError>
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
    ) -> Result<Option<a4n_core::models::generation::generation_spec::Model>, AppError> {
        let project = self.project_by_id(input.project_id).await?.ok_or_else(|| {
            AppError::InvalidArgument(format!("项目不存在: {}", input.project_id))
        })?;
        if project.source_mode != SOURCE_MODE_FREEFORM {
            return Ok(None);
        }
        let source_version_hash = source_text_hash(
            &serde_json::to_string(&source_material_json)
                .map_err(|error| AppError::InvalidArgument(error.to_string()))?,
        );
        self.upsert_freeform_canvas_generation_spec_record(
            FreeformCanvasGenerationSpecUpsertInput {
                project_id: input.project_id,
                target_entity_id: input.node.id,
                canvas_text_asset_id,
                target_kind: "canvas_prompt".to_string(),
                target_entity_kind: "canvas_node".to_string(),
                status: status.to_string(),
                source_version_hash,
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
            build_canvas_llm_text_request(input, llm_input, workflow_run_id, step_key)
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

        if let Some(checkpoint) = self
            .load_completed_llm_call_checkpoint(workflow_run_id, &call_key, &input_fingerprint_json)
            .await?
        {
            let checkpoint_json =
                workflow::required_llm_call_checkpoint_json(checkpoint, &call_key)
                    .map_err(workflow_checkpoint_contract_app_error)?;
            return validate_restored_canvas_llm_text_checkpoint(checkpoint_json, &call_key);
        }

        let response = llm_port
            .stream_complete(CanvasTextGenerationLlmCall {
                project_id: input.project_id,
                workflow_run_id,
                config_id: llm_input.config_id,
                request,
            })
            .await
            .map_err(|error| AppError::LlmError(format!("LlmTextNode 调用 LLM 失败: {error}")))?;
        if is_truncated_finish_reason(response.finish_reason.as_deref()) {
            return Err(AppError::InvalidArgument(
                "LlmTextNode 输出达到当前 max_tokens 上限，文本可能被截断；请调大输出档位或减少上游文本后重试"
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
            checkpoint_json: json!({
                "output": {"text": transformed_text},
                "parsed_output": {"text": transformed_text},
                "response_content": response.message.content,
                "usage": usage_json,
                "finish_reason": response.finish_reason,
            }),
            artifacts_json: None,
        })
        .await?;
        Ok(ValidatedCanvasLlmText {
            transformed_text,
            usage: response.usage,
            finish_reason: response.finish_reason,
        })
    }
}

fn validate_restored_canvas_llm_text_checkpoint(
    checkpoint_json: Value,
    call_key: &str,
) -> Result<ValidatedCanvasLlmText, AppError> {
    let response_content = checkpoint_json
        .get("response_content")
        .and_then(Value::as_str)
        .ok_or_else(|| {
            AppError::InvalidArgument(format!("{call_key}.response_content 必须是字符串"))
        })?;
    let parsed_text = checkpoint_json
        .get("parsed_output")
        .and_then(|value| value.get("text"))
        .and_then(Value::as_str)
        .ok_or_else(|| {
            AppError::InvalidArgument(format!("{call_key}.parsed_output.text 必须是字符串"))
        })?;
    let finish_reason = match checkpoint_json.get("finish_reason") {
        Some(Value::Null) | None => None,
        Some(value) => Some(
            value
                .as_str()
                .ok_or_else(|| {
                    AppError::InvalidArgument(format!(
                        "{call_key}.finish_reason 必须是字符串或 null"
                    ))
                })?
                .to_string(),
        ),
    };
    if is_truncated_finish_reason(finish_reason.as_deref()) {
        return Err(AppError::InvalidArgument(format!(
            "{call_key} checkpoint 标记为截断响应，不能用于恢复"
        )));
    }
    let transformed_text = response_content.trim().to_string();
    if parsed_text != transformed_text {
        return Err(AppError::InvalidArgument(format!(
            "{call_key} checkpoint 的 parsed_output 与 response_content 重新解析结果不一致"
        )));
    }
    if transformed_text.is_empty() {
        return Err(AppError::InvalidArgument(
            "LlmTextNode 返回空文本".to_string(),
        ));
    }
    let usage = checkpoint_json
        .get("usage")
        .filter(|value| !value.is_null())
        .cloned()
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
    checkpoint_json: Value,
    call_key: &str,
) -> Result<String, AppError> {
    validate_restored_canvas_llm_text_checkpoint(checkpoint_json, call_key)
        .map(|validated| validated.transformed_text)
}
