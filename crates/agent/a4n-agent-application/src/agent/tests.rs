//! Agent application projection and validation tests.

use super::*;

#[test]
fn context_summary_validation_rejects_oversized_serialized_payload() {
    let summary = AgentContextSummary {
        schema_version: a4n_agent_runtime_contract::AGENT_CONTEXT_SUMMARY_VERSION,
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
    let capped = child_budget_for_delegation(child, &parent).expect("child budget must be capped");
    assert_eq!(capped.max_steps, Some(99));
    assert_eq!(capped.max_model_calls, Some(99));
    assert_eq!(capped.max_tokens, Some(9_999));
    assert_eq!(capped.max_delegation_depth, Some(2));

    let inherited = child_budget_for_delegation(AgentBudget::default(), &parent).unwrap();
    assert_eq!(inherited.max_steps, None);
    assert_eq!(inherited.max_tokens, None);
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
fn editor_source_requires_a_database_id_and_uses_the_current_chapter_number() {
    let mut value = json!({
        "view_instance_id": "view-1",
        "project_id": 7,
        "chapter_id": 11,
        "chapter_number": 99,
        "editor_revision": "revision-1",
        "scene_id": null,
        "shot_id": null,
        "keyframe_id": null,
        "entity_id": null
    });

    let scope = AgentScope {
        project_id: Some(7),
        chapter_id: Some(11),
        chapter_number: Some(1),
        ..AgentScope::default()
    };
    let source = parse_persisted_editor_source(value.clone(), "director", &scope).unwrap();
    assert_eq!(source.chapter_id, 11);
    assert_eq!(source.chapter_number, Some(1));
    assert!(parse_persisted_editor_source(value.clone(), "canvas", &scope).is_err());
    value["chapter_id"] = json!(12);
    assert!(parse_persisted_editor_source(value.clone(), "director", &scope).is_err());
    value.as_object_mut().unwrap().remove("chapter_id");
    assert!(parse_persisted_editor_source(value, "director", &scope).is_err());
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
async fn completed_model_receipt_replays_after_lease_cleanup_without_another_final() {
    let database = std::sync::Arc::new(a4n_migration::test_support::connect_test_database().await);
    let application = AgentApplication::new(DbHandle::new(database.clone()));
    let thread = application
        .create_thread(1, CreateAgentThreadInput::default())
        .await
        .unwrap();
    let accepted = application
        .send_message(
            1,
            SendAgentMessageInput {
                thread_id: thread.id,
                client_message_id: uuid::Uuid::new_v4().to_string(),
                content: "仅回复：已完成".into(),
                ..SendAgentMessageInput::default()
            },
            false,
            false,
        )
        .await
        .unwrap();
    let claimed = application
        .claim_worker_run("completion-test")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(claimed.run.id.to_string(), accepted.run_id);
    let input_message_id = accepted.message_id.parse::<i64>().unwrap();
    let model = application.create_prepared_execution(WorkerExecutionInput {
        run_id: claimed.run.id,
        lease_token: claimed.lease_token.clone(),
        execution_key: "model-call:1".into(),
        execution_kind: "model_call".into(),
        tool_name: None,
        effect: Some("read".into()),
        replay_policy: Some("safe".into()),
        arguments_json: Some(json!({"messages": [{"role": "user", "content": "仅回复：已完成"}], "max_tokens": 32})),
        resource_preconditions_json: Some(json!({"input_message_id": input_message_id})),
    }).await.unwrap().unwrap();
    application.update_worker_execution(WorkerExecutionUpdate {
        run_id: claimed.run.id,
        lease_token: claimed.lease_token.clone(),
        execution_id: model.id,
        status: "succeeded".into(),
        result_json: Some(json!({"message": {"role": "assistant", "content": "已完成"}, "finish_reason": "stop", "usage": null})),
        result_summary: None,
        result_refs_json: None,
        error_message: None,
    }).await.unwrap();
    let mut request = a4n_worker_contract::AgentWorkerCompletionRequest {
        run_id: claimed.run.id,
        lease: a4n_worker_contract::WorkerLease {
            worker_id: "completion-test".into(),
            epoch: claimed.run.attempt as u64,
            fence_token: claimed.lease_token.clone(),
            expires_at: claimed.run.lease_expires_at.unwrap().to_string(),
        },
        content: "已完成".into(),
        result_summary: "已完成".into(),
        result_refs_json: None,
        observed_input_message_id: input_message_id,
        source_execution_id: model.id,
    };
    assert!(
        application
            .complete_worker_run_request(&request)
            .await
            .unwrap()
            .completed
    );
    let finished = agent::run_by_id(database.as_ref(), claimed.run.id)
        .await
        .unwrap()
        .unwrap();
    assert!(finished.lease_token.is_none());
    assert!(
        application
            .complete_worker_run_request(&request)
            .await
            .unwrap()
            .completed
    );
    let messages = agent::messages_by_run(database.as_ref(), claimed.run.id)
        .await
        .unwrap();
    assert_eq!(
        messages
            .iter()
            .filter(|message| message.client_message_id.as_deref()
                == Some(&format!("agent-final:{}", claimed.run.id)))
            .count(),
        1
    );
    request.content = "不是原模型回复".into();
    assert!(
        application
            .complete_worker_run_request(&request)
            .await
            .is_err()
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
