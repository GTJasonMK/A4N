mod call;
mod completion;
mod preparation;

use a4n_db_models::models::ai::optimization_session;
use a4n_domain::chapter::ChapterRecord;
use a4n_llm_provider_contract::provider::{ChatRequest, ChatResponse};
use serde_json::Value;

use crate::optimize::{
    AgentLoopExecutionError, OptimizationPatchProposal, OptimizeProposePatchesInput,
    OrchestratorContext,
};

use self::call::{PatchGenerationCallInput, execute_patch_generation_call};
pub(super) use self::completion::PendingPatchGeneration;
use self::completion::{PatchGenerationCompletionInput, prepare_patch_generation_completion};
use self::preparation::{PatchGenerationPreparationInput, prepare_patch_generation};

pub(super) struct PreparedPatchGeneration {
    pub(super) session: optimization_session::Model,
    pub(super) chapter: ChapterRecord,
    pub(super) turn_id: i64,
    pub(super) request: ChatRequest,
    pub(super) base_content_hash: String,
}

pub(super) struct PatchGenerationCallResult {
    pub(super) response: ChatResponse,
    pub(super) proposal: OptimizationPatchProposal,
    pub(super) restored: bool,
    pub(super) call_key: String,
    pub(super) input_fingerprint_json: Value,
}

pub(super) async fn prepare_patch_generation_result(
    self_: &OrchestratorContext,
    input: OptimizeProposePatchesInput<'_, '_>,
) -> Result<PendingPatchGeneration, AgentLoopExecutionError> {
    let OptimizeProposePatchesInput {
        session_id,
        issue_ids,
        prompt_renderer,
        agent_instruction_override,
        workflow_run_id,
        generation,
    } = input;
    let prepared = prepare_patch_generation(
        self_,
        PatchGenerationPreparationInput {
            session_id,
            issue_ids,
            prompt_renderer,
            agent_instruction_override,
        },
    )
    .await?;
    let PreparedPatchGeneration {
        session,
        chapter,
        turn_id,
        request,
        base_content_hash,
    } = prepared;
    let call = execute_patch_generation_call(
        self_,
        PatchGenerationCallInput {
            session: &session,
            chapter: &chapter,
            turn_id,
            base_content_hash: &base_content_hash,
            request,
            workflow_run_id,
            generation,
        },
    )
    .await;
    let call = match call {
        Ok(call) => call,
        Err(error) => {
            let status = if error.is_cancelled() {
                "cancelled"
            } else {
                "failed"
            };
            self_.fail_turn(turn_id, status, &error.to_string()).await?;
            return Err(error);
        }
    };
    prepare_patch_generation_completion(
        self_,
        PatchGenerationCompletionInput {
            requested_session_id: session_id,
            session,
            chapter_id: chapter.id,
            turn_id,
            base_content_hash,
            workflow_run_id,
            call,
        },
    )
    .await
}
