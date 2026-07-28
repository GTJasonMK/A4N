use a4n_core::{
    error::AppError,
    models::{ai::optimization_session, chapter::chapter},
};
use a4n_llm::provider::ChatRequest;

use crate::{
    optimize::{
        OptimizationPatchProposal, OrchestratorContext,
        agent_loop_executor::AgentLoopGenerationContext,
        patch::{content_optimization_patch_schema, parse_content_optimization_patch_value},
        turns::{EVENT_KIND_ERROR, EVENT_STATUS_FAILED},
    },
    structured_llm::{StructuredRepairCheckpointContext, StructuredValueRepairInput},
};

pub(super) async fn repair_patch_generation_response(
    service: &OrchestratorContext,
    session: &optimization_session::Model,
    chapter: &chapter::Model,
    turn_id: i64,
    base_content_hash: &str,
    workflow_run_id: i64,
    call_key: &str,
    repair_request: ChatRequest,
    raw_response: &str,
    generation: &AgentLoopGenerationContext<'_>,
) -> Result<OptimizationPatchProposal, AppError> {
    let parse_output = |value: &serde_json::Value| {
        parse_content_optimization_patch_value(
            session.id,
            chapter.project_id,
            chapter.id,
            base_content_hash,
            &chapter.content,
            value,
        )
    };
    match service
        .parse_validate_or_repair_structured_value(StructuredValueRepairInput {
            request: repair_request,
            raw_response,
            label: "内容优化局部补丁",
            schema: content_optimization_patch_schema()?,
            checkpoint: Some(StructuredRepairCheckpointContext {
                workflow_run_id,
                base_call_key: call_key.to_string(),
                call_order: 1,
            }),
            cancel_token: Some(generation.cancellation),
            progress: None,
            parse_output: &parse_output,
        })
        .await
    {
        Ok(repaired) => Ok(repaired.output),
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
