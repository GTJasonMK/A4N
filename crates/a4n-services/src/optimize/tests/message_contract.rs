use super::*;

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
        &OptimizeAgentPermissions::default(),
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
    let chat_prompt = build_content_optimization_chat_user_message(
        "帮我解释这段",
        &optimization_session::Model {
            id: 1,
            project_id: 1,
            chapter_id: 1,
            name: None,
            status: "active".to_string(),
            issues: None,
            agent_permissions: None,
            agent_instruction: None,
            fixes_applied: 0,
            summary: None,
            created_at: now,
            updated_at: now,
        },
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
        &OptimizeAgentPermissions::default(),
        None,
    )
    .expect_err("blank chapter content must fail");

    assert!(matches!(err, AppError::InvalidArgument(_)));
}

#[test]
fn agent_permissions_default_to_read_only_context() {
    let permissions = OptimizeAgentPermissions::from_variables(&std::collections::HashMap::new());

    assert!(permissions.read_project_detail);
    assert!(permissions.read_blueprint);
    assert!(permissions.read_outlines);
    assert!(permissions.read_character_profiles);
    assert!(permissions.read_foreshadowing);
}

#[test]
fn parse_content_optimization_issues_rejects_markdown_fence() {
    let err = parse_content_optimization_issues("```json\n{\"issues\":[]}\n```")
        .expect_err("markdown fence must fail");

    assert!(matches!(err, AppError::InvalidArgument(_)));
}

#[test]
fn optimize_streaming_rejects_max_tokens_finish_reason() {
    let err = super::super::streaming::ensure_optimize_response_not_truncated(Some("max_tokens"))
        .expect_err("max_tokens finish reason must fail before JSON parsing");

    assert!(matches!(err, AppError::InvalidArgument(_)));
    assert!(err.to_string().contains("max_tokens"));
    assert!(err.to_string().contains("JSON"));
}

#[test]
fn parse_content_optimization_issues_rejects_empty_response_clearly() {
    let err =
        parse_content_optimization_issues(" \n\t ").expect_err("empty response must fail clearly");

    assert!(err.to_string().contains("模型返回空内容"));
}

