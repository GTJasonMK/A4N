use super::{
    ImplicitAttributeReviewSummary, ImplicitUpdateDecisionSummary, parse_implicit_update_decision,
    types,
};
use crate::AppState;
use a4n_llm::provider::is_truncated_finish_reason;
use a4n_orchestrator::workflow::{WORKFLOW_STATUS_COMPLETED, WORKFLOW_STATUS_FAILED};
use serde_json::{Value, json};

pub(super) async fn finalize_implicit_update_review_workflow(
    state: &AppState,
    workflow_run_id: i64,
    result: &Result<ImplicitUpdateDecisionSummary, String>,
) -> Result<(), String> {
    let (status, can_resume, error_message) = match result {
        Ok(_) => (WORKFLOW_STATUS_COMPLETED, false, None),
        Err(message) => (WORKFLOW_STATUS_FAILED, true, Some(message.as_str())),
    };
    state
        .orchestrator
        .mark_workflow_status(workflow_run_id, status, can_resume, error_message)
        .await
        .map_err(|err| err.to_string())?;
    Ok(())
}

pub(crate) fn implicit_update_decision_output_json(
    parsed: &types::ParsedImplicitDecision,
) -> Value {
    json!({
        "decision": parsed.decision,
        "reasoning": parsed.reasoning,
        "suggested_new_value": parsed.suggested_new_value,
        "evidence_summary": parsed.evidence_summary})
}

pub(crate) fn implicit_update_review_checkpoint_response(
    checkpoint_json: &Value,
    call_key: &str,
    stats: &ImplicitAttributeReviewSummary,
) -> Result<(String, Option<Value>, Option<String>), String> {
    let response_text = checkpoint_json
        .get("response_content")
        .and_then(Value::as_str)
        .map(ToString::to_string)
        .ok_or_else(|| format!("{call_key}.response_content 必须是字符串"))?;
    let parsed = parse_implicit_update_decision(&response_text, stats)?;
    let reparsed_output = implicit_update_decision_output_json(&parsed);
    let parsed_output = checkpoint_json
        .get("parsed_output")
        .ok_or_else(|| format!("{call_key}.parsed_output 缺失"))?;
    if parsed_output != &reparsed_output {
        return Err(format!(
            "{call_key} checkpoint 的 parsed_output 与 response_content 重新解析结果不一致"
        ));
    }
    let usage = checkpoint_json
        .get("usage")
        .filter(|value| !value.is_null())
        .cloned();
    let finish_reason = match checkpoint_json.get("finish_reason") {
        Some(Value::Null) | None => None,
        Some(value) => Some(
            value
                .as_str()
                .map(ToString::to_string)
                .ok_or_else(|| format!("{call_key}.finish_reason 必须是字符串或 null"))?,
        ),
    };
    if is_truncated_finish_reason(finish_reason.as_deref()) {
        return Err(format!(
            "{call_key} checkpoint 标记为截断响应，不能用于恢复"
        ));
    }
    Ok((response_text, usage, finish_reason))
}
