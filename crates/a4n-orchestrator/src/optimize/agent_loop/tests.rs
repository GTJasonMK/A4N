use super::*;

#[test]
fn parses_valid_loop_decision() {
    let decision = parse_agent_loop_decision_response(
        r#"{"thought":"先读正文","public_update":"我会先读取当前章节，确认需要处理的内容。","action":"run_tool","arguments":{"tool":"get_project_information","arguments":{"mode":"read","source":"current_chapter"}}}"#,
    )
    .unwrap();

    assert_eq!(decision.thought, "先读正文");
    assert_eq!(
        decision.public_update.as_deref(),
        Some("我会先读取当前章节，确认需要处理的内容。")
    );
    assert_eq!(decision.action, AgentLoopAction::RunTool);
    assert_eq!(decision.arguments["tool"], "get_project_information");
}

#[test]
fn parses_thinking_observation() {
    let decision = parse_agent_loop_decision_response(
        r#"{"thought":"先明确目标","public_update":"我需要先确认当前请求是否依赖章节正文。","action":"think","arguments":{"focus":"判断当前请求是否需要上下文","known_facts":["用户要求优化当前内容"],"information_gaps":["需要当前章节正文"],"next_step":"读取 current_chapter"}}"#,
    )
    .unwrap();
    let observation = thinking_observation(1, &decision.thought, &decision.arguments).unwrap();

    assert_eq!(decision.action, AgentLoopAction::Think);
    assert_eq!(observation.value["action"], "think");
    assert!(
        observation
            .event_summary
            .contains("判断当前请求是否需要上下文")
    );
}

#[test]
fn rejects_invalid_thinking_arguments() {
    let decision = parse_agent_loop_decision_response(
        r#"{"thought":"x","public_update":"我已经完成本步判断。","action":"think","arguments":{"focus":"x","known_facts":[],"information_gaps":[]}}"#,
    )
    .unwrap();
    let err = thinking_observation(1, &decision.thought, &decision.arguments).unwrap_err();

    assert!(err.to_string().contains("next_step"));
}

#[test]
fn rejects_non_json_decision() {
    let err = parse_agent_loop_decision_response("```json\n{}\n```").unwrap_err();
    assert!(err.to_string().contains("严格 JSON 对象"));
}

#[test]
fn rejects_unknown_action() {
    let err = parse_agent_loop_decision_response(
        r#"{"thought":"x","public_update":"准备修改正文。","action":"edit_article","arguments":{}}"#,
    )
    .unwrap_err();
    assert!(err.to_string().contains("action 非法"));
}

#[test]
fn rejects_unknown_decision_field() {
    let err = parse_agent_loop_decision_response(
        r#"{"thought":"x","public_update":"我会继续。","action":"think","arguments":{"focus":"x","known_facts":[],"information_gaps":[],"next_step":"结束"},"legacy":true}"#,
    )
    .unwrap_err();
    assert!(err.to_string().contains("未知字段 legacy"));
}

#[test]
fn rejects_non_final_decision_without_public_update() {
    let err =
        parse_agent_loop_decision_response(r#"{"thought":"x","action":"think","arguments":{}}"#)
            .unwrap_err();
    assert!(err.to_string().contains("public_update"));
}

#[test]
fn rejects_unknown_think_argument_field() {
    let decision = parse_agent_loop_decision_response(
        r#"{"thought":"x","public_update":"我会先整理判断。","action":"think","arguments":{"focus":"x","known_facts":[],"information_gaps":[],"next_step":"结束","extra":true}}"#,
    )
    .unwrap();
    let err = thinking_observation(1, &decision.thought, &decision.arguments).unwrap_err();
    assert!(err.to_string().contains("未知字段 extra"));
}

#[test]
fn step_user_message_references_context_without_repeating_protocol_rules() {
    let message = build_agent_loop_step_user_message(&[], 1).unwrap();
    assert!(message.contains("context_refs"));
    assert!(message.contains("optimization_agent_base_task"));
    assert!(!message.contains("protocol_constraints"));
    assert!(!message.contains(r#""action": "edit_article""#));
    assert!(!message.contains(r#""action": "run_tool""#));
}

#[test]
fn agent_loop_request_moves_base_task_into_context_manifest() {
    let request = build_first_agent_loop_chat_request(
        42,
        Some(7),
        "content_optimization_agent_loop",
        "系统提示词".to_string(),
        "优化当前章节正文：主角突然转向。",
    )
    .unwrap();
    let manifest = request
        .context_manifest_json
        .as_ref()
        .expect("context manifest should be attached");
    let pack_kinds = manifest["packs"]
        .as_array()
        .expect("packs should be array")
        .iter()
        .filter_map(|pack| pack["pack_kind"].as_str())
        .collect::<Vec<_>>();
    assert!(pack_kinds.contains(&"optimization_agent_base_task"));
    assert!(pack_kinds.contains(&"optimization_agent_rules"));
    let manifest_text = serde_json::to_string(manifest).unwrap();
    assert!(manifest_text.contains("protocol_constraints"));
    assert!(manifest_text.contains("tool_names_are_not_actions"));
    let user_message = request
        .messages
        .iter()
        .filter(|message| {
            matches!(
                message.role,
                a4n_llm_provider_contract::provider::MessageRole::User
            )
        })
        .next_back()
        .expect("user message should exist");
    assert!(user_message.content.contains("context_refs"));
    assert!(
        user_message
            .content
            .contains("optimization_agent_base_task")
    );
    assert!(!user_message.content.contains("主角突然转向"));
    let trace_context = request
        .trace_context
        .as_ref()
        .expect("trace context should be attached");
    assert_eq!(trace_context.workflow, "content_optimization_agent_loop");
    assert_eq!(trace_context.call_key.as_deref(), Some("session_7_step_1"));
}
