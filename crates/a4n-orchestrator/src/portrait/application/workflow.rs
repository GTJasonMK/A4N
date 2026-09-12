use a4n_api_contract::portrait::PortraitSummary;
use a4n_error::AppError;
use a4n_llm::provider::{ChatRequest, is_truncated_finish_reason};
use serde_json::{Value, json};
use tokio_util::sync::CancellationToken;

use crate::{
    OrchestratorContext,
    portrait::{normalize_portrait_prompt_response, portrait_prompt_checkpoint_fingerprint},
    structured_llm::ensure_checkpoint_parsed_output_matches,
    workflow::{
        CompletedLlmCallCheckpoint, CompletedLlmCallCheckpointInput,
        CompletedWorkflowCheckpointInput, required_checkpoint_field, required_checkpoint_json,
    },
};

use super::{
    contracts::{PortraitGenerationHostPort, PortraitProgress},
    image::portrait_summary,
};

pub(super) struct InitialPortraitPrompt {
    pub(super) request: ChatRequest,
    pub(super) image_prompt: String,
    pub(super) raw_response_json: Value,
}

pub(super) fn report_progress(
    progress: &mut Option<PortraitProgress<'_>>,
    current: u32,
    total: u32,
    message: &str,
) -> Result<(), AppError> {
    if let Some(progress) = progress.as_deref_mut() {
        progress(current, total, message)?;
    }
    Ok(())
}

pub(super) fn require_not_cancelled(cancel_token: &CancellationToken) -> Result<(), AppError> {
    if cancel_token.is_cancelled() {
        return Err(AppError::Cancelled);
    }
    Ok(())
}

pub(super) async fn resolve_initial_portrait_prompt(
    context: &OrchestratorContext,
    workflow_run_id: i64,
    workflow_fingerprint: &Value,
    mut request: ChatRequest,
    progress: &mut Option<PortraitProgress<'_>>,
    cancel_token: &CancellationToken,
) -> Result<InitialPortraitPrompt, AppError> {
    report_progress(progress, 1, 6, "正在渲染人物三视图拆解提示词模板")?;
    require_not_cancelled(cancel_token)?;
    report_progress(progress, 2, 6, "正在调用模型生成图片提示词")?;
    let call_key = "portrait_generation.prompt".to_string();
    let prompt_fingerprint = portrait_prompt_checkpoint_fingerprint(workflow_fingerprint, &request)
        .map_err(AppError::InvalidArgument)?;
    crate::workflow::attach_llm_call_checkpoint_trace(
        &mut request,
        workflow_run_id,
        &call_key,
        &prompt_fingerprint,
    )?;
    context
        .mark_workflow_current_step(workflow_run_id, &call_key)
        .await?;
    if let Some(checkpoint) = context
        .load_completed_llm_call_checkpoint(workflow_run_id, &call_key, &prompt_fingerprint)
        .await?
    {
        report_progress(progress, 2, 6, "图片提示词已从断点恢复")?;
        let image_prompt = checkpoint
            .parsed_output
            .as_str()
            .ok_or_else(|| checkpoint_missing_field(&call_key, "parsed_output"))?;
        let reparsed = normalize_portrait_prompt_response(&checkpoint.response_content)?;
        ensure_checkpoint_parsed_output_matches(
            image_prompt,
            &reparsed,
            &format!("人物三视图提示词 {call_key}"),
        )?;
        return Ok(InitialPortraitPrompt {
            request,
            image_prompt: reparsed,
            raw_response_json: json!({
                "text": checkpoint.response_content,
                "usage": checkpoint.usage,
                "finish_reason": checkpoint.finish_reason,
            }),
        });
    }

    let response =
        crate::portrait::llm_stream::execute_portrait_llm_call(context, request.clone(), &call_key)
            .await?;
    if is_truncated_finish_reason(response.finish_reason.as_deref()) {
        return Err(AppError::InvalidArgument(
            "人物三视图提示词输出达到当前 max_tokens 上限，提示词可能被截断；请在模型配置中提高 max_output_tokens，或减少角色上下文后重试".to_string(),
        ));
    }
    let response_content = response.message.content.clone();
    let image_prompt = normalize_portrait_prompt_response(&response_content)?;
    let finish_reason = response.finish_reason.clone();
    let usage = response
        .usage
        .as_ref()
        .map(serde_json::to_value)
        .transpose()?;
    let raw_response_json = json!({
        "text": response_content.clone(),
        "usage": usage.clone(),
        "finish_reason": finish_reason,
    });
    context
        .save_completed_llm_call_checkpoint(CompletedLlmCallCheckpointInput {
            workflow_run_id,
            call_key,
            call_order: 1,
            input_fingerprint_json: prompt_fingerprint,
            checkpoint: CompletedLlmCallCheckpoint {
                parsed_output: Value::String(image_prompt.clone()),
                response_content,
                usage,
                finish_reason,
                artifacts_json: None,
            },
        })
        .await?;
    Ok(InitialPortraitPrompt {
        request,
        image_prompt,
        raw_response_json,
    })
}

