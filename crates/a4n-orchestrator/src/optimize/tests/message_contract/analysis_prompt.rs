#[test]
fn content_optimization_user_message_uses_json_payload() {
    let prompt = build_content_optimization_user_message(
        "第一段正文",
        Some(&json!({
            "sections": ["current_chapter", "current_chapter_outline", "previous_chapters"],
            "counts": {
                "current_chapter": 1,
                "current_chapter_outline": 1,
                "previous_chapters": 1
            }
        })),
        &optimize_test_permissions(),
        Some("优先找逻辑漏洞"),
    )
    .expect("content optimization prompt should serialize");

    assert!(prompt.contains(r#""agent""#));
    assert!(prompt.contains(r#""read_character_profiles": true"#));
    assert!(prompt.contains(r#""capabilities""#));
    assert!(prompt.contains(r#""full_chapter_rewrite": false"#));
    assert!(prompt.contains(r#""automatic_writeback": false"#));
    assert!(prompt.contains("优先找逻辑漏洞"));
    assert!(prompt.contains(
        r#""content_available_via": "run_tool.get_project_information(mode=read, source=current_chapter).result.content""#
    ));
    assert!(prompt.contains(
        r#""source_spans_available_via": "run_tool.get_project_information(mode=read, source=current_chapter).result.content.source_spans""#
    ));
    assert!(!prompt.contains(r#""content": "第一段正文""#));
    assert!(prompt.contains(r#""context_manifest""#));
    assert!(prompt.contains(r#""available_actions""#));
    assert!(prompt.contains(r#""protocol_constraints""#));
    assert!(prompt.contains(r#""tool_names_are_not_actions""#));
    assert!(prompt.contains(r#""public_update_required_for""#));
    assert!(prompt.contains(r#""root_action_enum": ["#));
    assert!(prompt.contains(r#""action": "edit_article""#));
    assert!(prompt.contains(r#""action": "run_tool""#));
    assert!(prompt.contains(r#""action": "think|run_tool|final""#));
    assert!(prompt.contains(r#""think_arguments""#));
    assert!(prompt.contains(r#""output_contract""#));
    assert!(prompt.contains(r#""issues": "问题对象数组""#));
    assert!(prompt.contains(r#""evidence_span_id""#));
    assert!(prompt.contains(r#""forbidden_issue_argument_fields": ["#));
    assert!(is_content_optimization_analysis_prompt(&prompt));
    assert!(!prompt.contains("请审阅以下章节内容并提出优化建议："));
}

#[test]
fn content_optimization_analysis_prompt_detection_uses_payload_task_name() {
    let now = Utc::now().naive_utc();
    let permissions = optimize_test_permissions();
    let chat_prompt = build_content_optimization_chat_user_message(
        "帮我解释这段",
        &optimization_session::Model {
            id: 1,
            project_id: 1,
            chapter_id: 1,
            name: None,
            status: "active".to_string(),
            issues: None,
            agent_permissions: Some(permissions.as_value()),
            agent_instruction: None,
            fixes_applied: 0,
            summary: None,
            archived_at: None,
            created_at: now,
            updated_at: now,
            archived_at: None,
        },
        &permissions,
        &json!([]),
        &json!({}),
    )
    .expect("chat prompt should serialize");

    assert!(!is_content_optimization_analysis_prompt(&chat_prompt));
    assert!(!is_content_optimization_analysis_prompt(
        "请根据以下 JSON 输入执行内容优化 Agent Loop。旧格式前缀但没有 payload"
    ));
}

#[test]
fn content_optimization_user_message_rejects_blank_content() {
    let err = build_content_optimization_user_message(
        " \n\t ",
        None,
        &optimize_test_permissions(),
        None,
    )
    .expect_err("blank chapter content must fail");

    assert!(matches!(err, AppError::InvalidArgument(_)));
}

#[test]
fn agent_permissions_fixture_allows_read_only_context() {
    let permissions = optimize_test_permissions();

    assert!(permissions.read_project_detail);
    assert!(permissions.read_blueprint);
    assert!(permissions.read_outlines);
    assert!(permissions.read_character_profiles);
    assert!(permissions.read_foreshadowing);
}
