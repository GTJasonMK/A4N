use a4n_llm_provider_contract::provider::{ChatMessage, ChatResponse};

use crate::{
    generation::{
        generation_finish_reason_text,
        stream::{
            LlmGenerationCheckpoint, LlmGenerationRequest, execute_llm_generation, provider_usage,
        },
    },
    optimize::{
        AgentLoopExecutionError, OrchestratorContext,
        turns::{EVENT_KIND_ERROR, EVENT_STATUS_FAILED},
    },
};

use super::contracts::PatchGenerationCallInput;

pub(super) async fn call_patch_generation_llm(
    service: &OrchestratorContext,
    session_id: i64,
    turn_id: i64,
    input: PatchGenerationCallInput<'_, '_>,
) -> Result<ChatResponse, AgentLoopExecutionError> {
    let PatchGenerationCallInput {
        request,
        generation,
        ..
    } = input;
    let output = match execute_llm_generation(
        service.llm_backend(),
        generation.publisher,
        LlmGenerationRequest {
            request,
            stream_id: "raw_json".to_string(),
            cancellation: generation.cancellation.clone(),
            checkpoint: Some(LlmGenerationCheckpoint {
                writer: generation.checkpoint_writer,
                session: generation.session,
                metadata: crate::generation::checkpoint::GenerationCheckpointMetadata {
                    step_key: Some("optimization_patch_generation".to_string()),
                    provider_model: Some(service.llm_model_name()),
                    ..Default::default()
                },
            }),
            usage_base: generation.session.snapshot().usage.clone(),
        },
    )
    .await
    {
        Ok(output) => output,
        Err(error) => {
            let public_message = error.message.clone();
            service
                .append_turn_event(
                    turn_id,
                    EVENT_KIND_ERROR,
                    EVENT_STATUS_FAILED,
                    &format!("局部补丁模型调用失败：{public_message}"),
                    None,
                )
                .await?;
            service
                .save_agent_event(
                    session_id,
                    &format!("LLM 局部补丁调用失败：{public_message}"),
                )
                .await?;
            return Err(AgentLoopExecutionError::Generation(error));
        }
    };
    Ok(ChatResponse {
        message: ChatMessage::assistant(output.content),
        usage: output.usage.as_ref().map(provider_usage).transpose()?,
        finish_reason: Some(generation_finish_reason_text(output.finish_reason)),
        reasoning_content: None,
    })
}
