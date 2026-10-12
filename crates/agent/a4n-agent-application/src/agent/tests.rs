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
        Some(7),
        &AgentResourceRef {
            kind: "chapter".into(),
            id: "11".into(),
            label: None,
            revision: None,
            hash: None,
        },
    )
    .expect("chapter binding");
    assert_eq!(
        chapter,
        "subagent:v2:content_optimization:project:7:chapter:11"
    );

    let project = canonical_subagent_binding_key(
        "canvas",
        Some(7),
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
fn redundant_resource_identity_is_completed_without_changing_permissions() {
    let mut scope = AgentScope {
        project_id: Some(7),
        chapter_id: Some(11),
        allowed_tools: vec!["read_chapter_visual_plan".into()],
        ..Default::default()
    };
    complete_scope_resource(&mut scope, "chapter", "11").unwrap();
    let completed = scope.clone();
    complete_scope_resource(&mut scope, "chapter", "11").unwrap();
    assert_eq!(scope, completed);
    assert_eq!(scope.allowed_tools, vec!["read_chapter_visual_plan"]);
    assert!(complete_scope_resource(&mut scope, "chapter", "12").is_err());
    assert_eq!(scope, completed);
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
        agent::messages_by_thread(database.as_ref(), thread.id, None, SNAPSHOT_LIMIT)
            .await
            .expect("messages query")
            .is_empty()
    );
}

