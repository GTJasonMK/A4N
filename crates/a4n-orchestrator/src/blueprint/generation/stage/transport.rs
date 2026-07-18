use a4n_core::error::AppError;
use a4n_llm::provider::{ChatRequest, ChatResponse};
use a4n_prompt::blueprint_generation::ensure_blueprint_response_not_truncated;
use tokio_util::sync::CancellationToken;

use crate::blueprint::OrchestratorContext;

use super::super::contracts::BlueprintStageSpec;
use super::contracts::BlueprintStageLlmOutput;
use super::rules::ensure_not_cancelled;

pub(super) async fn generate_blueprint_stage_output(
    context: &OrchestratorContext,
    request: ChatRequest,
    spec: &BlueprintStageSpec,
    cancel_token: Option<&CancellationToken>,
) -> Result<BlueprintStageLlmOutput, AppError> {
    let response = chat_with_cancel(context, request, cancel_token).await?;
    ensure_not_cancelled(cancel_token)?;
    ensure_blueprint_response_not_truncated(response.finish_reason.as_deref(), spec.title)?;
    let usage = response
        .usage
        .as_ref()
        .map(serde_json::to_value)
        .transpose()?;
    Ok(BlueprintStageLlmOutput {
        response_content: response.message.content,
        usage,
        finish_reason: response.finish_reason,
    })
}

async fn chat_with_cancel(
    context: &OrchestratorContext,
    request: ChatRequest,
    cancel_token: Option<&CancellationToken>,
) -> Result<ChatResponse, AppError> {
    ensure_not_cancelled(cancel_token)?;
    match cancel_token {
        Some(token) => tokio::select! {
            _ = token.cancelled() => Err(AppError::Cancelled),
            result = context.llm.chat(request) => result,
        },
        None => context.llm.chat(request).await,
    }
}
