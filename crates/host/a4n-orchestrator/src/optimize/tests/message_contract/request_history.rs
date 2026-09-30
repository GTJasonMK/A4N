#[test]
fn content_optimization_patch_user_message_keeps_only_task_inputs() {
    let issues = vec![OptimizeIssue {
        id: "issue-001".to_string(),
        tool: "plot_hole".to_string(),
        arguments: json!({"content": "主角突然转向"}),
        description: "主角行动动机不足".to_string(),
        severity: "major".to_string(),
    }];

    let prompt = build_content_optimization_patch_user_message("hash-001", &issues)
        .expect("patch prompt should serialize");

    let payload: serde_json::Value =
        serde_json::from_str(a4n_prompt::request::json_user_message_payload(&prompt).unwrap())
            .unwrap();
    assert_eq!(payload["chapter"]["base_content_hash"], "hash-001");
    assert_eq!(payload["selected_issues"][0]["id"], "issue-001");
    assert_eq!(
        payload["selected_issues"][0]["description"],
        "主角行动动机不足"
    );
    assert!(payload.get("agent").is_none());
    assert!(payload.get("output_contract").is_none());
    assert!(prompt.contains("optimization_patch_chapter_content"));
    assert!(prompt.contains("optimization_patch_project_context"));
    assert!(!prompt.contains("主角突然转向。"));
    assert!(!prompt.contains("主角转向"));
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
    let user_message = build_content_optimization_patch_user_message("hash-001", &issues)
        .expect("patch prompt should serialize");
    let mut request = super::super::build_content_optimization_patch_request(
        42,
        7,
        "补丁系统提示词".to_string(),
        user_message,
        "主角突然转向。",
        &json!({"current_chapter_outline": {"summary": "主角转向"},
                "current_chapter": {"content": "主角突然转向。", "source_spans": [{"text": "主角突然转向"}]}}),
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

    a4n_llm_context::preview_context_aware_chat_request(&mut request, 1_000_000, 8192).unwrap();
    assert_eq!(
        request
            .messages
            .iter()
            .map(|message| message.content.matches("主角突然转向。").count())
            .sum::<usize>(),
        1
    );

    let trace_context = request
        .trace_context
        .as_ref()
        .expect("trace context should be attached");
    assert_eq!(trace_context.workflow, "content_optimization_patch");
    assert_eq!(trace_context.call_key.as_deref(), Some("session_7_patch"));
}

#[test]
fn optimization_chat_history_preserves_long_instructions_and_appends_without_rewriting_prior_turns()
{
    let now = Utc::now().naive_utc();
    let long_instruction = format!("首轮约束{}末尾约束", "甲".repeat(6500));
    let long_reply = format!("{}完整结论", "乙".repeat(6500));
    let turns = (1..=14)
        .map(|id| OptimizationTurnRecord {
            id,
            session_id: 7,
            agent_turn_id: None,
            user_text: if id == 1 {
                long_instruction.clone()
            } else {
                format!("第{id}轮")
            },
            status: "completed".into(),
            permissions_snapshot: None,
            final_reply: Some(long_reply.clone()),
            error: None,
            created_at: now,
            updated_at: now,
        })
        .collect::<Vec<_>>();
    let history = optimization_chat_history(&turns);
    let items = history.as_array().unwrap();
    assert_eq!(items.len(), 14);
    assert_eq!(items[0]["user"], long_instruction);
    assert_eq!(items[0]["assistant"], long_reply);
    assert_eq!(items[13]["user"], "第14轮");
    assert_eq!(json!(&items[..13]), optimization_chat_history(&turns[..13]));
    assert!(items[0].get("updated_at").is_none());
}
