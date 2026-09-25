//! Agent application projection and validation tests.

use super::*;

#[test]
fn context_summary_validation_rejects_oversized_serialized_payload() {
    let summary = AgentContextSummary {
        current_goal: "goal".into(),
        constraints: vec!["x".repeat(4_000); 64],
        decisions: vec!["x".repeat(4_000); 64],
        facts_resources: vec!["x".repeat(4_000); 64],
        completed: vec!["x".repeat(4_000); 64],
        pending: vec!["x".repeat(4_000); 64],
        tool_state: vec!["x".repeat(4_000); 64],
        covered_through_sequence: 1,
    };
    assert!(summary.validate().is_err());
}

#[test]
fn thread_sequence_view_exposes_next_sequence_after_latest_value() {
    assert_eq!(next_sequence_view(0, "next_event_sequence").unwrap(), 1);
    assert_eq!(
        next_sequence_view(1443, "next_event_sequence").unwrap(),
        1444
    );
    assert!(next_sequence_view(-1, "next_event_sequence").is_err());
}

#[test]
fn budget_validation_rejects_zero_limits_and_caps_child_budget() {
    let invalid = AgentBudget {
        max_children: Some(0),
        ..AgentBudget::default()
    };
    assert!(validate_agent_budget(&invalid).is_err());
    let invalid_duration = AgentBudget {
        max_duration_seconds: Some(i64::MAX as u64 + 1),
        ..AgentBudget::default()
    };
    assert!(validate_agent_budget(&invalid_duration).is_err());

    let parent = AgentBudget {
        max_steps: Some(10),
        max_model_calls: Some(8),
        max_tokens: Some(1_000),
        max_delegation_depth: Some(2),
        ..AgentBudget::default()
    };
    let child = AgentBudget {
        max_steps: Some(99),
        max_model_calls: Some(99),
        max_tokens: Some(9_999),
        max_delegation_depth: Some(9),
        ..AgentBudget::default()
    };
    let capped = child_budget_within_parent(
        child,
        &parent,
        &json!({
            "steps_used": 3,
            "model_calls_used": 2,
            "tokens_used": 250
        }),
    )
    .expect("child budget must be capped");
    assert_eq!(capped.max_steps, Some(7));
    assert_eq!(capped.max_model_calls, Some(6));
    assert_eq!(capped.max_tokens, Some(750));
    assert_eq!(capped.max_delegation_depth, Some(2));

    let exhausted = child_budget_within_parent(
        AgentBudget::default(),
        &AgentBudget {
            max_steps: Some(3),
            ..AgentBudget::default()
        },
        &json!({"steps_used": 3}),
    )
    .expect_err("an exhausted parent budget must not admit another child");
    assert!(exhausted.to_string().contains("max_steps"));
    let malformed = child_budget_within_parent(
        AgentBudget::default(),
        &AgentBudget::default(),
        &json!({"steps_used": "3"}),
    )
    .expect_err("malformed parent usage must not reset to zero");
    assert!(malformed.to_string().contains("steps_used"));
}

#[test]
fn tool_search_excludes_hidden_commands_and_honors_limit() {
    let application = AgentApplication::new(DbHandle::new(std::sync::Arc::new(
        sea_orm::DatabaseConnection::Disconnected,
    )));
    let result = application
        .search_tools(SearchAgentToolsInput {
            query: None,
            domain: None,
            effect: None,
            capability: None,
            limit: Some(1),
        })
        .expect("search tools");
    assert_eq!(result.tools.len(), 1);
    assert!(result.total >= result.tools.len());
    assert_ne!(result.tools[0].exposure, AgentToolExposure::Hidden);

    let delegate = application
        .search_tools(SearchAgentToolsInput {
            query: Some("delegate".into()),
            domain: None,
            effect: None,
            capability: Some("main".into()),
            limit: None,
        })
        .expect("synthetic tools must be discoverable");
    assert_eq!(
        delegate
            .tools
            .iter()
            .map(|tool| tool.name.as_str())
            .collect::<Vec<_>>(),
        ["delegate", "review_agent_delegation"]
    );

    let generic = application
        .search_tools(SearchAgentToolsInput {
            query: Some("create_project".into()),
            domain: None,
            effect: None,
            capability: Some("general".into()),
            limit: None,
        })
        .expect("generic capability should search the full manifest");
    assert_eq!(
        generic
            .tools
            .iter()
            .map(|tool| tool.name.as_str())
            .collect::<Vec<_>>(),
        ["create_project"]
    );

    let optimization = application
        .search_tools(SearchAgentToolsInput {
            query: Some("edit article apply patches".into()),
            domain: None,
            effect: None,
            capability: Some("content_optimization".into()),
            limit: Some(20),
        })
        .expect("multi-term searches should use token matches");
    let names = optimization
        .tools
        .iter()
        .map(|tool| tool.name.as_str())
        .collect::<Vec<_>>();
    assert!(names.contains(&"edit_article"));
    assert!(names.contains(&"apply_optimization_patches"));
}

