mod checkpoint;
mod contracts;
mod preparation;
mod transport;

use a4n_db_models::models::{ai::optimization_session, chapter::chapter};
use a4n_error::AppError;

use crate::generation::generation_public_error;
use crate::generation::stream::{generation_usage, publish_generation_usage};
use crate::optimize::{
    AgentLoopExecutionError, OptimizationPatchProposal, OrchestratorContext,
    patch::parse_content_optimization_patch_response,
    turns::{EVENT_KIND_ERROR, EVENT_STATUS_FAILED},
};

use super::PatchGenerationCallResult;
use checkpoint::restore_patch_generation_checkpoint;
use contracts::PreparedPatchGenerationCall;
use preparation::prepare_patch_generation_call;
use transport::call_patch_generation_llm;

pub(super) use contracts::PatchGenerationCallInput;

pub(super) async fn execute_patch_generation_call(
    service: &OrchestratorContext,
    input: PatchGenerationCallInput<'_, '_>,
) -> Result<PatchGenerationCallResult, AgentLoopExecutionError> {
    let PatchGenerationCallInput {
        session,
        chapter,
        turn_id,
        base_content_hash,
        request,
        workflow_run_id,
        generation,
    } = input;
    let PreparedPatchGenerationCall {
        request,
        call_key,
        input_fingerprint_json,
    } = prepare_patch_generation_call(service, session.id, workflow_run_id, request).await?;

    let (response, proposal, restored) = if let Some(restored) =
        restore_patch_generation_checkpoint(
            service,
            session,
            chapter,
            turn_id,
            base_content_hash,
            workflow_run_id,
            &call_key,
            &input_fingerprint_json,
        )
        .await?
    {
        generation
            .publisher
            .text_delta("raw_json", &restored.response.message.content)
            .await
            .map_err(|error| {
                AgentLoopExecutionError::Generation(generation_public_error(
                    error.code(),
                    "checkpoint",
                    error,
                ))
            })?;
        if let Some(usage) = restored.response.usage.as_ref() {
            let usage = generation_usage(usage.clone())?;
            publish_generation_usage(
                generation.publisher,
                &usage,
                generation.session.snapshot().usage.as_ref(),
            )
            .await?;
        }
        (restored.response, restored.proposal, true)
    } else {
        let response = call_patch_generation_llm(
            service,
            session.id,
            turn_id,
            PatchGenerationCallInput {
                session,
                chapter,
                turn_id,
                base_content_hash,
                request,
                workflow_run_id,
                generation,
            },
        )
        .await?;
        let proposal = parse_patch_generation_response(
            service,
            session,
            chapter,
            turn_id,
            base_content_hash,
            &response.message.content,
        )
        .await?;
        (response, proposal, false)
    };

    Ok(PatchGenerationCallResult {
        response,
        proposal,
        restored,
        call_key,
        input_fingerprint_json,
    })
}

async fn parse_patch_generation_response(
    service: &OrchestratorContext,
    session: &optimization_session::Model,
    chapter: &chapter::Model,
    turn_id: i64,
    base_content_hash: &str,
    raw_response: &str,
) -> Result<OptimizationPatchProposal, AppError> {
    let proposal = parse_content_optimization_patch_response(
        session.id,
        chapter.project_id,
        chapter.id,
        base_content_hash,
        &chapter.content,
        raw_response,
    );
    match proposal {
        Ok(proposal) => Ok(proposal),
        Err(err) => {
            service
                .append_turn_event(
                    turn_id,
                    EVENT_KIND_ERROR,
                    EVENT_STATUS_FAILED,
                    &format!("局部补丁校验失败：{err}"),
                    None,
                )
                .await?;
            Err(err)
        }
    }
}
