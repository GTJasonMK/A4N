mod call;
mod completion;
mod preparation;

use a4n_core::error::AppError;
use a4n_core::models::{ai::optimization_session, chapter::chapter};
use a4n_llm::provider::{ChatRequest, ChatResponse};
use serde_json::Value;

use crate::optimize::streaming::report_optimize_progress;
use crate::optimize::{
    OptimizationPatchProposal, OptimizeProposePatchesInput, OrchestratorContext,
};

use self::call::{PatchGenerationCallInput, execute_patch_generation_call};
use self::completion::{PatchGenerationCompletionInput, complete_patch_generation};
use self::preparation::{PatchGenerationPreparationInput, prepare_patch_generation};

pub(super) struct PreparedPatchGeneration {
    pub(super) session: optimization_session::Model,
    pub(super) chapter: chapter::Model,
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

impl OrchestratorContext {
    pub async fn propose_patches(
        &self,
        input: OptimizeProposePatchesInput<'_, '_, '_>,
    ) -> Result<OptimizationPatchProposal, AppError> {
        let OptimizeProposePatchesInput {
            session_id,
            issue_ids,
            prompt_renderer,
            agent_instruction_override,
            workflow_run_id,
            mut progress,
            mut stream,
        } = input;
        let prepared = prepare_patch_generation(
            self,
            PatchGenerationPreparationInput {
                session_id,
                issue_ids,
                prompt_renderer,
                agent_instruction_override,
                progress: &mut progress,
            },
        )
        .await?;
        report_optimize_progress(&mut progress, 2, 4, "正在调用模型生成局部补丁")?;

        let PreparedPatchGeneration {
            session,
            chapter,
            turn_id,
            request,
            base_content_hash,
        } = prepared;
        let call = execute_patch_generation_call(
            self,
            PatchGenerationCallInput {
                session: &session,
                chapter: &chapter,
                turn_id,
                base_content_hash: &base_content_hash,
                request,
                workflow_run_id,
                stream: &mut stream,
            },
        )
        .await?;

        complete_patch_generation(
            self,
            PatchGenerationCompletionInput {
                requested_session_id: session_id,
                session,
                chapter_id: chapter.id,
                turn_id,
                base_content_hash,
                workflow_run_id,
                call,
            },
            &mut progress,
        )
        .await
    }
}
