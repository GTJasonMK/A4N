use a4n_llm::provider::{ChatMessage, ChatRequest, ChatResponse};
use serde_json::Value;

use super::super::super::{
    AgentLoopDecision, AgentLoopDecisionRequest, AgentLoopExecutionError, AppError,
    EVENT_KIND_ERROR, EVENT_STATUS_FAILED, OrchestratorContext, json,
    parse_agent_loop_decision_response,
};
use super::super::checkpoint::restored_agent_loop_decision;
use super::contracts::AgentDecisionCallResult;
use crate::generation::{
    checkpoint::GenerationCheckpointMetadata,
    generation_finish_reason_text, generation_public_error,
    stream::{
        LlmGenerationCheckpoint, LlmGenerationRequest, execute_llm_generation, generation_usage,
        provider_usage, publish_generation_usage,
    },
};
use crate::optimize::runtime_tools::accounting::record_model_execution;

pub(super) async fn execute_agent_loop_decision_call(
    context: &OrchestratorContext,
    turn_id: i64,
    session_id: i64,
    request: &AgentLoopDecisionRequest<'_>,
    chat_request: ChatRequest,
    call_key: &str,
    input_fingerprint_json: &Value,
    execution: Option<a4n_db_models::models::agent::execution::Model>,
) -> Result<AgentDecisionCallResult, AgentLoopExecutionError> {
    let generation = request.generation;
    if let Some(checkpoint) = context
        .load_completed_llm_call_checkpoint(
            request.workflow_run_id,
            call_key,
            input_fingerprint_json,
        )
        .await?
    {
        let (response, decision) = restored_agent_loop_decision(
            checkpoint.parsed_output,
            checkpoint.response_content,
            checkpoint.usage,
            checkpoint.finish_reason,
            call_key,
        )?;
        if let Some(execution) = &execution
            && (execution.status != "succeeded"
                || execution.result_json.as_ref()
                    != Some(&serde_json::to_value(&response).map_err(AppError::from)?))
        {
            return Err(AppError::InvalidArgument(
                "内容优化 checkpoint 与 Agent 模型计费回执不一致".into(),
            )
            .into());
        }
        let usage_base = generation.session.snapshot().usage.clone();
        generation
            .publisher
            .text_delta("raw_json", &response.message.content)
            .await
            .map_err(|error| {
                AgentLoopExecutionError::Generation(generation_public_error(
                    error.code(),
                    "checkpoint",
                    error,
                ))
            })?;
        if let Some(usage_info) = response.usage.as_ref() {
            let usage = generation_usage(usage_info.clone())?;
            publish_generation_usage(generation.publisher, &usage, usage_base.as_ref()).await?;
        }
        context
            .save_agent_event(
                session_id,
                &format!(
                    "已恢复 {} 第 {} 步 LLM 调用断点。",
                    request.phase, request.step
                ),
            )
            .await?;
        return Ok(AgentDecisionCallResult {
            response,
            decision,
            restored: true,
        });
    }

    if let Some(execution) = &execution {
        if execution.status == "succeeded" {
            let response: ChatResponse = serde_json::from_value(
                execution
                    .result_json
                    .clone()
                    .ok_or_else(|| AppError::InvalidArgument("内容优化模型缺少持久回执".into()))?,
            )
            .map_err(AppError::from)?;
            let decision = parse_agent_loop_decision(
                context,
                turn_id,
                session_id,
                request,
                &response.message.content,
            )
            .await?;
            generation
                .publisher
                .text_delta("raw_json", &response.message.content)
                .await
                .map_err(|error| {
                    AgentLoopExecutionError::Generation(generation_public_error(
                        error.code(),
                        "checkpoint",
                        error,
                    ))
                })?;
            if let Some(usage) = response.usage.clone() {
                publish_generation_usage(
                    generation.publisher,
                    &generation_usage(usage)?,
                    generation.session.snapshot().usage.as_ref(),
                )
                .await?;
            }
            return Ok(AgentDecisionCallResult {
                response,
                decision,
                restored: false,
            });
        }
        if execution.status != "prepared" {
            return Err(AppError::InvalidArgument(
                "上次内容优化模型调用结果不明，禁止重复消耗预算".into(),
            )
            .into());
        }
    }
    record_model_execution(context, generation, execution.as_ref(), None, None).await?;
    let usage_base = generation.session.snapshot().usage.clone();
    let output = match execute_llm_generation(
        context.llm_backend(),
        generation.publisher,
        LlmGenerationRequest {
            request: chat_request,
            stream_id: "raw_json".to_string(),
            cancellation: generation.cancellation.clone(),
            checkpoint: Some(LlmGenerationCheckpoint {
                writer: generation.checkpoint_writer,
                session: generation.session,
                metadata: GenerationCheckpointMetadata {
                    step_key: Some(request.workflow.to_string()),
                    call_key: Some(call_key.to_string()),
                    provider_model: Some(context.llm_model_name()),
                    ..Default::default()
                },
            }),
            usage_base,
        },
    )
    .await
    {
        Ok(output) => output,
        Err(error) => {
            record_model_execution(
                context,
                generation,
                execution.as_ref(),
                None,
                Some(format!("{}: {}", error.code, error.message)),
            )
            .await?;
            return Err(error.into());
        }
    };
    let response = ChatResponse {
        message: ChatMessage::assistant(output.content),
        usage: output.usage.as_ref().map(provider_usage).transpose()?,
        finish_reason: Some(generation_finish_reason_text(output.finish_reason)),
    };
    let missing_usage = if let Some(execution) = &execution {
        response.usage.is_none()
            && context
                .agent()
                .run_budget_snapshot(execution.run_id)
                .await?
                .remaining
                .max_tokens
                .is_some()
    } else {
        false
    };
    record_model_execution(
        context,
        generation,
        execution.as_ref(),
        Some(serde_json::to_value(&response).map_err(AppError::from)?),
        missing_usage.then(|| "模型缺少 usage，无法核实 Agent token 预算".into()),
    )
    .await?;
    if missing_usage {
        return Err(
            AppError::InvalidArgument("模型缺少 usage，无法核实 Agent token 预算".into()).into(),
        );
    }
    let decision = parse_agent_loop_decision(
        context,
        turn_id,
        session_id,
        request,
        &response.message.content,
    )
    .await?;
    Ok(AgentDecisionCallResult {
        response,
        decision,
        restored: false,
    })
}

async fn parse_agent_loop_decision(
    context: &OrchestratorContext,
    turn_id: i64,
    session_id: i64,
    request: &AgentLoopDecisionRequest<'_>,
    raw_response: &str,
) -> Result<AgentLoopDecision, AppError> {
    let decision = parse_agent_loop_decision_response(raw_response);
    match decision {
        Ok(decision) => Ok(decision),
        Err(err) => {
            context
                .append_turn_event(
                    turn_id,
                    EVENT_KIND_ERROR,
                    EVENT_STATUS_FAILED,
                    &format!("模型决策解析失败：{err}"),
                    Some(json!({
                        "step": request.step,
                        "phase": request.phase,
                    })),
                )
                .await?;
            context
                .save_agent_event(
                    session_id,
                    &format!("{} 决策解析失败：{err}", request.parse_error_label),
                )
                .await?;
            Err(err)
        }
    }
}
