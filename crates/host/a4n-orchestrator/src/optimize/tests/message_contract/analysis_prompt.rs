#[test]
fn content_optimization_user_message_uses_json_payload() {
    let prompt = build_content_optimization_user_message(
        "第一段正文",
        &json!({
            "sections": ["current_chapter", "current_chapter_outline", "previous_chapters"],
            "counts": {
                "current_chapter": 1,
                "current_chapter_outline": 1,
                "previous_chapters": 1
            }
        }),
    )
    .expect("content optimization prompt should serialize");

    let payload: serde_json::Value =
        serde_json::from_str(a4n_prompt::request::json_user_message_payload(&prompt).unwrap())
            .unwrap();
    assert_eq!(
        payload["chapter"]["content_chars"],
        "第一段正文".chars().count()
    );
    assert_eq!(
        payload["context_manifest"]["counts"]["previous_chapters"],
        1
    );
    assert!(
        payload["chapter"]["content_available_via"]
            .as_str()
            .unwrap()
            .contains("current_chapter")
    );
    assert!(is_content_optimization_analysis_prompt(&prompt));
    assert!(!prompt.contains("第一段正文"));
    for redundant in [
        "available_actions",
        "protocol_constraints",
        "decision_contract",
        "completion_policy",
    ] {
        assert!(
            payload.get(redundant).is_none(),
            "{redundant} belongs to the fixed rules"
        );
    }
}

#[test]
fn content_optimization_analysis_prompt_detection_uses_payload_task_name() {
    let now = Utc::now().naive_utc();
    let permissions = optimize_test_permissions();
    let chat_prompt = build_content_optimization_chat_user_message(
        "帮我解释这段",
        &OptimizationSessionRecord {
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
    let err = build_content_optimization_user_message(" \n\t ", &json!({}))
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
