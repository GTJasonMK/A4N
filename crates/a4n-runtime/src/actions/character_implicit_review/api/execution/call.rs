use crate::{
    AppState,
    actions::{
        character_implicit_review::{
            ImplicitUpdateDecisionSummary,
            checkpoint::{
                implicit_update_decision_output_json, implicit_update_review_checkpoint_response,
            },
            contracts::CheckImplicitUpdateRequestContext,
            parse_implicit_update_decision,
        },
        workflow_checkpoint::{CompletedLlmCallCheckpointInput, required_llm_call_checkpoint_json},
    },
};
use a4n_llm::provider::is_truncated_finish_reason;
use serde_json::Value;

pub(super) struct ManualImplicitReviewCallInput {
    pub(super) profile_id: i64,
    pub(super) attribute_key: String,
    pub(super) workflow_run_id: i64,
    pub(super) call_key: String,
    pub(super) input_fingerprint_json: Value,
}

pub(super) async fn run_manual_implicit_review_call(
    state: &AppState,
    request_context: CheckImplicitUpdateRequestContext,
    input: ManualImplicitReviewCallInput,
) -> Result<ImplicitUpdateDecisionSummary, String> {
    let (response_text, response_usage_json, response_finish_reason, restored) = if let Some(
        checkpoint,
    ) = state
        .load_completed_llm_call_checkpoint(
            input.workflow_run_id,
            &input.call_key,
            &input.input_fingerprint_json,
        )
        .await
        .map_err(|error| error.to_string())?
    {
        let checkpoint_json = required_llm_call_checkpoint_json(checkpoint, &input.call_key)
            .map_err(|error| error.to_string())?;
        let (response_text, usage, finish_reason) = implicit_update_review_checkpoint_response(
            &checkpoint_json,
            &input.call_key,
            &request_context.stats,
        )?;
        (response_text, usage, finish_reason, true)
    } else {
        let response = state
            .llm
            .chat(request_context.request)
            .await
            .map_err(|error| error.to_string())?;
        if is_truncated_finish_reason(response.finish_reason.as_deref()) {
            return Err("隐性属性复审输出达到当前 max_tokens 上限，返回 JSON 被截断；请在设置页调大“JSON 标准”输出档位，或减少本次复审上下文后重试".to_string());
        }
        let response_text = response.message.content.clone();
        let response_usage_json = response
            .usage
            .as_ref()
            .map(serde_json::to_value)
            .transpose()
            .map_err(|error| format!("LLM 用量序列化失败: {error}"))?;
        (
            response_text,
            response_usage_json,
            response.finish_reason,
            false,
        )
    };
    let parsed = parse_implicit_update_decision(&response_text, &request_context.stats)?;
    if !restored {
        state
            .save_completed_llm_call_checkpoint(CompletedLlmCallCheckpointInput {
                workflow_run_id: input.workflow_run_id,
                call_key: input.call_key,
                call_order: 1,
                input_fingerprint_json: input.input_fingerprint_json,
                checkpoint_json: serde_json::json!({
                    "output": implicit_update_decision_output_json(&parsed),
                    "parsed_output": implicit_update_decision_output_json(&parsed),
                    "response_content": response_text,
                    "usage": response_usage_json,
                    "finish_reason": response_finish_reason,
                }),
                artifacts_json: None,
            })
            .await
            .map_err(|error| error.to_string())?;
    }
    Ok(ImplicitUpdateDecisionSummary {
        profile_id: input.profile_id,
        attribute_key: input.attribute_key,
        current_value: request_context.current_value,
        decision: parsed.decision,
        reasoning: parsed.reasoning,
        suggested_new_value: parsed.suggested_new_value,
        evidence_summary: parsed.evidence_summary,
        stats: request_context.stats,
    })
}
