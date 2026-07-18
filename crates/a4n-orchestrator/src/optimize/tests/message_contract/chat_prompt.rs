#[test]
fn content_optimization_chat_user_message_uses_session_context() {
    let now = Utc::now().naive_utc();
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
        agent_permissions: None,
        agent_instruction: Some("优先检查动机".to_string()),
        fixes_applied: 0,
        summary: Some("发现 1 个问题".to_string()),
        created_at: now,
        updated_at: now,
    };

    let prompt = build_content_optimization_chat_user_message(
        "这里最大的漏洞是什么？",
        &session,
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

    assert!(prompt.contains(r#""user_message": "这里最大的漏洞是什么？""#));
    assert!(!prompt.contains("主角做出选择"));
    assert!(prompt.contains(r#""context_manifest""#));
    assert!(prompt.contains(r#""think_arguments""#));
    assert!(prompt.contains("get_project_information"));
    assert!(prompt.contains(r#""final_arguments""#));
    assert!(prompt.contains(r#""protocol_constraints""#));
    assert!(prompt.contains(r#""tool_names_are_not_actions""#));
    assert!(prompt.contains(r#""public_update_required_for""#));
    assert!(prompt.contains(r#""action": "edit_article""#));
    assert!(prompt.contains(r#""action": "run_tool""#));
    assert!(prompt.contains(r#""conversation_history""#));
    assert!(!prompt.contains(r#""permissions""#));
    assert!(!prompt.contains(r#""capabilities""#));
    assert!(!prompt.contains(r#""apply_local_patches""#));
    assert!(prompt.contains("当前消息本身已经足以回答"));
    assert!(prompt.contains(r#""completion_policy""#));
    assert!(prompt.contains("人物动机不足"));
    assert!(!prompt.contains("优先检查动机"));
    assert!(!prompt.contains(r#""selected_issue_ids""#));
    assert!(prompt.contains("严格 JSON 决策对象"));
}

#[test]
fn content_optimization_chat_system_prompt_includes_runtime_tool_schema() {
    let engine = TeraPromptEngine::from_directory(&prompts_dir()).unwrap();
    let tools_prompt = serde_json::to_string_pretty(
        &super::super::runtime_tools::optimization_runtime_tool_specs(),
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
    assert!(prompt.contains("工具名，不是顶层 `action`"));
    assert!(prompt.contains(r#"{"action":"run_tool","arguments":{"tool":"edit_article""#));
}
