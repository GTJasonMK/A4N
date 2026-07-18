mod checkpoint;
mod contracts;
mod preparation;
mod repair;
mod transport;

use a4n_core::error::AppError;

use crate::optimize::OrchestratorContext;

use super::PatchGenerationCallResult;
use checkpoint::restore_patch_generation_checkpoint;
use contracts::PreparedPatchGenerationCall;
use preparation::prepare_patch_generation_call;
use repair::repair_patch_generation_response;
use transport::call_patch_generation_llm;

pub(super) use contracts::PatchGenerationCallInput;

pub(super) async fn execute_patch_generation_call(
    service: &OrchestratorContext,
    input: PatchGenerationCallInput<'_, '_>,
) -> Result<PatchGenerationCallResult, AppError> {
    let PatchGenerationCallInput {
        session,
        chapter,
        turn_id,
        base_content_hash,
        request,
        workflow_run_id,
        stream,
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
        (restored.response, restored.proposal, true)
    } else {
        let repair_request = request.clone();
        let response =
            call_patch_generation_llm(service, session.id, turn_id, request, stream).await?;
        let proposal = repair_patch_generation_response(
            service,
            session,
            chapter,
            turn_id,
            base_content_hash,
            workflow_run_id,
            &call_key,
            repair_request,
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
