mod call;

use serde_json::{Value, json};

use a4n_error::AppError;
use a4n_llm_provider_contract::provider::ChatRequest;

use crate::{
    structured_llm::{
        StructuredLlmProgress, StructuredLlmStreamResult, ensure_checkpoint_parsed_output_matches,
    },
    workflow::{
        CompletedLlmCallCheckpoint, CompletedLlmCallCheckpointInput,
        llm_call_checkpoint_fingerprint_sha256, workflow_checkpoint_contract_app_error,
    },
};

use self::call::chat_scene_profile_stage;
use super::super::parsing::parse_scene_stage_response;
use super::super::progress::report_scene_profile_sync_progress;
use super::SceneProfileStageContext;
use super::request::scene_profile_stage_fingerprint;

pub(super) async fn run_scene_profile_stage_with_checkpoint<T>(
    context: &mut SceneProfileStageContext<'_, '_>,
    request: &mut ChatRequest,
    prompt_name: &str,
    step_key: String,
    step_order: i32,
    stage_progress: StructuredLlmProgress,
    parse: impl FnOnce(&Value) -> Result<T, String>,
) -> Result<(Vec<Value>, T), AppError> {
    let root_field = stage_progress.spec.root_array_key;
    if let Some(trace_context) = request.trace_context.as_mut() {
        trace_context.call_key = Some(step_key.clone());
    }
    let fingerprint = scene_profile_stage_fingerprint(&step_key, prompt_name, root_field, request)?;
    let fingerprint_sha = llm_call_checkpoint_fingerprint_sha256(&fingerprint)
        .map_err(workflow_checkpoint_contract_app_error)?;
    if let Some(trace_context) = request.trace_context.as_mut() {
        trace_context.input_fingerprint_sha256 = Some(fingerprint_sha);
    }
    let (response, stored_output) = match context
        .state
        .load_completed_llm_call_checkpoint(context.workflow_run_id, &step_key, &fingerprint)
        .await?
    {
        Some(checkpoint) => (
            StructuredLlmStreamResult {
                response_content: checkpoint.response_content,
                usage: checkpoint.usage.map(serde_json::from_value).transpose()?,
                finish_reason: checkpoint.finish_reason,
            },
            Some(checkpoint.parsed_output),
        ),
        None => (
            chat_scene_profile_stage(
                context.state,
                request.clone(),
                prompt_name,
                stage_progress,
                context.progress,
            )
            .await?,
            None,
        ),
    };
    let (output, parsed) =
        parse_scene_stage_response(&response.response_content, prompt_name, root_field, parse)
            .map_err(|error| {
                AppError::LlmError(format!("{prompt_name} 输出不符合合同：{error}"))
            })?;
    let parsed_output = json!({root_field: &output});
    let restored = stored_output.is_some();
    let response_length = response.response_content.chars().count();
    if let Some(stored_output) = stored_output {
        ensure_checkpoint_parsed_output_matches(
            &stored_output,
            &parsed_output,
            &format!("场景档案同步 {step_key}"),
        )?;
    } else {
        context
            .state
            .save_completed_llm_call_checkpoint(CompletedLlmCallCheckpointInput {
                workflow_run_id: context.workflow_run_id,
                call_key: step_key,
                call_order: step_order,
                input_fingerprint_json: fingerprint,
                checkpoint: CompletedLlmCallCheckpoint {
                    parsed_output,
                    response_content: response.response_content,
                    usage: response.usage.map(serde_json::to_value).transpose()?,
                    finish_reason: response.finish_reason,
                    artifacts_json: None,
                },
            })
            .await?;
    }
    context.response_chars.push(json!({
        "prompt_name": prompt_name,
        "response_chars": response_length,
        "source": if restored { "llm_call_checkpoint" } else { "llm" },
    }));
    let item_count = output.len();
    let source = if restored {
        "已从断点恢复"
    } else {
        "已完成"
    };
    report_scene_profile_sync_progress(
        context.progress,
        stage_progress.span.end(),
        format!("{prompt_name} {source}：校验通过 {item_count} 条{root_field}"),
    )?;
    Ok((output, parsed))
}
