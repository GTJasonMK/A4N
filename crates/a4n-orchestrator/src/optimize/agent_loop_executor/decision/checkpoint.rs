use super::super::{
    AgentLoopDecision, AppError, ChatMessage, ChatResponse, Value, json,
    parse_agent_loop_decision_value, response_content_from_checkpoint,
};

pub(super) fn agent_loop_decision_json(decision: &AgentLoopDecision) -> Value {
    json!({
        "thought": decision.thought,
        "public_update": decision.public_update,
        "action": decision.action.as_str(),
        "arguments": decision.arguments,
    })
}

pub(super) fn restored_agent_loop_decision(
    checkpoint_json: Value,
    call_key: &str,
) -> Result<(ChatResponse, AgentLoopDecision), AppError> {
    let (response_content, usage, finish_reason) =
        response_content_from_checkpoint(&checkpoint_json, call_key)?;
    let parsed_output = checkpoint_json
        .get("parsed_output")
        .cloned()
        .ok_or_else(|| AppError::InvalidState {
            current: "missing_parsed_output".to_string(),
            operation: format!("读取内容优化 Agent Loop 调用断点 {call_key}"),
        })?;
    let decision = parse_agent_loop_decision_value(&parsed_output)?;
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
                "response_content": response_content,
                "parsed_output": {
                    "thought": "读取正文",
                    "public_update": "正在读取正文。",
                    "action": "run_tool",
                    "arguments": {
                        "tool": "get_project_information",
                        "arguments": {"mode": "read", "source": "current_chapter"}
                    }
                },
                "finish_reason": "stop"
            }),
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
    fn restores_repaired_agent_loop_checkpoint_from_parsed_output() {
        let response_content =
            r#"{"thought":"完成","public_update":null,"action":"final","arguments":{"issues":[]}}"#;
        let (_, decision) = restored_agent_loop_decision(
            json!({
                "response_content": response_content,
                "parsed_output": {
                    "thought": "完成",
                    "public_update": null,
                    "action": "final",
                    "arguments": {"issues": [{"drifted": true}]}
                }
            }),
            "test-call",
        )
        .expect("repaired checkpoint should restore its validated parsed output");

        assert_eq!(decision.arguments["issues"][0]["drifted"], json!(true));
    }
}
