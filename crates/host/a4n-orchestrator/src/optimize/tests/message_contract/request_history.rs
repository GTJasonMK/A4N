#[test]
fn content_optimization_patch_user_message_includes_capability_boundary() {
    let issues = vec![OptimizeIssue {
        id: "issue-001".to_string(),
        tool: "plot_hole".to_string(),
        arguments: json!({"content": "主角突然转向"}),
        description: "主角行动动机不足".to_string(),
        severity: "major".to_string(),
    }];

    let prompt = build_content_optimization_patch_user_message(
        "hash-001",
        &issues,
        &optimize_test_permissions(),
        Some("只处理动机问题"),
    )
    .expect("patch prompt should serialize");

    assert!(prompt.contains(r#""capabilities""#));
    assert!(prompt.contains(r#""edit_article": true"#));
    assert!(prompt.contains(r#""automatic_writeback": false"#));
    assert!(prompt.contains(r#""base_content_hash": "hash-001""#));
    assert!(prompt.contains("optimization_patch_chapter_content"));
    assert!(prompt.contains("optimization_patch_project_context"));
    assert!(!prompt.contains("主角突然转向。"));
    assert!(!prompt.contains("主角转向"));
    assert!(prompt.contains(r#""old_text_must_match_once": true"#));
    assert!(prompt.contains("只处理动机问题"));
}

#[test]
fn content_optimization_patch_request_moves_content_and_context_into_manifest() {
    let issues = vec![OptimizeIssue {
        id: "issue-001".to_string(),
        tool: "edit_article".to_string(),
        arguments: json!({"goal": "补足动机"}),
        description: "主角行动动机不足".to_string(),
        severity: "major".to_string(),
    }];
    let permissions = optimize_test_permissions();
    let user_message = build_content_optimization_patch_user_message(
        "hash-001",
        &issues,
        &permissions,
        Some("只处理动机问题"),
    )
    .expect("patch prompt should serialize");
    let request = super::super::build_content_optimization_patch_request(
        42,
        7,
        "补丁系统提示词".to_string(),
        user_message,
        "主角突然转向。",
        &json!({"current_chapter_outline": {"summary": "主角转向"}}),
    )
    .expect("patch request should build");

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
    assert!(pack_kinds.contains(&"optimization_patch_chapter_content"));
    assert!(pack_kinds.contains(&"optimization_patch_project_context"));

    let user_message = request
        .messages
        .iter()
        .find(|message| matches!(message.role, MessageRole::User))
        .expect("user message should exist");
    assert!(user_message.content.contains("context_refs"));
    assert!(
        user_message
            .content
            .contains("optimization_patch_chapter_content")
    );
    assert!(
        user_message
            .content
            .contains("optimization_patch_project_context")
    );
    assert!(!user_message.content.contains("主角突然转向。"));
    assert!(!user_message.content.contains("主角转向"));

    let trace_context = request
        .trace_context
        .as_ref()
        .expect("trace context should be attached");
    assert_eq!(trace_context.workflow, "content_optimization_patch");
    assert_eq!(trace_context.call_key.as_deref(), Some("session_7_patch"));
}

#[test]
fn optimization_chat_history_uses_turns_and_truncates_final_reply() {
    let now = Utc::now().naive_utc();
    let turns = vec![OptimizationTurnRecord {
        id: 3,
        session_id: 7,
        agent_turn_id: None,
        user_text: "上一轮用户请求".to_string(),
        status: "completed".to_string(),
        permissions_snapshot: None,
        final_reply: Some("甲".repeat(OPTIMIZATION_CHAT_CONTENT_LIMIT + 1)),
        error: None,
        created_at: now,
        updated_at: now,
    }];

    let history = optimization_chat_history(&turns);
    let items = history.as_array().expect("history must be an array");
    assert_eq!(items.len(), 1);
    assert_eq!(items[0]["turn_id"], 3);
    assert_eq!(items[0]["status"], "completed");
    assert_eq!(items[0]["user"]["content"], "上一轮用户请求");
    let serialized = serde_json::to_string(&history).unwrap();
    assert!(!serialized.contains("Agent 规划"));
    assert!(!serialized.contains("完整规划提示词"));
    assert_eq!(items[0]["assistant"]["truncated"], true);
    assert_eq!(
        items[0]["assistant"]["original_chars"],
        OPTIMIZATION_CHAT_CONTENT_LIMIT + 1
    );
    assert!(
        items[0]["assistant"]["content"]
            .as_str()
            .expect("content must be a string")
            .contains("内容已截断")
    );
}