pub(super) async fn load_completed_portrait_checkpoint(
    context: &OrchestratorContext,
    host_port: &dyn PortraitGenerationHostPort,
    workflow_run_id: i64,
    step_key: &str,
    fingerprint: &Value,
    project_id: i64,
) -> Result<Option<PortraitSummary>, AppError> {
    let Some(checkpoint) = context
        .load_completed_workflow_checkpoint(Some(workflow_run_id), step_key, fingerprint)
        .await?
    else {
        return Ok(None);
    };
    let checkpoint_json = required_checkpoint_json(&checkpoint, step_key).map_err(|error| {
        AppError::InvalidState {
            current: error.to_string(),
            operation: format!("恢复立绘最终断点 {step_key}"),
        }
    })?;
    let portrait_id = required_checkpoint_field(&checkpoint_json, step_key, "portrait_id")
        .map_err(|error| AppError::InvalidState {
            current: error.to_string(),
            operation: format!("恢复立绘最终断点 {step_key}"),
        })?
        .as_i64()
        .ok_or_else(|| checkpoint_missing_field(step_key, "portrait_id"))?;
    let image_path = checkpoint_string(&checkpoint_json, step_key, "image_path")?;
    let portrait = context
        .character_portrait_by_id(portrait_id)
        .await?
        .ok_or_else(|| AppError::NotFound("立绘不存在".to_string()))?;
    if portrait.project_id != project_id {
        return Err(AppError::InvalidState {
            current: format!("portrait_project_id={}", portrait.project_id),
            operation: format!("恢复项目 #{project_id} 的立绘断点 {step_key}"),
        });
    }
    if portrait.image_path != image_path {
        return Err(AppError::InvalidState {
            current: format!(
                "checkpoint_image_path={image_path},db_image_path={}",
                portrait.image_path
            ),
            operation: format!("恢复立绘最终断点 {step_key}"),
        });
    }
    host_port.validate_portrait_file(&portrait.image_path, "断点立绘图片")?;
    portrait_summary(&portrait).map(Some)
}

pub(super) async fn save_completed_portrait_checkpoint(
    context: &OrchestratorContext,
    workflow_run_id: i64,
    step_key: String,
    step_order: i32,
    fingerprint: Value,
    summary: &PortraitSummary,
) -> Result<(), AppError> {
    context
        .save_completed_workflow_checkpoint(CompletedWorkflowCheckpointInput {
            workflow_run_id: Some(workflow_run_id),
            step_key,
            step_order,
            input_fingerprint_json: fingerprint,
            checkpoint_json: Some(json!({
                "portrait_id": summary.id,
                "image_path": summary.image_path,
                "summary": summary,
            })),
        })
        .await
}

fn checkpoint_string(
    checkpoint_json: &Value,
    step_key: &str,
    field: &str,
) -> Result<String, AppError> {
    required_checkpoint_field(checkpoint_json, step_key, field)
        .map_err(|error| AppError::InvalidState {
            current: error.to_string(),
            operation: format!("读取立绘断点 {step_key}"),
        })?
        .as_str()
        .map(ToString::to_string)
        .ok_or_else(|| checkpoint_missing_field(step_key, field))
}

fn checkpoint_missing_field(step_key: &str, field: &str) -> AppError {
    AppError::InvalidState {
        current: format!("missing_or_invalid_checkpoint_field:{field}"),
        operation: format!("读取立绘断点 {step_key}"),
    }
}
