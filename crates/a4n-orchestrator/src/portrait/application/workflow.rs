use a4n_core::error::AppError;
use a4n_llm::provider::{ChatRequest, is_truncated_finish_reason};
use serde_json::{Value, json};
use tokio_util::sync::CancellationToken;

use crate::{
    OrchestratorContext,
    portrait::{normalize_portrait_prompt_response, portrait_prompt_checkpoint_fingerprint},
    workflow::{
        CompletedLlmCallCheckpointInput, CompletedWorkflowCheckpointInput,
        required_checkpoint_field, required_checkpoint_json, required_llm_call_checkpoint_json,
    },
};

use super::{
    contracts::{PortraitGenerationHostPort, PortraitProgress, PortraitSummary},
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

pub(super) async fn finalize_portrait_application<T>(
    context: &OrchestratorContext,
    workflow_run_id: i64,
    result: &Result<T, AppError>,
    success_message: &str,
) -> Result<(), AppError> {
    let (outcome, error_message) = match result {
        Ok(_) => (crate::workflow::WorkflowRunFinalizeOutcome::Completed, None),
        Err(AppError::Cancelled) => (
            crate::workflow::WorkflowRunFinalizeOutcome::Cancelled,
            Some(AppError::Cancelled.to_string()),
        ),
        Err(error) => (
            crate::workflow::WorkflowRunFinalizeOutcome::Failed,
            Some(error.to_string()),
        ),
    };
    context
        .finalize_workflow_run_outcome_validated(workflow_run_id, outcome, error_message.as_deref())
        .await
        .map_err(|status_error| {
            if matches!(&status_error, AppError::Cancelled) {
                return AppError::Cancelled;
            }
            match result {
                Ok(_) => AppError::InvalidState {
                    current: "portrait_application_completed".to_string(),
                    operation: format!("{success_message}，但工作流状态写入失败: {status_error}"),
                },
                Err(error) => AppError::InvalidState {
                    current: error.to_string(),
                    operation: format!("工作流失败状态写入失败: {status_error}"),
                },
            }
        })?;
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
        let checkpoint_json =
            required_llm_call_checkpoint_json(checkpoint, &call_key).map_err(|error| {
                AppError::InvalidState {
                    current: error.to_string(),
                    operation: format!("恢复人物三视图提示词断点 {call_key}"),
                }
            })?;
        let response_content = checkpoint_string(&checkpoint_json, &call_key, "response_content")?;
        let image_prompt = checkpoint_string(&checkpoint_json, &call_key, "parsed_output")?;
        let reparsed = normalize_portrait_prompt_response(&response_content)?;
        if reparsed != image_prompt {
            return Err(AppError::InvalidState {
                current: "checkpoint_parsed_output_mismatch".to_string(),
                operation: format!("恢复人物三视图提示词断点 {call_key}"),
            });
        }
        let usage = checkpoint_json
            .get("usage")
            .cloned()
            .ok_or_else(|| checkpoint_missing_field(&call_key, "usage"))?;
        let finish_reason = checkpoint_json
            .get("finish_reason")
            .cloned()
            .ok_or_else(|| checkpoint_missing_field(&call_key, "finish_reason"))?;
        return Ok(InitialPortraitPrompt {
            request,
            image_prompt,
            raw_response_json: json!({
                "text": response_content,
                "usage": usage,
                "finish_reason": finish_reason,
            }),
        });
    }

    let response =
        crate::portrait::llm_stream::execute_portrait_llm_call(context, request.clone(), &call_key)
            .await?;
    if is_truncated_finish_reason(response.finish_reason.as_deref()) {
        return Err(AppError::InvalidArgument(
            "人物三视图提示词输出达到当前 max_tokens 上限，提示词可能被截断；请在设置页调大“媒体提示词”输出档位，或减少角色上下文后重试".to_string(),
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
            checkpoint_json: json!({
                "output": image_prompt.clone(),
                "parsed_output": image_prompt.clone(),
                "response_content": response_content,
                "usage": usage,
                "finish_reason": finish_reason,
            }),
            artifacts_json: None,
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
    Ok(Some(portrait_summary(&portrait)))
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
            checkpoint_json: json!({
                "portrait_id": summary.id,
                "image_path": summary.image_path,
                "summary": summary,
            }),
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