#[test]
fn parse_content_optimization_issues_requires_array() {
    let err =
        parse_content_optimization_issues(r#"{"issues":{}}"#).expect_err("issues must be an array");

    assert!(matches!(err, AppError::InvalidArgument(_)));
}

#[test]
fn parse_optimization_issues_accepts_canonical_array_only() {
    let issues = parse_optimization_issues(&json!([{
        "id": "issue-001",
        "tool": "edit_article",
        "arguments": {"goal": "补足动机"},
        "description": "动机不足",
        "severity": "major"
    }]))
    .expect("canonical issues should parse");

    assert_eq!(issues.len(), 1);
    assert_eq!(issues[0].id, "issue-001");
    assert_eq!(issues[0].tool, "edit_article");
    assert_eq!(issues[0].description, "动机不足");
    assert_eq!(issues[0].severity, "major");

    let wrapped = parse_optimization_issues(&json!({
        "issues": [{"id": "issue-001", "tool": "edit_article", "arguments": {}}]
    }))
    .expect_err("wrapped issues are not the persisted contract");
    assert!(wrapped.to_string().contains("issues 必须是数组"));

    let missing_arguments = parse_optimization_issues(&json!([{
        "id": "issue-001",
        "tool": "edit_article"
    }]))
    .expect_err("arguments are required");
    assert!(missing_arguments.to_string().contains("缺少 arguments"));

    let missing_severity = parse_optimization_issues(&json!([{
        "id": "issue-001",
        "tool": "edit_article",
        "arguments": {"goal": "补足动机"},
        "description": "动机不足"
    }]))
    .expect_err("severity is required by the persisted issue contract");
    assert!(missing_severity.to_string().contains("severity"));

    let unknown_field = parse_optimization_issues(&json!([{
        "id": "issue-001",
        "tool": "edit_article",
        "arguments": {"goal": "补足动机"},
        "description": "动机不足",
        "severity": "major",
        "legacy": true
    }]))
    .expect_err("unknown issue fields must fail");
    assert!(unknown_field.to_string().contains("未知字段 legacy"));
}

#[test]
fn materialize_optimization_issue_evidence_uses_source_span_id() {
    let issues = parse_optimization_issues(&json!([{
        "id": "issue-001",
        "tool": "edit_article",
        "arguments": {
            "evidence_span_id": "ch1:s2:f2",
            "goal": "补足行动动机"
        },
        "description": "动机不足",
        "severity": "major"
    }]))
    .expect("canonical model issues should parse before materialization");
    let materialized = materialize_optimization_issue_evidence(
        issues,
        &json!({
            "current_chapter": {
                "source_spans": [
                    {"id": "ch1:s2:f1", "text": "主角收到旧信"},
                    {"id": "ch1:s2:f2", "text": "决定进入旧城"}
                ]
            }
        }),
    )
    .expect("source span id should be materialized");

    assert_eq!(materialized[0].arguments["evidence"], "决定进入旧城");
    assert!(materialized[0].arguments.get("evidence_span_id").is_none());
}

#[test]
fn materialize_optimization_issue_evidence_rejects_legacy_evidence() {
    let issues = parse_optimization_issues(&json!([{
        "id": "issue-001",
        "tool": "edit_article",
        "arguments": {
            "evidence": "模型自由抄写的证据",
            "goal": "补足行动动机"
        },
        "description": "动机不足",
        "severity": "major"
    }]))
    .expect("shape parser should not validate model boundary evidence");
    let err = materialize_optimization_issue_evidence(
        issues,
        &json!({
            "current_chapter": {
                "source_spans": [{"id": "ch1:s2:f2", "text": "决定进入旧城"}]
            }
        }),
    )
    .expect_err("legacy evidence must be rejected at model boundary");

    assert!(err.to_string().contains("arguments.evidence 已废弃"));
}

#[test]
fn materialize_optimization_issue_evidence_rejects_unknown_argument_fields() {
    let issues = parse_optimization_issues(&json!([{
        "id": "issue-001",
        "tool": "edit_article",
        "arguments": {
            "evidence_span_id": "ch1:s2:f2",
            "goal": "补足行动动机",
            "old_text": "模型不应在 issue 阶段输出补丁字段"
        },
        "description": "动机不足",
        "severity": "major"
    }]))
    .expect("shape parser should parse issue before materialization");
    let err = materialize_optimization_issue_evidence(
        issues,
        &json!({
            "current_chapter": {
                "source_spans": [{"id": "ch1:s2:f2", "text": "决定进入旧城"}]
            }
        }),
    )
    .expect_err("issue arguments must reject unknown model-owned fields");

    assert!(err.to_string().contains("arguments 包含未知字段 old_text"));
}

#[test]
fn materialize_optimization_issue_evidence_rejects_unknown_span_id() {
    let issues = parse_optimization_issues(&json!([{
        "id": "issue-001",
        "tool": "edit_article",
        "arguments": {
            "evidence_span_id": "ch1:s9:f9",
            "goal": "补足行动动机"
        },
        "description": "动机不足",
        "severity": "major"
    }]))
    .expect("shape parser should accept issue before materialization");
    let err = materialize_optimization_issue_evidence(
        issues,
        &json!({
            "current_chapter": {
                "source_spans": [{"id": "ch1:s2:f2", "text": "决定进入旧城"}]
            }
        }),
    )
    .expect_err("unknown span id must fail visibly");

    assert!(err.to_string().contains("evidence_span_id 不在"));
}

#[test]
fn parse_content_optimization_patch_response_accepts_empty_patch_list() {
    let proposal = super::super::patch::parse_content_optimization_patch_response(
        1,
        2,
        3,
        "hash-001",
        "正文内容",
        r#"{"summary":"没有找到可安全唯一定位的局部修改。","patches":[]}"#,
    )
    .expect("empty patch proposal should be a valid no-op proposal");

    assert_eq!(proposal.summary, "没有找到可安全唯一定位的局部修改。");
    assert!(proposal.patches.is_empty());
}

#[test]
fn parse_content_optimization_patch_response_rejects_unknown_patch_field() {
    let err = super::super::patch::parse_content_optimization_patch_response(
        1,
        2,
        3,
        "hash-001",
        "主角突然转向。",
        r#"{"summary":"补足动机","patches":[{"patch_id":"patch-001","issue_id":"issue-001","operation":"replace","old_text":"主角突然转向。","new_text":"主角想到旧信中的线索，突然转向。","reason":"补足动机","risk":"low","legacy":true}]}"#,
    )
    .expect_err("unknown patch fields must fail");

    assert!(err.to_string().contains("unknown field `legacy`"));
}

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
        &OptimizeAgentPermissions::default(),
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
    let permissions = OptimizeAgentPermissions::default();
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
    let turns = vec![optimization_turn::Model {
        id: 3,
        session_id: 7,
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
