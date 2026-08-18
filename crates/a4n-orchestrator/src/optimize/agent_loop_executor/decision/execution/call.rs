use a4n_llm::provider::{ChatMessage, ChatRequest, ChatResponse};
use serde_json::Value;

use super::super::super::{
    AgentLoopDecision, AgentLoopDecisionRequest, AgentLoopExecutionError, AppError,
    EVENT_KIND_ERROR, EVENT_STATUS_FAILED, OrchestratorContext, json,
    parse_agent_loop_decision_response,
};
use super::super::checkpoint::restored_agent_loop_decision;
use super::contracts::AgentDecisionCallResult;
use crate::generation::checkpoint::GenerationCheckpointMetadata;
use crate::generation::generation_public_error;
use crate::generation::stream::{
    LlmGenerationCheckpoint, LlmGenerationRequest, execute_llm_generation, generation_usage,
    provider_usage, publish_generation_usage,
};
use a4n_service_protocol::GenerationFinishReason;

pub(super) async fn execute_agent_loop_decision_call(
    context: &OrchestratorContext,
    turn_id: i64,
    session_id: i64,
    request: &AgentLoopDecisionRequest<'_>,
    chat_request: ChatRequest,
    call_key: &str,
    input_fingerprint_json: &Value,
) -> Result<AgentDecisionCallResult, AgentLoopExecutionError> {
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
        let generation = request.generation;
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

    let generation = request.generation;
    let usage_base = generation.session.snapshot().usage.clone();
    let output = execute_llm_generation(
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
    .await?;
    let response = ChatResponse {
        message: ChatMessage::assistant(output.content),
        usage: output.usage.as_ref().map(provider_usage).transpose()?,
        finish_reason: Some(match output.finish_reason {
            GenerationFinishReason::Stop => "stop".to_string(),
            GenerationFinishReason::ToolCallsComplete => "tool_calls".to_string(),
        }),
    };
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