#[test]
fn inbox_claimed_status_remains_distinct_from_pending() {
    assert_eq!(
        parse_inbox_status("claimed").expect("claimed is a live lease state"),
        AgentInboxStatus::Claimed
    );
    assert!(parse_inbox_status("legacy").is_err());
}

#[test]
fn remote_new_run_requires_runtime_snapshot_but_local_run_does_not() {
    let error = admit_new_run_runtime_snapshot(false, true)
        .expect_err("remote new run without snapshot must be rejected");
    assert!(error.to_string().contains("runtime_config_snapshot"));
    admit_new_run_runtime_snapshot(true, true).expect("remote snapshot must be admitted");
    admit_new_run_runtime_snapshot(false, false).expect("local runtime may omit snapshot");
}

#[test]
fn canonical_subagent_key_matches_runtime_domain_binding() {
    let chapter = canonical_subagent_binding_key(
        "content_optimization",
        &AgentResourceRef {
            kind: "chapter".into(),
            id: "11".into(),
            label: None,
            revision: None,
            hash: None,
        },
    )
    .expect("chapter binding");
    assert_eq!(chapter, "subagent:content_optimization:chapter:11");

    let project = canonical_subagent_binding_key(
        "canvas",
        &AgentResourceRef {
            kind: "project".into(),
            id: "7".into(),
            label: None,
            revision: None,
            hash: None,
        },
    )
    .expect("project binding");
    assert_eq!(project, "subagent:canvas:project:7");
}

#[test]
fn generic_subagent_tool_scope_is_discovery_open() {
    let mut scope = AgentScope::default();
    apply_subagent_tool_policy("research", &mut scope)
        .expect("generic capability should remain discovery-open");
    assert!(scope.allowed_tools.is_empty());

    scope.allowed_tools = vec!["create_project".into()];
    apply_subagent_tool_policy("research", &mut scope)
        .expect("explicit generic manifest tool should be valid");
    scope.allowed_tools = vec!["create_llm_config".into()];
    assert!(apply_subagent_tool_policy("research", &mut scope).is_err());
}

#[test]
fn generic_children_keep_nested_delegation_open_but_domains_do_not() {
    let mut generic = AgentScope::default();
    apply_subagent_capability_policy("research", &mut generic)
        .expect("generic capability should remain open");
    assert!(generic.allowed_capabilities.is_empty());

    let mut optimize = AgentScope::default();
    apply_subagent_capability_policy("content_optimization", &mut optimize)
        .expect("domain capability should receive its own fence");
    assert_eq!(optimize.allowed_capabilities, ["content_optimization"]);

    let parent = AgentScope {
        allowed_capabilities: vec!["research".into()],
        allowed_tools: vec!["list_projects".into()],
        ..AgentScope::default()
    };
    inherit_subagent_allowlists(&parent, &mut generic)
        .expect("explicit parent allowlists should be inherited");
    assert_eq!(generic.allowed_capabilities, ["research"]);
    assert_eq!(generic.allowed_tools, ["list_projects"]);
}

#[test]
fn legacy_editor_source_uses_only_the_bound_chapter_as_fallback() {
    let value = json!({
        "view_instance_id": "view-1",
        "project_id": 7,
        "editor_revision": "revision-1",
        "scene_id": null,
        "shot_id": null,
        "keyframe_id": null,
        "entity_id": null
    });

    let normalized = normalize_legacy_editor_source(value, Some(11))
        .expect("legacy editor source should use the bound chapter");

    assert_eq!(normalized["chapter_id"], 11);
}

#[test]
fn legacy_editor_source_without_a_bound_chapter_is_rejected() {
    let error = normalize_legacy_editor_source(json!({}), None)
        .expect_err("chapter-less legacy editor source must be rejected");

    assert!(error.to_string().contains("线程 scope 未绑定章节"));
}

