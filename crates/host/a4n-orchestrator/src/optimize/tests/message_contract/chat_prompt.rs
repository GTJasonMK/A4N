#[test]
fn content_optimization_chat_user_message_uses_session_context() {
    let now = Utc::now().naive_utc();
    let permissions = optimize_test_permissions();
    let session = optimization_session::Model {
        id: 7,
        project_id: 3,
        chapter_id: 11,
        name: Some("测试会话".to_string()),
        status: "waiting_user".to_string(),
        issues: Some(json!([{
            "id": "issue-001",
            "tool": "plot_hole",
            "arguments": {},
            "description": "人物动机不足",
            "severity": "major"
        }])),
        agent_permissions: Some(permissions.as_value()),
        agent_instruction: Some("优先检查动机".to_string()),
        fixes_applied: 0,
        summary: Some("发现 1 个问题".to_string()),
        archived_at: None,
        created_at: now,
        updated_at: now,
    };

    let prompt = build_content_optimization_chat_user_message(
        "这里最大的漏洞是什么？",
        &session,
        &permissions,
        &json!([{"role": "assistant", "content": "上一轮结论"}]),
        &json!({
            "sections": ["current_chapter", "previous_chapters"],
            "counts": {
                "current_chapter": 1,
                "previous_chapters": 3
            }
        }),
    )
    .expect("chat prompt should serialize");

    let payload: serde_json::Value =
        serde_json::from_str(a4n_prompt::request::json_user_message_payload(&prompt).unwrap())
            .unwrap();
    assert_eq!(payload["user_message"], "这里最大的漏洞是什么？");
    assert_eq!(
        payload["context_manifest"]["counts"]["previous_chapters"],
        3
    );
    assert_eq!(payload["conversation_history"][0]["content"], "上一轮结论");
    assert_eq!(
        payload["session"]["issues"][0]["description"],
        "人物动机不足"
    );
    assert_eq!(payload["automatic_writeback"], false);
    assert!(!prompt.contains("优先检查动机"));
    for redundant in [
        "protocol_constraints",
        "available_actions",
        "decision_contract",
        "completion_policy",
    ] {
        assert!(payload.get(redundant).is_none());
    }
    assert!(prompt.len() < 1500);
}

#[test]
fn content_optimization_chat_system_prompt_includes_runtime_tool_schema() {
    let engine = TeraPromptEngine::from_directory(&prompts_dir()).unwrap();
    let tools_prompt = serde_json::to_string(
        &super::super::runtime_tools::optimization_runtime_tool_specs_for_prompt(),
    )
    .unwrap();
    let variables = std::collections::HashMap::from([
        (
            "agent_instruction".to_string(),
            serde_json::Value::String("帮我优化第二段".to_string()),
        ),
        (
            "tools_prompt".to_string(),
            serde_json::Value::String(tools_prompt),
        ),
    ]);

    let prompt = engine
        .render("content_optimization_chat", &variables, None)
        .expect("chat system prompt should render runtime tool schemas");

    assert!(prompt.contains("get_project_information"));
    assert!(prompt.contains(r#""mode""#));
    assert!(prompt.contains(r#""read""#));
    assert!(prompt.contains(r#""search""#));
    assert!(prompt.contains("edit_article"));
    assert!(prompt.contains("tool"));
    assert!(prompt.contains("action"));
    assert!(prompt.len() < 6_000);
}
