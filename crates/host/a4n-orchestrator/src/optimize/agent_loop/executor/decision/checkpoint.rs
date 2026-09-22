use super::super::{
    AgentLoopDecision, AppError, ChatMessage, ChatResponse, Value, json,
    parse_agent_loop_decision_response,
};
use crate::structured_llm::ensure_checkpoint_parsed_output_matches;

pub(super) fn agent_loop_decision_json(decision: &AgentLoopDecision) -> Value {
    json!({
        "thought": decision.thought,
        "public_update": decision.public_update,
        "action": decision.action.as_str(),
        "arguments": decision.arguments,
    })
}

pub(super) fn restored_agent_loop_decision(
    parsed_output: Value,
    response_content: String,
    usage: Option<Value>,
    finish_reason: Option<String>,
    call_key: &str,
) -> Result<(ChatResponse, AgentLoopDecision), AppError> {
    let decision = parse_agent_loop_decision_response(&response_content)?;
    ensure_checkpoint_parsed_output_matches(
        &parsed_output,
        &agent_loop_decision_json(&decision),
        &format!("内容优化 Agent Loop {call_key}"),
    )?;
    a4n_llm_provider_contract::validate_finish_reason(finish_reason.as_deref())?;
    Ok((
        ChatResponse {
            message: ChatMessage::assistant(response_content),
            usage: usage
                .map(serde_json::from_value)
                .transpose()
                .map_err(|err| AppError::InvalidState {
                    current: "invalid_checkpoint_usage".to_string(),
                    operation: format!("恢复内容优化 Agent Loop 调用断点 {call_key}: {err}"),
                })?,
            finish_reason,
            reasoning_content: None,
        },
        decision,
    ))
}

#[cfg(test)]
mod tests {
    use super::{agent_loop_decision_json, restored_agent_loop_decision};
    use crate::optimize::agent_loop::AgentLoopAction;
    use serde_json::json;

    #[test]
    fn restores_agent_loop_decision_when_checkpoint_matches_reparse() {
        let response_content = r#"{"thought":"读取正文","public_update":"正在读取正文。","action":"run_tool","arguments":{"tool":"get_project_information","arguments":{"mode":"read","source":"current_chapter"}}}"#;
        let (_, parsed) = restored_agent_loop_decision(
            json!({
                "thought": "读取正文",
                "public_update": "正在读取正文。",
                "action": "run_tool",
                "arguments": {
                    "tool": "get_project_information",
                    "arguments": {"mode": "read", "source": "current_chapter"}
                }
            }),
            response_content.to_string(),
            None,
            Some("stop".to_string()),
            "test-call",
        )
        .expect("matching checkpoint should restore");

        assert!(matches!(parsed.action, AgentLoopAction::RunTool));
        assert_eq!(
            agent_loop_decision_json(&parsed)["arguments"]["tool"],
            json!("get_project_information")
        );
    }

    #[test]
    fn rejects_checkpoint_without_a_successful_finish_reason() {
        let response =
            r#"{"thought":"完成","public_update":null,"action":"final","arguments":{"issues":[]}}"#;
        let parsed = super::parse_agent_loop_decision_response(response).unwrap();
        for reason in [None, Some("content_filter"), Some("length")] {
            assert!(
                restored_agent_loop_decision(
                    agent_loop_decision_json(&parsed),
                    response.into(),
                    None,
                    reason.map(str::to_string),
                    "test-call",
                )
                .is_err()
            );
        }
    }

    #[test]
    fn rejects_mismatched_agent_loop_checkpoint() {
        let response_content =
            r#"{"thought":"完成","public_update":null,"action":"final","arguments":{"issues":[]}}"#;
        let error = restored_agent_loop_decision(
            json!({
                "thought": "完成",
                "public_update": null,
                "action": "final",
                "arguments": {"issues": [{"drifted": true}]}
            }),
            response_content.to_string(),
            None,
            None,
            "test-call",
        )
        .expect_err("mismatched checkpoint must fail");

        assert!(
            error
                .to_string()
                .contains("checkpoint_parsed_output_mismatch")
        );
    }
}