#[tokio::test]
async fn list_messages_paginates_past_the_snapshot_window() {
    use sea_orm::ActiveModelTrait;

    let database = std::sync::Arc::new(a4n_migration::test_support::connect_test_database().await);
    let application = AgentApplication::new(DbHandle::new(database.clone()));
    let thread = agent::create_thread(
        database.as_ref(),
        AgentThreadCreateInput {
            owner_user_id: 1,
            thread_kind: "main".into(),
            profile_id: "main".into(),
            title: Some("old message pagination test".into()),
            project_id: None,
            resource_type: None,
            resource_id: None,
            canonical_binding_key: Some(format!(
                "agent-message-pagination:{}",
                uuid::Uuid::new_v4()
            )),
            scope_json: serde_json::to_value(AgentScope::default()).unwrap(),
            default_model_config_id: None,
            is_default: false,
        },
    )
    .await
    .expect("thread must be created");
    let mut message_ids = Vec::with_capacity(1_105);
    for index in 0..1_105 {
        let message = agent::append_message(
            database.as_ref(),
            AgentMessageInput {
                thread_id: thread.id,
                turn_id: None,
                run_id: None,
                role: "user".into(),
                sender_kind: "user".into(),
                sender_thread_id: None,
                client_message_id: None,
                content: format!("message-{index}"),
                status: "complete".into(),
                payload_json: None,
            },
        )
        .await
        .expect("message must be inserted");
        message_ids.push(message.id);
    }

    // A cursor based on IDs must not lose a message when its timestamp moves back.
    a4n_db_models::models::agent::message::ActiveModel {
        id: sea_orm::Set(message_ids[98]),
        created_at: sea_orm::Set(chrono::Utc::now().naive_utc() - chrono::Duration::days(1)),
        ..Default::default()
    }
    .update(database.as_ref())
    .await
    .expect("fixture timestamp must be updated");

    let messages = application
        .list_messages(
            1,
            ListAgentMessagesInput {
                thread_id: thread.id.to_string(),
                before: Some(message_ids[100].to_string()),
                limit: Some(5),
            },
        )
        .await
        .expect("older messages must be listed");

    assert_eq!(
        messages
            .iter()
            .map(|message| message.content.as_str())
            .collect::<Vec<_>>(),
        [
            "message-99",
            "message-98",
            "message-97",
            "message-96",
            "message-95"
        ]
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

#[tokio::test]
async fn list_threads_preserves_filter_archive_order_and_limit_semantics() {
    use a4n_db_models::models::agent::thread;
    use sea_orm::{ActiveModelTrait, Set};

    let database = std::sync::Arc::new(a4n_migration::test_support::connect_test_database().await);
    let application = AgentApplication::new(DbHandle::new(database.clone()));
    let now = chrono::Utc::now().naive_utc();
    let rows = [
        ("target", false, 3),
        ("other", false, 5),
        ("target", true, 4),
        ("target", false, 1),
    ];
    let mut ids = Vec::with_capacity(rows.len());
    for (index, (profile_id, archived, updated_seconds)) in rows.into_iter().enumerate() {
        let row = thread::ActiveModel {
            owner_user_id: Set(1),
            thread_kind: Set("subagent".into()),
            profile_id: Set(profile_id.into()),
            title: Set(Some(format!("Thread {index}"))),
            project_id: Set(None),
            resource_type: Set(None),
            resource_id: Set(None),
            canonical_binding_key: Set(None),
            scope_json: Set(serde_json::to_value(AgentScope::default()).unwrap()),
            default_model_config_id: Set(None),
            status: Set(if archived { "archived" } else { "idle" }.into()),
            is_default: Set(false),
            revision: Set(1),
            next_turn_sequence: Set(0),
            next_event_sequence: Set(0),
            context_summary: Set(None),
            created_at: Set(now),
            updated_at: Set(now + chrono::Duration::seconds(updated_seconds)),
            archived_at: Set(archived.then_some(now)),
            ..Default::default()
        }
        .insert(database.as_ref())
        .await
        .expect("Agent thread fixture must be inserted");
        ids.push(row.id.to_string());
    }

    let filtered = application
        .list_threads(
            1,
            ListAgentThreadsInput {
                capability: Some(" target ".into()),
                include_archived: false,
                limit: Some(1),
            },
        )
        .await
        .expect("filtered thread list must load");
    assert_eq!(
        filtered
            .iter()
            .map(|thread| thread.id.as_str())
            .collect::<Vec<_>>(),
        [ids[0].as_str()],
    );

    let with_archived = application
        .list_threads(
            1,
            ListAgentThreadsInput {
                capability: Some("target".into()),
                include_archived: true,
                limit: Some(10),
            },
        )
        .await
        .expect("archived thread list must load");
    assert_eq!(
        with_archived
            .iter()
            .map(|thread| thread.id.as_str())
            .collect::<Vec<_>>(),
        [ids[2].as_str(), ids[0].as_str(), ids[3].as_str()],
    );

    let all = application
        .list_threads(
            1,
            ListAgentThreadsInput {
                capability: None,
                include_archived: false,
                limit: Some(10),
            },
        )
        .await
        .expect("unfiltered thread list must load");
    assert_eq!(
        all.iter()
            .map(|thread| thread.id.as_str())
            .collect::<Vec<_>>(),
        [ids[1].as_str(), ids[0].as_str(), ids[3].as_str()],
    );
}

#[tokio::test]
#[ignore = "manual PostgreSQL performance benchmark; requires isolated A4N_TEST_DATABASE_URL"]
async fn benchmark_agent_thread_list_pushes_filter_and_limit_into_database() {
    use a4n_db_models::models::agent::thread;
    use sea_orm::{ConnectionTrait, EntityTrait, Set};
    use std::time::Instant;

    const THREAD_COUNT: usize = 10_000;
    const ARCHIVED_COUNT: usize = 9_800;
    const RESULT_COUNT: usize = 200;
    const CONTEXT_SUMMARY_BYTES: usize = 2 * 1024;
    const WARMUPS: usize = 2;
    const SAMPLES: usize = 15;

    let database = std::sync::Arc::new(a4n_migration::test_support::connect_test_database().await);
    let application = AgentApplication::new(DbHandle::new(database.clone()));
    let now = chrono::Utc::now().naive_utc();
    let context_summary = serde_json::json!({
        "schema_version": a4n_agent_runtime_contract::AGENT_CONTEXT_SUMMARY_VERSION,
        "current_goal": "x".repeat(CONTEXT_SUMMARY_BYTES),
        "constraints": [],
        "decisions": [],
        "facts_resources": [],
        "completed": [],
        "pending": [],
        "tool_state": [],
        "covered_through_sequence": 0,
    })
    .to_string();
    let scope_json = serde_json::to_value(AgentScope::default()).unwrap();
    let rows = (0..THREAD_COUNT)
        .map(|index| {
            let archived = index < ARCHIVED_COUNT;
            let profile_id = if archived && index % 2 == 1 {
                "other"
            } else {
                "content_optimization"
            };
            let updated_at = if archived {
                now + chrono::Duration::seconds(index as i64)
            } else {
                now - chrono::Duration::days(1)
                    + chrono::Duration::seconds((index - ARCHIVED_COUNT) as i64)
            };
            thread::ActiveModel {
                owner_user_id: Set(1),
                thread_kind: Set("subagent".into()),
                profile_id: Set(profile_id.into()),
                title: Set(Some(format!("Performance thread {index}"))),
                project_id: Set(None),
                resource_type: Set(None),
                resource_id: Set(None),
                canonical_binding_key: Set(None),
                scope_json: Set(scope_json.clone()),
                default_model_config_id: Set(None),
                status: Set(if archived { "archived" } else { "idle" }.into()),
                is_default: Set(false),
                revision: Set(1),
                next_turn_sequence: Set(0),
                next_event_sequence: Set(0),
                context_summary: Set(Some(context_summary.clone())),
                created_at: Set(now),
                updated_at: Set(updated_at),
                archived_at: Set(archived.then_some(now)),
                ..Default::default()
            }
        })
        .collect::<Vec<_>>();
    for chunk in rows.chunks(250) {
        thread::Entity::insert_many(chunk.iter().cloned())
            .exec(database.as_ref())
            .await
            .expect("Agent thread fixture must be inserted");
    }
    database
        .execute_unprepared("ANALYZE agent_thread")
        .await
        .expect("Agent thread statistics must be refreshed");

    let input = ListAgentThreadsInput {
        capability: None,
        include_archived: false,
        limit: Some(RESULT_COUNT as u32),
    };

    async fn previous_list(
        db: &sea_orm::DatabaseConnection,
        user_id: i64,
        input: &ListAgentThreadsInput,
    ) -> Vec<AgentThreadView> {
        use a4n_db_models::models::agent::thread;
        use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, QueryOrder};

        let capability = input
            .capability
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty());
        let limit = input.limit.expect("benchmark provides a limit") as usize;
        thread::Entity::find()
            .filter(thread::Column::OwnerUserId.eq(user_id))
            .order_by_desc(thread::Column::UpdatedAt)
            .order_by_desc(thread::Column::Id)
            .all(db)
            .await
            .expect("legacy Agent thread rows must load")
            .into_iter()
            .map(a4n_db::agent::AgentThreadRecord::from)
            .filter(|row| input.include_archived || row.archived_at.is_none())
            .filter(|row| capability.is_none_or(|value| row.profile_id == value))
            .take(limit)
            .map(thread_view)
            .collect::<Result<Vec<_>, _>>()
            .expect("legacy Agent thread rows must project")
    }

    fn serialize(rows: Vec<AgentThreadView>) -> (Vec<AgentThreadView>, usize) {
        let bytes = serde_json::to_vec(&rows)
            .expect("Agent thread list must serialize")
            .len();
        (rows, bytes)
    }

    let old = previous_list(database.as_ref(), 1, &input).await;
    let new = application
        .list_threads(1, input.clone())
        .await
        .expect("filtered Agent thread list must load");
    assert_eq!(old, new);
    assert_eq!(new.len(), RESULT_COUNT);
    let response_bytes = serde_json::to_vec(&new)
        .expect("Agent thread response must serialize")
        .len();

    for _ in 0..WARMUPS {
        assert_eq!(previous_list(database.as_ref(), 1, &input).await, new);
        assert_eq!(
            application
                .list_threads(1, input.clone())
                .await
                .expect("filtered warmup must load"),
            new,
        );
    }
    let mut old_samples = Vec::with_capacity(SAMPLES);
    let mut new_samples = Vec::with_capacity(SAMPLES);
    for sample in 0..SAMPLES {
        if sample % 2 == 0 {
            let started = Instant::now();
            let (old, bytes) = serialize(previous_list(database.as_ref(), 1, &input).await);
            old_samples.push(started.elapsed().as_secs_f64() * 1_000.0);
            assert_eq!(bytes, response_bytes);
            assert_eq!(old, new);

            let started = Instant::now();
            let (new_sample, bytes) = serialize(
                application
                    .list_threads(1, input.clone())
                    .await
                    .expect("filtered sample must load"),
            );
            new_samples.push(started.elapsed().as_secs_f64() * 1_000.0);
            assert_eq!(bytes, response_bytes);
            assert_eq!(new_sample, new);
        } else {
            let started = Instant::now();
            let (new_sample, bytes) = serialize(
                application
                    .list_threads(1, input.clone())
                    .await
                    .expect("filtered sample must load"),
            );
            new_samples.push(started.elapsed().as_secs_f64() * 1_000.0);
            assert_eq!(bytes, response_bytes);
            assert_eq!(new_sample, new);

            let started = Instant::now();
            let (old, bytes) = serialize(previous_list(database.as_ref(), 1, &input).await);
            old_samples.push(started.elapsed().as_secs_f64() * 1_000.0);
            assert_eq!(bytes, response_bytes);
            assert_eq!(old, new);
        }
    }

    fn percentile(samples: &[f64], percentile: f64) -> f64 {
        let mut sorted = samples.to_vec();
        sorted.sort_by(f64::total_cmp);
        sorted[((sorted.len() - 1) as f64 * percentile).round() as usize]
    }
    let old_p50 = percentile(&old_samples, 0.50);
    let old_p95 = percentile(&old_samples, 0.95);
    let new_p50 = percentile(&new_samples, 0.50);
    let new_p95 = percentile(&new_samples, 0.95);
    eprintln!(
        "agent_thread_list total={THREAD_COUNT} archived={ARCHIVED_COUNT} result={} context_summary_bytes={CONTEXT_SUMMARY_BYTES} response_bytes={response_bytes} old_p50_p95_ms={old_p50:.3}/{old_p95:.3} new_p50_p95_ms={new_p50:.3}/{new_p95:.3} improvement={:.1}%/{:.1}%",
        new.len(),
        (1.0 - new_p50 / old_p50) * 100.0,
        (1.0 - new_p95 / old_p95) * 100.0,
    );
}