#[tokio::test]
async fn remote_admission_rejects_before_persisting_turn_or_run() {
    let database = std::sync::Arc::new(a4n_migration::test_support::connect_test_database().await);
    let application = AgentApplication::new(DbHandle::new(database.clone()));
    let owner_user_id = 1;
    let thread = agent::create_thread(
        database.as_ref(),
        AgentThreadCreateInput {
            owner_user_id,
            thread_kind: "main".into(),
            profile_id: "main".into(),
            title: Some("snapshot admission test".into()),
            project_id: None,
            resource_type: None,
            resource_id: None,
            canonical_binding_key: Some(format!(
                "agent-snapshot-admission:{}",
                uuid::Uuid::new_v4()
            )),
            scope_json: serde_json::to_value(AgentScope::default()).unwrap(),
            default_model_config_id: None,
            is_default: false,
        },
    )
    .await
    .expect("thread must be created");

    let error = application
        .send_message(
            owner_user_id,
            SendAgentMessageInput {
                thread_id: thread.id.to_string(),
                client_message_id: uuid::Uuid::new_v4().to_string(),
                content: "需要远程快照".into(),
                ..SendAgentMessageInput::default()
            },
            false,
            true,
        )
        .await
        .expect_err("missing remote snapshot must be rejected");
    assert!(error.to_string().contains("runtime_config_snapshot"));
    assert!(
        agent::runs_by_thread(database.as_ref(), thread.id, SNAPSHOT_LIMIT)
            .await
            .expect("runs query")
            .is_empty()
    );
    assert!(
        agent::messages_by_thread(database.as_ref(), thread.id, SNAPSHOT_LIMIT)
            .await
            .expect("messages query")
            .is_empty()
    );
}

#[tokio::test]
async fn user_spawn_honors_parent_delegation_budget() {
    let database = std::sync::Arc::new(a4n_migration::test_support::connect_test_database().await);
    let application = AgentApplication::new(DbHandle::new(database.clone()));
    let parent_thread = agent::create_thread(
        database.as_ref(),
        AgentThreadCreateInput {
            owner_user_id: 1,
            thread_kind: "main".into(),
            profile_id: "main".into(),
            title: Some("spawn budget test".into()),
            project_id: None,
            resource_type: None,
            resource_id: None,
            canonical_binding_key: Some(format!("agent-spawn-budget:{}", uuid::Uuid::new_v4())),
            scope_json: serde_json::to_value(AgentScope::default()).unwrap(),
            default_model_config_id: None,
            is_default: false,
        },
    )
    .await
    .expect("parent thread must be created");
    let client_message_id = uuid::Uuid::new_v4().to_string();
    let (_, parent_run, _) = agent::create_turn_with_run(
        database.as_ref(),
        AgentTurnInput {
            thread_id: parent_thread.id,
            client_message_id: client_message_id.clone(),
            message: AgentMessageInput {
                thread_id: parent_thread.id,
                turn_id: None,
                run_id: None,
                role: "user".into(),
                sender_kind: "user".into(),
                sender_thread_id: None,
                client_message_id: Some(client_message_id),
                content: "创建一个子 Agent".into(),
                status: "complete".into(),
                payload_json: None,
            },
            execution_mode: "autonomous".into(),
            model_override_json: None,
            budget_json: serde_json::to_value(AgentBudget {
                max_children: Some(1),
                max_steps: Some(10),
                ..AgentBudget::default()
            })
            .unwrap(),
            runtime_snapshot_required: false,
            trigger: "user".into(),
            goal: Some("创建一个子 Agent".into()),
        },
    )
    .await
    .expect("parent run must be created");

    let spawn = |task: &str| SpawnAgentInput {
        parent_run_id: parent_run.id.to_string(),
        capability: "general".into(),
        task: task.into(),
        title: None,
        bound_resource: None,
        scope: AgentScope::default(),
        success_criteria: Vec::new(),
        budget: Some(AgentBudget {
            max_steps: Some(99),
            ..AgentBudget::default()
        }),
        runtime_config_snapshot: None,
    };
    application
        .spawn_agent(1, spawn("第一个任务"))
        .await
        .expect("first child must be admitted");
    let error = application
        .spawn_agent(1, spawn("第二个任务"))
        .await
        .expect_err("max_children must apply to user spawn");
    assert!(error.to_string().contains("max_children"));
}
