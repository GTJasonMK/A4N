use a4n_llm::provider::ChatRequest;

use super::super::super::{
    AgentLoopDecision, AgentLoopDecisionRequest, AppError, EVENT_KIND_ERROR, EVENT_STATUS_FAILED,
    OrchestratorContext, agent_loop_decision_schema, json, parse_agent_loop_decision_value,
};
use super::rules::agent_loop_call_order;
use crate::structured_llm::{StructuredRepairCheckpointContext, StructuredValueRepairInput};

pub(super) async fn repair_agent_loop_decision(
    context: &OrchestratorContext,
    turn_id: i64,
    session_id: i64,
    request: &AgentLoopDecisionRequest<'_>,
    repair_request: ChatRequest,
    raw_response: &str,
    call_key: &str,
) -> Result<AgentLoopDecision, AppError> {
    let parse_output = |value: &serde_json::Value| parse_agent_loop_decision_value(value);
    match context
        .parse_validate_or_repair_structured_value(StructuredValueRepairInput {
            request: repair_request,
            raw_response,
            label: "内容优化 Agent Loop 决策",
            schema: agent_loop_decision_schema(),
            checkpoint: Some(StructuredRepairCheckpointContext {
                workflow_run_id: request.workflow_run_id,
                base_call_key: call_key.to_string(),
                call_order: agent_loop_call_order(request.step)?,
            }),
            cancel_token: Some(request.generation.cancellation),
            progress: None,
            parse_output: &parse_output,
        })
        .await
    {
        Ok(repaired) => Ok(repaired.output),
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
