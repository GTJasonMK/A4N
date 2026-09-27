use a4n_db::agent::{
    AgentDbError, AgentUserDelegationInput, append_event, append_message, bind_delegation_child,
    cancel_run_tree, claim_inbox, claim_runnable_run, consume_inbox_with_message,
    create_delegation, create_execution_for_run_lease, create_thread, create_turn_with_run,
    create_user_delegation, delegation_by_parent_and_key, enqueue_inbox, expire_pending_approvals,
    expired_running_runs, finish_run_with_message, latest_events_by_thread,
    mark_turn_active_for_run_lease, recover_expired_run, release_inbox_claim, renew_run_lease,
    request_approval_for_run_lease, supersede_inbox, update_delegation,
    update_execution_for_run_lease, update_run_state, update_run_state_if_unchanged,
    update_run_state_with_budget, update_run_usage,
};
use a4n_db_models::models::agent::{
    approval, delegation, event, execution, inbox, run, thread, turn,
};
use a4n_domain::agent::{
    AgentApprovalInput, AgentDelegationInput, AgentEventInput, AgentExecutionInput,
    AgentInboxInput, AgentMessageInput, AgentThreadCreateInput, AgentTurnInput,
};
use chrono::{Duration, Utc};
use sea_orm::{ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, Set};
use serde_json::json;
use uuid::Uuid;

async fn setup_db() -> DatabaseConnection {
    a4n_migration::test_support::connect_test_database().await
}

async fn create_test_thread(db: &DatabaseConnection, profile: &str) -> thread::Model {
    create_thread(
        db,
        AgentThreadCreateInput {
            owner_user_id: 1,
            thread_kind: if profile == "main" {
                "main".to_string()
            } else {
                "subagent".to_string()
            },
            profile_id: profile.to_string(),
            title: Some(format!("Agent test {profile}")),
            project_id: None,
            resource_type: None,
            resource_id: None,
            canonical_binding_key: Some(format!("agent-test:{profile}:{}", Uuid::new_v4())),
            scope_json: json!({}),
            default_model_config_id: None,
            is_default: false,
        },
    )
    .await
    .expect("thread must be created")
}

async fn create_test_run(
    db: &DatabaseConnection,
    thread_id: i64,
    trigger: &str,
) -> (turn::Model, run::Model) {
    let client_message_id = Uuid::new_v4().to_string();
    let (turn, run, _) = create_turn_with_run(
        db,
        AgentTurnInput {
            thread_id,
            client_message_id: client_message_id.clone(),
            message: AgentMessageInput {
                thread_id,
                turn_id: None,
                run_id: None,
                role: "user".to_string(),
                sender_kind: "user".to_string(),
                sender_thread_id: None,
                client_message_id: Some(client_message_id),
                content: "执行测试任务".to_string(),
                status: "complete".to_string(),
                payload_json: None,
            },
            execution_mode: "autonomous".to_string(),
            model_override_json: None,
            budget_json: json!({}),
            runtime_snapshot_required: false,
            trigger: trigger.to_string(),
            goal: Some("验证 Agent 生命周期".to_string()),
        },
    )
    .await
    .expect("turn and run must be created");
    (turn, run)
}

fn budget_execution(run_id: i64, key: &str, model: bool) -> AgentExecutionInput {
    AgentExecutionInput {
        run_id,
        execution_key: key.into(),
        execution_kind: if model { "model_call" } else { "tool_call" }.into(),
        status: "prepared".into(),
        tool_name: (!model).then(|| "read_canvas_graph".into()),
        effect: Some("read".into()),
        replay_policy: Some("safe".into()),
        arguments_json: Some(json!({"max_tokens": 100})),
        resource_preconditions_json: None,
    }
}

#[tokio::test]
async fn failed_duration_budget_keeps_its_usage_readable() {
    use a4n_db::agent::run_budget_snapshot;
    let db = setup_db().await;
    let thread = create_test_thread(&db, "main").await;
    let (_, run) = create_test_run(&db, thread.id, "user").await;
    let id = run.id;
    let mut row: run::ActiveModel = run.into();
    row.budget_json = Set(json!({"max_duration_seconds": i64::MAX}));
    row.started_at = Set(Some(Utc::now().naive_utc()));
    row.status = Set("running".into());
    let row = row.update(&db).await.unwrap();
    assert!(matches!(
        run_budget_snapshot(&db, id).await,
        Err(AgentDbError::InvalidArgument(_))
    ));
    let mut row: run::ActiveModel = row.into();
    row.status = Set("failed".into());
    row.is_active = Set(false);
    row.last_error = Set(Some("Agent 时长预算超出时钟范围".into()));
    row.update(&db).await.unwrap();
    let snapshot = run_budget_snapshot(&db, id).await.unwrap();
    assert_eq!(
        snapshot.usage["duration_deadline_unix_ms"],
        serde_json::Value::Null
    );
    assert_eq!(snapshot.usage["model_calls_used"], 0);
    assert_eq!(snapshot.usage["tokens_used"], 0);

    let child_thread = create_test_thread(&db, "general").await;
    let (_, child) = create_test_run(&db, child_thread.id, "delegation").await;
    let child_id = child.id;
    let mut child: run::ActiveModel = child.into();
    child.parent_run_id = Set(Some(id));
    child.root_run_id = Set(Some(id));
    child.update(&db).await.unwrap();
    assert!(
        matches!(run_budget_snapshot(&db, child_id).await,
        Err(AgentDbError::InvalidArgument(message)) if message.contains("时钟范围")),
        "an active child must retain its ancestor's duration constraint"
    );
}

#[tokio::test]
async fn budget_one_admits_one_call_and_extension_resumes_the_same_run() {
    use a4n_db::agent::{resume_run, run_budget_snapshot, wait_run_for_lease};
    use a4n_domain::agent::AgentBudget;
    let db = setup_db().await;
    let thread = create_test_thread(&db, "main").await;
    let (_, run) = create_test_run(&db, thread.id, "user").await;
    let mut active: run::ActiveModel = run.into();
    active.budget_json = Set(json!({"max_model_calls": 1}));
    active.update(&db).await.unwrap();
    let run = claim_runnable_run(&db, "budget-test")
        .await
        .unwrap()
        .unwrap();
    let token = run.lease_token.as_deref().unwrap();
    update_run_usage(
        &db,
        run.id,
        token,
        json!({"steps_used": 1, "model_calls_used": 1}),
    )
    .await
    .unwrap();
    assert_eq!(
        run_budget_snapshot(&db, run.id)
            .await
            .unwrap()
            .remaining
            .max_model_calls,
        Some(1)
    );
    let execution =
        create_execution_for_run_lease(&db, budget_execution(run.id, "model-call:1", true), token)
            .await
            .unwrap();
    assert!(matches!(
        create_execution_for_run_lease(&db, budget_execution(run.id, "model-call:2", true), token)
            .await,
        Err(AgentDbError::BudgetExhausted(_))
    ));
    update_execution_for_run_lease(
        &db,
        run.id,
        token,
        execution.id,
        "succeeded",
        Some(json!({"usage": null})),
        None,
        None,
        None,
    )
    .await
    .unwrap();
    let usage = run_budget_snapshot(&db, run.id).await.unwrap().usage;
    assert_eq!(usage["usage_complete"], false);
    assert!(usage["tokens_reserved"].as_u64().unwrap() > 0);
    wait_run_for_lease(&db, run.id, token, "budget")
        .await
        .unwrap();
    assert!(resume_run(&db, 1, thread.id, run.id, None).await.is_err());
    let resumed = resume_run(
        &db,
        1,
        thread.id,
        run.id,
        Some(&AgentBudget {
            max_model_calls: Some(2),
            ..AgentBudget::default()
        }),
    )
    .await
    .unwrap();
    assert_eq!(resumed.id, run.id);
    assert_eq!(resumed.started_at, run.started_at);
    assert_eq!(resumed.usage_json["model_calls_used"], 1);
    let reclaimed = claim_runnable_run(&db, "budget-test")
        .await
        .unwrap()
        .unwrap();
    assert_ne!(reclaimed.lease_token, run.lease_token);
    create_execution_for_run_lease(
        &db,
        budget_execution(run.id, "model-call:2", true),
        reclaimed.lease_token.as_deref().unwrap(),
    )
    .await
    .unwrap();
}

#[tokio::test]
async fn approved_delegation_creates_one_child_and_consumes_approval_atomically() {
    use a4n_db::agent::{
        AgentChildDelegationInput, create_child_delegation, resolve_approval_for_run,
    };
    let db = setup_db().await;
    let parent_thread = create_test_thread(&db, "main").await;
    let child_thread = create_test_thread(&db, "general").await;
    let (_, parent) = create_test_run(&db, parent_thread.id, "user").await;
    let claimed = claim_runnable_run(&db, "delegate-test")
        .await
        .unwrap()
        .unwrap();
    let execution = create_execution_for_run_lease(
        &db,
        AgentExecutionInput {
            run_id: parent.id,
            execution_key: "tool-call:delegate-approved".into(),
            execution_kind: "delegation".into(),
            status: "prepared".into(),
            tool_name: Some("delegate".into()),
            effect: Some("write".into()),
            replay_policy: Some("idempotent".into()),
            arguments_json: Some(json!({"capability": "general", "task": "检查结果"})),
            resource_preconditions_json: None,
        },
        claimed.lease_token.as_deref().unwrap(),
    )
    .await
    .unwrap();
    let (grant, _, _) = request_approval_for_run_lease(
        &db,
        claimed.lease_token.as_deref().unwrap(),
        AgentApprovalInput {
            thread_id: parent_thread.id,
            run_id: parent.id,
            execution_id: execution.id,
            operation_digest: "delegate-approved".into(),
            operation_json: json!({"command": "delegate"}),
            resource_preconditions_json: None,
            expires_at: None,
        },
    )
    .await
    .unwrap();
    resolve_approval_for_run(&db, grant.id, 1, true, None)
        .await
        .unwrap();
    let claimed = claim_runnable_run(&db, "delegate-test")
        .await
        .unwrap()
        .unwrap();
    let input = AgentChildDelegationInput {
        parent_lease_token: claimed.lease_token,
        delegation: AgentDelegationInput {
            parent_thread_id: parent_thread.id,
            parent_run_id: parent.id,
            parent_execution_id: Some(execution.id),
            child_thread_id: child_thread.id,
            child_turn_id: None,
            child_run_id: None,
            delegation_key: "approved-child".into(),
            capability: "general".into(),
            objective: "检查结果".into(),
            success_criteria_json: json!(["提交证据"]),
            scope_json: json!({}),
            budget_json: json!({}),
            status: "pending".into(),
        },
        child_turn: AgentTurnInput {
            thread_id: child_thread.id,
            client_message_id: "approved-child".into(),
            message: AgentMessageInput {
                thread_id: child_thread.id,
                turn_id: None,
                run_id: None,
                role: "user".into(),
                sender_kind: "agent".into(),
                sender_thread_id: Some(parent_thread.id),
                client_message_id: Some("approved-child".into()),
                content: "检查结果".into(),
                status: "complete".into(),
                payload_json: None,
            },
            execution_mode: "review_writes".into(),
            model_override_json: None,
            budget_json: json!({}),
            runtime_snapshot_required: false,
            trigger: "delegation".into(),
            goal: Some("检查结果".into()),
        },
    };
    let (_, _, child, _) = create_child_delegation(&db, input.clone()).await.unwrap();
    let (_, _, replayed, _) = create_child_delegation(&db, input).await.unwrap();
    assert_eq!(child.id, replayed.id);
    assert_eq!(child.parent_run_id, Some(parent.id));
    assert_eq!(
        approval::Entity::find_by_id(grant.id)
            .one(&db)
            .await
            .unwrap()
            .unwrap()
            .status,
        "consumed"
    );
    assert_eq!(
        execution::Entity::find_by_id(execution.id)
            .one(&db)
            .await
            .unwrap()
            .unwrap()
            .status,
        "running"
    );
}

#[tokio::test]
async fn generation_ownership_fences_attempts_and_keeps_cancelled_call_cost() {
    use a4n_db::agent::{
        AgentOwnedTask, AgentTaskExecutionInput, create_task_execution, generation_owner,
        generation_owner_stopped, run_budget_snapshot, update_task_execution, wait_run_for_lease,
    };
    use a4n_db::generation::{
        PersistGenerationLifecycleInput, ReserveGenerationTaskInput,
        persist_generation_status_changed, reserve_generation_task,
    };
    use a4n_queue::priority::{TaskKind, TaskPriority};
    use a4n_service_protocol::{GenerationDeliveryMode, GenerationTaskStatus};

    let db = setup_db().await;
    a4n_db::package::ensure_host_package_states(&db, &["ai-workflow".into()])
        .await
        .unwrap();
    let user = a4n_db::auth::create_user(
        &db,
        "agent_generation",
        "agent_generation@example.invalid",
        "test-password",
        "user",
    )
    .await
    .unwrap();
    assert_eq!(user.id, 1);
    let thread = create_test_thread(&db, "content_optimization").await;
    let (_, owner) = create_test_run(&db, thread.id, "user").await;
    let claimed = claim_runnable_run(&db, "owner").await.unwrap().unwrap();
    let request_id = Uuid::new_v4().to_string();
    let reservation = ReserveGenerationTaskInput {
        requested_by_user_id: 1,
        start_request_id: request_id,
        business_key_json: json!({"version": 1, "agent_test": owner.id}),
        input_fingerprint_json: json!({"prompt": "original request"}),
        task_kind: TaskKind::TextGeneration,
        priority: TaskPriority::High,
        delivery_mode: GenerationDeliveryMode::Stream,
        batch_policy: None,
        project_id: None,
        root_project_id: None,
        source_snapshot_id: None,
        task_scope: None,
        workflow_run_id: None,
        generation_run_id: None,
        target: None,
        root_task_id: None,
        parent_task_id: None,
        item_key: None,
        retry_of_task_id: None,
    };
    let task = reserve_generation_task(&db, reservation.clone())
        .await
        .unwrap()
        .task;
    let attempt = task.checkpoint.attempt as u32;
    let task_id = task.checkpoint.task_id.clone();
    for unowned_task_id in [task_id.as_str(), "unknown-task"] {
        assert!(
            generation_owner(&db, unowned_task_id, attempt)
                .await
                .unwrap()
                .is_none()
        );
    }
    assert!(matches!(
        generation_owner(&db, &task_id, attempt + 1).await,
        Err(AgentDbError::Conflict(_))
    ));
    let owned_task = AgentOwnedTask::Generation {
        task_id: task_id.clone(),
        attempt,
    };
    let mut start = budget_execution(owner.id, "tool-call:optimization-start", false);
    start.tool_name = Some("send_content_optimization_message".into());
    start.resource_preconditions_json = Some(owned_task.preconditions());
    create_execution_for_run_lease(&db, start, claimed.lease_token.as_deref().unwrap())
        .await
        .unwrap();
    let mut running_snapshot = task.public_snapshot.clone().unwrap();
    running_snapshot.covers_through_sequence += 1;
    running_snapshot.status = GenerationTaskStatus::Running;
    running_snapshot.updated_at_ms = Utc::now().timestamp_millis();
    persist_generation_status_changed(
        &db,
        PersistGenerationLifecycleInput {
            task_id: task_id.clone(),
            attempt,
            expected_checkpoint_revision: task.checkpoint.checkpoint_revision,
            expected_status: GenerationTaskStatus::Reserved,
            public_snapshot: running_snapshot,
        },
    )
    .await
    .unwrap();
    wait_run_for_lease(
        &db,
        owner.id,
        claimed.lease_token.as_deref().unwrap(),
        "external",
    )
    .await
    .unwrap();
    assert_eq!(
        generation_owner(&db, &task_id, attempt)
            .await
            .unwrap()
            .unwrap()
            .id,
        owner.id
    );
    assert!(matches!(
        generation_owner(&db, &task_id, attempt + 1).await,
        Err(AgentDbError::Conflict(_))
    ));

    let alias_thread = create_test_thread(&db, "content_optimization").await;
    let (_, alias_run) = create_test_run(&db, alias_thread.id, "user").await;
    let alias_claimed = claim_runnable_run(&db, "observer").await.unwrap().unwrap();
    let mut alias = reservation;
    alias.start_request_id = Uuid::new_v4().to_string();
    let mut alias_start = budget_execution(alias_run.id, "tool-call:alias", false);
    alias_start.tool_name = Some("send_content_optimization_message".into());
    alias_start.resource_preconditions_json =
        Some(json!({"generation_start_request_id": alias.start_request_id}));
    create_execution_for_run_lease(
        &db,
        alias_start,
        alias_claimed.lease_token.as_deref().unwrap(),
    )
    .await
    .unwrap();
    assert_eq!(
        reserve_generation_task(&db, alias)
            .await
            .unwrap()
            .task
            .checkpoint
            .task_id,
        task_id
    );
    assert_eq!(
        generation_owner(&db, &task_id, attempt)
            .await
            .unwrap()
            .unwrap()
            .id,
        owner.id
    );

    let input = || AgentTaskExecutionInput {
        task: owned_task.clone(),
        execution_key: "decision:1".into(),
        tool_name: None,
        arguments: json!({"max_tokens": 100}),
        input_message_id: None,
        base_input_fingerprint: None,
    };
    let model = create_task_execution(&db, input()).await.unwrap().unwrap();
    assert_eq!(
        create_task_execution(&db, input())
            .await
            .unwrap()
            .unwrap()
            .id,
        model.id
    );
    update_task_execution(&db, &owned_task, model.id, None, None, None)
        .await
        .unwrap();
    cancel_run_tree(&db, owner.id, Some("test cancellation"))
        .await
        .unwrap();
    let stopped = generation_owner(&db, &task_id, attempt)
        .await
        .unwrap()
        .unwrap();
    assert!(generation_owner_stopped(&db, &stopped).await.unwrap());
    assert!(create_task_execution(&db, input()).await.is_err());
    update_task_execution(
        &db,
        &owned_task,
        model.id,
        Some(json!({"usage": {"total_tokens": 125}})),
        None,
        None,
    )
    .await
    .unwrap();
    let usage = run_budget_snapshot(&db, owner.id).await.unwrap().usage;
    assert_eq!(usage["tokens_used"], 125);
    assert_eq!(usage["tokens_reserved"], 0);
    assert_eq!(usage["model_calls_used"], 1);
    assert!(
        update_task_execution(
            &db,
            &owned_task,
            model.id,
            Some(json!({"usage": {"total_tokens": 1}})),
            None,
            None
        )
        .await
        .is_err()
    );
}

#[tokio::test]
async fn shared_budget_serializes_siblings_and_counts_replay_once() {
    use a4n_db::agent::run_budget_snapshot;
    let db = setup_db().await;
    let root_thread = create_test_thread(&db, "main").await;
    let (_, root) = create_test_run(&db, root_thread.id, "user").await;
    let mut active: run::ActiveModel = root.clone().into();
    active.budget_json = Set(json!({"max_model_calls": 1, "max_tokens": 5000}));
    active.update(&db).await.unwrap();
    let root_claim = claim_runnable_run(&db, "root").await.unwrap().unwrap();
    let mut claims = Vec::new();
    for _ in 0..2 {
        let thread = create_test_thread(&db, "general").await;
        let (_, child) = create_test_run(&db, thread.id, "delegation").await;
        let mut child: run::ActiveModel = child.into();
        child.parent_run_id = Set(Some(root.id));
        child.root_run_id = Set(Some(root.id));
        child.update(&db).await.unwrap();
        claims.push(claim_runnable_run(&db, "child").await.unwrap().unwrap());
    }
    let inputs = [
        budget_execution(claims[0].id, "model-call:1", true),
        budget_execution(claims[1].id, "model-call:1", true),
    ];
    let (a, b) = tokio::join!(
        create_execution_for_run_lease(
            &db,
            inputs[0].clone(),
            claims[0].lease_token.as_deref().unwrap()
        ),
        create_execution_for_run_lease(
            &db,
            inputs[1].clone(),
            claims[1].lease_token.as_deref().unwrap()
        ),
    );
    let (winner, loser, index) = match (a, b) {
        (Ok(winner), Err(loser)) => (winner, loser, 0),
        (Err(loser), Ok(winner)) => (winner, loser, 1),
        _ => panic!("exactly one sibling must reserve the shared model budget"),
    };
    assert!(matches!(loser, AgentDbError::BudgetExhausted(_)));
    let token = claims[index].lease_token.as_deref().unwrap();
    update_execution_for_run_lease(
        &db,
        winner.run_id,
        token,
        winner.id,
        "succeeded",
        Some(json!({"usage": {"total_tokens": 123}})),
        None,
        None,
        None,
    )
    .await
    .unwrap();
    let replay = create_execution_for_run_lease(&db, inputs[index].clone(), token)
        .await
        .unwrap();
    assert_eq!(replay.id, winner.id);
    let total = run_budget_snapshot(&db, root.id).await.unwrap();
    assert_eq!(total.usage["model_calls_used"], 1);
    assert_eq!(total.usage["tokens_used"], 123);
    assert_eq!(total.remaining.max_tokens, Some(4877));
    assert_eq!(total.remaining.max_model_calls, Some(0));
    assert!(
        create_execution_for_run_lease(
            &db,
            budget_execution(root.id, "model-call:parent", true),
            root_claim.lease_token.as_deref().unwrap()
        )
        .await
        .is_err()
    );
    let persisted = run::Entity::find_by_id(root.id)
        .one(&db)
        .await
        .unwrap()
        .unwrap()
        .usage_json;
    assert_eq!(
        persisted["runtime_snapshot_required"],
        root.usage_json["runtime_snapshot_required"]
    );
    assert!(persisted.get("model_calls_used").is_none());
    assert!(persisted.get("tokens_used").is_none());
}

#[tokio::test]
async fn token_reservation_is_shared_until_an_actual_receipt_arrives() {
    use a4n_db::agent::run_budget_snapshot;
    let db = setup_db().await;
    let thread = create_test_thread(&db, "main").await;
    let (_, row) = create_test_run(&db, thread.id, "user").await;
    let request = budget_execution(row.id, "model-call:1", true);
    let reservation =
        a4n_domain::agent::agent_model_token_reservation(request.arguments_json.as_ref().unwrap());
    let mut active: run::ActiveModel = row.clone().into();
    active.budget_json = Set(json!({"max_tokens": reservation + 100}));
    active.update(&db).await.unwrap();
    let claimed = claim_runnable_run(&db, "tokens").await.unwrap().unwrap();
    let token = claimed.lease_token.as_deref().unwrap();
    let first = create_execution_for_run_lease(&db, request, token)
        .await
        .unwrap();
    assert!(matches!(
        create_execution_for_run_lease(&db, budget_execution(row.id, "model-call:2", true), token)
            .await,
        Err(AgentDbError::BudgetExhausted(_))
    ));
    update_execution_for_run_lease(
        &db,
        row.id,
        token,
        first.id,
        "failed",
        None,
        None,
        None,
        Some("receipt lost".into()),
    )
    .await
    .unwrap();
    assert_eq!(
        run_budget_snapshot(&db, row.id)
            .await
            .unwrap()
            .remaining
            .max_tokens,
        Some(100)
    );
    assert_eq!(
        run_budget_snapshot(&db, row.id).await.unwrap().usage["tokens_reserved"],
        reservation
    );
}

#[tokio::test]
async fn delegation_review_requires_matching_observed_versions_and_is_not_implicit() {
    use a4n_db::agent::review_delegation;
    use a4n_domain::agent::AgentDelegationReviewInput;
    let db = setup_db().await;
    let parent_thread = create_test_thread(&db, "main").await;
    let child_thread = create_test_thread(&db, "canvas").await;
    let (turn, parent) = create_test_run(&db, parent_thread.id, "user").await;
    let (child_turn, child) = create_test_run(&db, child_thread.id, "delegation").await;
    let ref1 = json!({"kind": "canvas_graph", "id": "17", "revision": "2"});
    let mut ids = Vec::new();
    for key in ["accept", "revise"] {
        let row = create_delegation(
            &db,
            AgentDelegationInput {
                parent_thread_id: parent_thread.id,
                parent_run_id: parent.id,
                parent_execution_id: None,
                child_thread_id: child_thread.id,
                child_turn_id: None,
                child_run_id: None,
                delegation_key: key.into(),
                capability: "canvas".into(),
                objective: "修改画布".into(),
                success_criteria_json: json!([]),
                scope_json: json!({}),
                budget_json: json!({}),
                status: "pending".into(),
            },
        )
        .await
        .unwrap();
        bind_delegation_child(&db, row.id, parent.id, child_turn.id, child.id)
            .await
            .unwrap();
        update_delegation(
            &db,
            row.id,
            "awaiting_acceptance",
            Some(json!([ref1])),
            Some("交付".into()),
            None,
        )
        .await
        .unwrap();
        ids.push(row.id);
    }
    let review = |id, status: &str, refs| AgentDelegationReviewInput {
        user_id: 1,
        delegation_id: id,
        expected_child_run_id: child.id,
        status: status.into(),
        reason: "核查了指定版本".into(),
        reviewed_refs: refs,
    };
    update_run_state(&db, child.id, None, "completed", None, None)
        .await
        .unwrap();
    assert!(
        review_delegation(&db, review(ids[0], "accepted", vec![ref1.clone()]))
            .await
            .is_err()
    );
    let claimed = claim_runnable_run(&db, "review").await.unwrap().unwrap();
    let token = claimed.lease_token.as_deref().unwrap();
    let evidence = create_execution_for_run_lease(
        &db,
        budget_execution(parent.id, "tool-call:read", false),
        token,
    )
    .await
    .unwrap();
    update_execution_for_run_lease(
        &db,
        parent.id,
        token,
        evidence.id,
        "succeeded",
        Some(json!({})),
        None,
        Some(json!([ref1])),
        None,
    )
    .await
    .unwrap();
    let premature = finish_run_with_message(
        &db,
        parent.id,
        token,
        AgentMessageInput {
            thread_id: parent_thread.id,
            turn_id: Some(turn.id),
            run_id: Some(parent.id),
            role: "assistant".into(),
            sender_kind: "agent".into(),
            sender_thread_id: None,
            client_message_id: Some("final-review".into()),
            content: "完成答复".into(),
            status: "complete".into(),
            payload_json: None,
        },
        None,
        turn.input_message_id,
    )
    .await;
    assert!(matches!(premature, Err(AgentDbError::CompletionBlocked(_))));
    assert_eq!(
        delegation::Entity::find_by_id(ids[0])
            .one(&db)
            .await
            .unwrap()
            .unwrap()
            .status,
        "awaiting_acceptance"
    );
    let mut wrong_user = review(ids[0], "accepted", vec![ref1.clone()]);
    wrong_user.user_id = 2;
    assert!(review_delegation(&db, wrong_user).await.is_err());
    assert!(
        review_delegation(
            &db,
            review(
                ids[0],
                "accepted",
                vec![json!({"kind": "canvas_graph", "id": "17", "revision": "1"})]
            )
        )
        .await
        .is_err()
    );
    let accepted = review_delegation(&db, review(ids[0], "accepted", vec![ref1.clone()]))
        .await
        .unwrap();
    assert_eq!(accepted.status, "accepted");
    assert_eq!(
        review_delegation(&db, review(ids[0], "accepted", vec![ref1]))
            .await
            .unwrap()
            .id,
        accepted.id
    );
    let revised = review_delegation(&db, review(ids[1], "revision_requested", vec![]))
        .await
        .unwrap();
    assert_eq!(revised.status, "running");
    assert_ne!(revised.child_run_id, Some(child.id));
    let revision_run = run::Entity::find_by_id(revised.child_run_id.unwrap())
        .one(&db)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(revision_run.parent_run_id, Some(parent.id));
    assert_eq!(revision_run.root_run_id, Some(parent.id));
    assert_eq!(revision_run.thread_id, child.thread_id);
    assert_eq!(
        review_delegation(&db, review(ids[1], "revision_requested", vec![]))
            .await
            .unwrap()
            .child_run_id,
        revised.child_run_id
    );
}

#[tokio::test]
async fn question_commits_receipt_before_question_and_fast_reply_never_sleeps() {
    use a4n_db::agent::{AgentQuestionInput, messages_by_run, request_user_input};
    let db = setup_db().await;
    let thread = create_test_thread(&db, "main").await;
    let (_, owner) = create_test_run(&db, thread.id, "user").await;
    let claimed = claim_runnable_run(&db, "question").await.unwrap().unwrap();
    let token = claimed.lease_token.as_deref().unwrap();
    let mut input = budget_execution(owner.id, "tool-call:ask", false);
    input.tool_name = Some("request_agent_input".into());
    input.arguments_json = Some(json!({"question":"请选择结尾"}));
    let execution = create_execution_for_run_lease(&db, input, token)
        .await
        .unwrap();
    enqueue_test_inbox(&db, thread.id, owner.turn_id, owner.id).await;
    let input = || AgentQuestionInput {
        run_id: owner.id,
        lease_token: token.into(),
        execution_id: execution.id,
        question: "请选择结尾".into(),
        observation: json!({"waiting_for_user":true}),
    };
    let question = request_user_input(&db, input()).await.unwrap();
    assert_eq!(
        request_user_input(&db, input()).await.unwrap().id,
        question.id
    );
    let messages = messages_by_run(&db, owner.id).await.unwrap();
    let tool = messages
        .iter()
        .find(|message| message.role == "tool")
        .unwrap();
    assert!(tool.id < question.id);
    assert_eq!(
        messages
            .iter()
            .filter(|message| message.client_message_id.as_deref()
                == Some(format!("agent-notice:{}", execution.id).as_str()))
            .count(),
        1
    );
    assert_eq!(
        run::Entity::find_by_id(owner.id)
            .one(&db)
            .await
            .unwrap()
            .unwrap()
            .status,
        "queued"
    );
}

#[tokio::test]
async fn parent_budget_extension_wakes_descendants_on_both_sides_of_wait_race() {
    use a4n_db::agent::{resume_run, wait_run_for_lease};
    use a4n_domain::agent::AgentBudget;
    let db = setup_db().await;
    let parent_thread = create_test_thread(&db, "main").await;
    let (_, root) = create_test_run(&db, parent_thread.id, "user").await;
    let parent = claim_runnable_run(&db, "root").await.unwrap().unwrap();
    let mut active: run::ActiveModel = parent.clone().into();
    active.budget_json = Set(json!({"max_model_calls":1}));
    active.update(&db).await.unwrap();
    create_execution_for_run_lease(
        &db,
        budget_execution(root.id, "model-call:1", true),
        parent.lease_token.as_deref().unwrap(),
    )
    .await
    .unwrap();
    update_run_state(
        &db,
        root.id,
        Some(parent.lease_token.as_deref().unwrap()),
        "waiting",
        Some("child"),
        None,
    )
    .await
    .unwrap();
    for (index, wait_first) in [false, true].into_iter().enumerate() {
        let thread = create_test_thread(&db, "general").await;
        let (_, child) = create_test_run(&db, thread.id, "delegation").await;
        let mut active: run::ActiveModel = child.clone().into();
        active.parent_run_id = Set(Some(root.id));
        active.root_run_id = Set(Some(root.id));
        active.update(&db).await.unwrap();
        let child = claim_runnable_run(&db, "child").await.unwrap().unwrap();
        let token = child.lease_token.as_deref().unwrap();
        assert!(matches!(
            create_execution_for_run_lease(
                &db,
                budget_execution(child.id, "model-call:1", true),
                token
            )
            .await,
            Err(AgentDbError::BudgetExhausted(_))
        ));
        if wait_first {
            wait_run_for_lease(&db, child.id, token, "budget")
                .await
                .unwrap();
        }
        resume_run(
            &db,
            1,
            parent_thread.id,
            root.id,
            Some(&AgentBudget {
                max_model_calls: Some(index as u32 + 2),
                ..Default::default()
            }),
        )
        .await
        .unwrap();
        if !wait_first {
            wait_run_for_lease(&db, child.id, token, "budget")
                .await
                .unwrap();
        }
        let child = claim_runnable_run(&db, "resumed-child")
            .await
            .unwrap()
            .unwrap();
        create_execution_for_run_lease(
            &db,
            budget_execution(child.id, "model-call:1", true),
            child.lease_token.as_deref().unwrap(),
        )
        .await
        .unwrap();
        update_run_state(
            &db,
            child.id,
            child.lease_token.as_deref(),
            "completed",
            None,
            None,
        )
        .await
        .unwrap();
    }
}

#[tokio::test]
async fn pause_tree_resumes_only_its_own_pauses() {
    use a4n_db::agent::{pause_run_tree, resume_run};
    let db = setup_db().await;
    let root_thread = create_test_thread(&db, "main").await;
    let (_, root) = create_test_run(&db, root_thread.id, "user").await;
    let mut children = Vec::new();
    for _ in 0..2 {
        let thread = create_test_thread(&db, "general").await;
        let (_, child) = create_test_run(&db, thread.id, "delegation").await;
        let mut active: run::ActiveModel = child.clone().into();
        active.parent_run_id = Set(Some(root.id));
        active.root_run_id = Set(Some(root.id));
        active.update(&db).await.unwrap();
        children.push(child);
    }
    pause_run_tree(&db, children[1].id).await.unwrap();
    let paused = pause_run_tree(&db, root.id).await.unwrap();
    assert_eq!(paused.len(), 2);
    assert!(
        paused
            .iter()
            .all(|run| run.status == "paused" && run.lease_token.is_none())
    );
    resume_run(&db, 1, root_thread.id, root.id, None)
        .await
        .unwrap();
    assert_eq!(
        run::Entity::find_by_id(children[0].id)
            .one(&db)
            .await
            .unwrap()
            .unwrap()
            .status,
        "queued"
    );
    assert_eq!(
        run::Entity::find_by_id(children[1].id)
            .one(&db)
            .await
            .unwrap()
            .unwrap()
            .status,
        "paused"
    );
}

#[tokio::test]
async fn original_history_paginates_beyond_one_thousand_messages_without_context_gaps() {
    use a4n_db::agent::{messages_before, messages_for_context};
    let db = setup_db().await;
    let thread = create_test_thread(&db, "main").await;
    let (_, owner) = create_test_run(&db, thread.id, "user").await;
    let mut first = 0;
    for index in 0..1005 {
        let message = append_message(
            &db,
            AgentMessageInput {
                thread_id: thread.id,
                turn_id: Some(owner.turn_id),
                run_id: Some(owner.id),
                role: "assistant".into(),
                sender_kind: "agent".into(),
                sender_thread_id: None,
                client_message_id: None,
                content: format!("original-{index}"),
                status: "complete".into(),
                payload_json: None,
            },
        )
        .await
        .unwrap();
        if index == 0 {
            first = message.id;
        }
    }
    let latest = messages_before(&db, thread.id, None, 1000).await.unwrap();
    let old = messages_before(&db, thread.id, Some(latest[0].id), 100)
        .await
        .unwrap();
    assert_eq!(old.len(), 6);
    assert!(old.iter().any(|message| message.id == first));
    assert_eq!(
        messages_for_context(&db, thread.id, owner.id, 0)
            .await
            .unwrap()
            .len(),
        1006
    );
}

#[tokio::test]
async fn external_task_wait_wakes_only_its_live_execution() {
    use a4n_db::agent::{
        list_external_task_waits, wait_run_for_lease_with_request, wake_external_task_wait,
    };
    let db = setup_db().await;
    let thread = create_test_thread(&db, "canvas").await;
    let (_, run) = create_test_run(&db, thread.id, "user").await;
    let claim = claim_runnable_run(&db, "wait").await.unwrap().unwrap();
    let token = claim.lease_token.as_deref().unwrap();
    let mut input = budget_execution(run.id, "tool-call:wait", false);
    input.execution_kind = "wait".into();
    input.tool_name = Some("wait_workflow_run".into());
    let execution = create_execution_for_run_lease(&db, input, token)
        .await
        .unwrap();
    update_execution_for_run_lease(
        &db,
        run.id,
        token,
        execution.id,
        "running",
        None,
        None,
        None,
        None,
    )
    .await
    .unwrap();
    wait_run_for_lease_with_request(&db, run.id, token, "external", None)
        .await
        .unwrap();
    let waiting_event = event::Entity::find()
        .filter(event::Column::RunId.eq(run.id))
        .filter(event::Column::Kind.eq("run_waiting"))
        .one(&db)
        .await
        .unwrap()
        .expect("waiting event must be persisted");
    assert_eq!(
        waiting_event
            .payload_json
            .as_ref()
            .and_then(|payload| payload.get("waiting_reason"))
            .and_then(serde_json::Value::as_str),
        Some("external")
    );
    assert_eq!(list_external_task_waits(&db, 0).await.unwrap().len(), 1);
    wake_external_task_wait(&db, run.id, execution.id + 1)
        .await
        .unwrap();
    assert!(claim_runnable_run(&db, "wrong").await.unwrap().is_none());
    wake_external_task_wait(&db, run.id, execution.id)
        .await
        .unwrap();
    assert_eq!(
        claim_runnable_run(&db, "ready").await.unwrap().unwrap().id,
        run.id
    );
    cancel_run_tree(&db, run.id, None).await.unwrap();
    wake_external_task_wait(&db, run.id, execution.id)
        .await
        .unwrap();
    assert!(
        claim_runnable_run(&db, "cancelled")
            .await
            .unwrap()
            .is_none()
    );
}

#[tokio::test]
async fn editor_wait_is_fenced_and_expires_without_dispatching() {
    use a4n_db::agent::{
        claim_editor_execution, expire_editor_requests, wait_run_for_lease_with_request,
    };
    let db = setup_db().await;
    let thread = create_test_thread(&db, "director").await;
    let (_, run) = create_test_run(&db, thread.id, "user").await;
    let claimed = claim_runnable_run(&db, "editor-test")
        .await
        .unwrap()
        .unwrap();
    let token = claimed.lease_token.as_deref().unwrap();
    let input = AgentExecutionInput {
        run_id: run.id,
        execution_key: "tool-call:editor-1".into(),
        execution_kind: "tool_call".into(),
        status: "prepared".into(),
        tool_name: Some("inspect_editor_target".into()),
        effect: Some("read".into()),
        replay_policy: Some("safe".into()),
        arguments_json: Some(json!({})),
        resource_preconditions_json: None,
    };
    let execution = create_execution_for_run_lease(&db, input.clone(), token)
        .await
        .unwrap();
    let mut different_tool = input;
    different_tool.tool_name = Some("save_editor_draft".into());
    assert!(
        create_execution_for_run_lease(&db, different_tool, token)
            .await
            .is_err()
    );
    let request =
        json!({"execution_id": execution.id.to_string(), "source":{"view_instance_id":"original"}});
    let waiting =
        wait_run_for_lease_with_request(&db, run.id, token, "external", Some(request.clone()))
            .await
            .unwrap();
    assert!(waiting.lease_token.is_none());
    assert!(claim_runnable_run(&db, "other").await.unwrap().is_none());
    assert!(
        claim_editor_execution(&db, 2, run.id, execution.id)
            .await
            .is_err()
    );
    assert!(
        claim_editor_execution(&db, 1, run.id, execution.id + 1)
            .await
            .is_err()
    );
    let (left, right) = tokio::join!(
        claim_editor_execution(&db, 1, run.id, execution.id),
        claim_editor_execution(&db, 1, run.id, execution.id),
    );
    let claims = [left, right]
        .into_iter()
        .filter_map(Result::ok)
        .collect::<Vec<_>>();
    assert_eq!(claims.len(), 1);
    assert!(
        update_run_usage(&db, run.id, token, json!({}))
            .await
            .is_err()
    );
    wait_run_for_lease_with_request(
        &db,
        run.id,
        claims[0].lease_token.as_deref().unwrap(),
        "external",
        Some(request),
    )
    .await
    .unwrap();
    assert_eq!(expire_editor_requests(&db, 10).await.unwrap(), 0);
    let past = Utc::now().naive_utc() - Duration::seconds(31);
    event::Entity::update_many()
        .col_expr(
            event::Column::CreatedAt,
            sea_orm::sea_query::Expr::value(past),
        )
        .filter(event::Column::RunId.eq(run.id))
        .filter(event::Column::Kind.eq("run_waiting"))
        .exec(&db)
        .await
        .unwrap();
    run::Entity::update_many()
        .col_expr(
            run::Column::UpdatedAt,
            sea_orm::sea_query::Expr::value(past),
        )
        .filter(run::Column::Id.eq(run.id))
        .exec(&db)
        .await
        .unwrap();
    assert!(
        claim_editor_execution(&db, 1, run.id, execution.id)
            .await
            .is_err()
    );
    assert_eq!(expire_editor_requests(&db, 10).await.unwrap(), 1);
    assert_eq!(expire_editor_requests(&db, 10).await.unwrap(), 0);
    let failed = execution::Entity::find_by_id(execution.id)
        .one(&db)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(failed.status, "failed");
    assert!(failed.result_json.is_none());
    assert!(failed.error_message.unwrap().contains("未执行写入"));
    let failed_run = run::Entity::find_by_id(run.id)
        .one(&db)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(failed_run.status, "failed");
    assert!(!failed_run.is_active);
    assert!(claim_runnable_run(&db, "resumed").await.unwrap().is_none());
    let failed_turn = turn::Entity::find_by_id(failed_run.turn_id)
        .one(&db)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(failed_turn.status, "failed");
}

#[tokio::test]
async fn assistant_tool_call_message_may_have_empty_content() {
    let db = setup_db().await;
    let thread = create_test_thread(&db, "main").await;

    let message = append_message(
        &db,
        AgentMessageInput {
            thread_id: thread.id,
            turn_id: None,
            run_id: None,
            role: "assistant".to_string(),
            sender_kind: "agent".to_string(),
            sender_thread_id: None,
            client_message_id: Some(Uuid::new_v4().to_string()),
            content: "  ".to_string(),
            status: "complete".to_string(),
            payload_json: Some(json!({
                "tool_calls": [{
                    "id": "call-1",
                    "name": "locate_writing_target",
                    "arguments": {}
                }]
            })),
        },
    )
    .await
    .expect("assistant tool-call decision should be durable without text");
    assert_eq!(message.content, "  ");

    let error = append_message(
        &db,
        AgentMessageInput {
            thread_id: thread.id,
            turn_id: None,
            run_id: None,
            role: "assistant".to_string(),
            sender_kind: "agent".to_string(),
            sender_thread_id: None,
            client_message_id: Some(Uuid::new_v4().to_string()),
            content: "".to_string(),
            status: "complete".to_string(),
            payload_json: None,
        },
    )
    .await
    .expect_err("ordinary empty assistant messages must remain invalid");
    assert!(matches!(error, AgentDbError::InvalidArgument(_)));
}

#[tokio::test]
async fn latest_agent_events_keep_newest_sequence_window() {
    let db = setup_db().await;
    let thread = create_test_thread(&db, "main").await;

    for index in 0..=1_000 {
        append_event(
            &db,
            AgentEventInput {
                thread_id: thread.id,
                turn_id: None,
                run_id: None,
                execution_id: None,
                delegation_id: None,
                message_id: None,
                actor_kind: "test".into(),
                actor_id: None,
                kind: format!("test_{index}"),
                status: "complete".into(),
                public_text: String::new(),
                payload_json: None,
            },
        )
        .await
        .expect("event should be appended");
    }

    let events = latest_events_by_thread(&db, thread.id, 1_000)
        .await
        .expect("latest events should load");
    assert_eq!(events.len(), 1_000);
    assert_eq!(events.first().map(|event| event.sequence), Some(2));
    assert_eq!(events.last().map(|event| event.sequence), Some(1_001));
}

async fn enqueue_test_inbox(
    db: &DatabaseConnection,
    thread_id: i64,
    turn_id: i64,
    run_id: i64,
) -> inbox::Model {
    enqueue_inbox(
        db,
        AgentInboxInput {
            thread_id,
            turn_id: Some(turn_id),
            run_id: Some(run_id),
            source_kind: "user".to_string(),
            source_thread_id: None,
            inbox_kind: "followup".to_string(),
            delivery_mode: "followup".to_string(),
            priority: 0,
            dedupe_key: Uuid::new_v4().to_string(),
            content: "后续消息".to_string(),
            payload_json: None,
            available_at: None,
        },
    )
    .await
    .expect("inbox must be created")
}

#[tokio::test]
async fn recovery_required_terminates_run_and_projects_thread_idle() {
    let db = setup_db().await;
    let thread = create_test_thread(&db, "main").await;
    let (_, run) = create_test_run(&db, thread.id, "user").await;
    let claimed = a4n_db::agent::claim_runnable_run(&db, "test-worker")
        .await
        .expect("run claim must succeed")
        .expect("run must be claimable");
    assert_eq!(claimed.id, run.id);

    let updated = update_run_state(
        &db,
        run.id,
        claimed.lease_token.as_deref(),
        "recovery_required",
        Some("reconcile"),
        Some("provider outcome is unknown"),
    )
    .await
    .expect("recovery_required update must succeed");

    assert!(!updated.is_active);
    assert!(updated.completed_at.is_some());
    assert!(updated.worker_id.is_none());
    assert!(updated.lease_token.is_none());
    assert!(updated.lease_expires_at.is_none());
    let thread = thread::Entity::find_by_id(thread.id)
        .one(&db)
        .await
        .expect("thread query must succeed")
        .expect("thread must exist");
    assert_eq!(thread.status, "idle");
}

#[tokio::test]
async fn stale_resume_cannot_revoke_a_new_worker_lease() {
    let db = setup_db().await;
    let thread = create_test_thread(&db, "main").await;
    let (_, run) = create_test_run(&db, thread.id, "user").await;
    let paused = update_run_state(&db, run.id, None, "paused", Some("user"), None)
        .await
        .expect("run must pause");

    update_run_state_if_unchanged(&db, &paused, None, "queued", None, None, None)
        .await
        .expect("current pause state must resume");
    let claimed = claim_runnable_run(&db, "resume-race-worker")
        .await
        .expect("run claim must succeed")
        .expect("resumed run must be claimable");
    let lease_token = claimed.lease_token.clone().expect("lease must exist");

    let stale_update =
        update_run_state_if_unchanged(&db, &paused, None, "queued", None, None, None).await;
    assert!(matches!(stale_update, Err(AgentDbError::Conflict(_))));

    let current = a4n_db::agent::run_by_id(&db, run.id)
        .await
        .expect("run query must succeed")
        .expect("run must exist");
    assert_eq!(current.status, "running");
    assert_eq!(current.lease_token.as_deref(), Some(lease_token.as_str()));
}

#[tokio::test]
async fn waiting_lifecycle_event_includes_its_reason() {
    let db = setup_db().await;
    let thread = create_test_thread(&db, "main").await;
    let (_, run) = create_test_run(&db, thread.id, "user").await;

    update_run_state(&db, run.id, None, "waiting", Some("budget"), None)
        .await
        .expect("run must enter budget wait");
    let events = latest_events_by_thread(&db, thread.id, 100)
        .await
        .expect("run events must load");
    let waiting_event = events
        .iter()
        .find(|event| event.kind == "run_waiting")
        .expect("waiting lifecycle event must exist");

    assert_eq!(
        waiting_event
            .payload_json
            .as_ref()
            .and_then(|payload| payload.get("waiting_reason"))
            .and_then(serde_json::Value::as_str),
        Some("budget")
    );
}

#[tokio::test]
async fn run_claim_only_takes_queued_work_and_single_worker_wins() {
    let db = setup_db().await;
    let first_thread = create_test_thread(&db, "main").await;
    let second_thread = create_test_thread(&db, "canvas").await;
    let (_, first_run) = create_test_run(&db, first_thread.id, "user").await;
    let (_, second_run) = create_test_run(&db, second_thread.id, "user").await;
    let mut active: run::ActiveModel = first_run.clone().into();
    active.status = Set("running".to_string());
    active.worker_id = Set(Some("dead-worker".to_string()));
    active.lease_token = Set(Some("expired-token".to_string()));
    active.lease_expires_at = Set(Some(Utc::now().naive_utc() - Duration::seconds(1)));
    active
        .update(&db)
        .await
        .expect("expired running lease must be prepared");

    let (left, right) = tokio::join!(
        claim_runnable_run(&db, "worker-a"),
        claim_runnable_run(&db, "worker-b")
    );
    let claimed = [left, right]
        .into_iter()
        .filter_map(|result| result.expect("claim must succeed"))
        .collect::<Vec<_>>();
    assert_eq!(claimed.len(), 1);
    assert_eq!(claimed[0].id, second_run.id);

    let expired = run::Entity::find_by_id(first_run.id)
        .one(&db)
        .await
        .expect("run query must succeed")
        .expect("expired running run must exist");
    assert_eq!(expired.status, "running");
    assert_eq!(expired.worker_id.as_deref(), Some("dead-worker"));
}

#[tokio::test]
async fn inbox_claim_token_guards_release_supersede_and_consume() {
    let db = setup_db().await;
    let thread = create_test_thread(&db, "main").await;
    let (turn, run) = create_test_run(&db, thread.id, "user").await;
    let inbox = enqueue_test_inbox(&db, thread.id, turn.id, run.id).await;
    let claimed = claim_inbox(&db, Some(thread.id))
        .await
        .expect("claim must succeed")
        .expect("inbox must be claimable");
    assert_eq!(claimed.id, inbox.id);

    let error = release_inbox_claim(&db, inbox.id, "wrong-token", None)
        .await
        .expect_err("wrong token must be rejected");
    assert!(matches!(error, AgentDbError::Conflict(_)));
    let token = claimed
        .claim_token
        .as_deref()
        .expect("claim token must exist");
    let released = release_inbox_claim(&db, inbox.id, token, None)
        .await
        .expect("valid release must succeed");
    assert_eq!(released.status, "pending");
    assert!(released.claim_token.is_none());

    let claimed = claim_inbox(&db, Some(thread.id))
        .await
        .expect("second claim must succeed")
        .expect("released inbox must be claimable");
    let error = supersede_inbox(&db, inbox.id, Some("wrong-token"))
        .await
        .expect_err("wrong supersede token must be rejected");
    assert!(matches!(error, AgentDbError::Conflict(_)));
    let token = claimed
        .claim_token
        .as_deref()
        .expect("claim token must exist");
    let consumed = a4n_db::agent::consume_inbox(&db, inbox.id, token)
        .await
        .expect("valid consume must succeed");
    assert_eq!(consumed.status, "consumed");
    assert!(consumed.consumed_at.is_some());
}

#[tokio::test]
async fn expired_inbox_claim_is_reclaimed_once() {
    let db = setup_db().await;
    let thread = create_test_thread(&db, "main").await;
    let (turn, run) = create_test_run(&db, thread.id, "user").await;
    let inbox = enqueue_test_inbox(&db, thread.id, turn.id, run.id).await;
    let claimed = claim_inbox(&db, Some(thread.id))
        .await
        .expect("first claim must succeed")
        .expect("inbox must be claimable");
    let old_token = claimed.claim_token.clone().expect("claim token must exist");
    let mut active: inbox::ActiveModel = claimed.into();
    active.claim_expires_at = Set(Some(Utc::now().naive_utc() - Duration::seconds(1)));
    active
        .update(&db)
        .await
        .expect("inbox claim expiry must be updated");

    let (left, right) = tokio::join!(
        claim_inbox(&db, Some(thread.id)),
        claim_inbox(&db, Some(thread.id))
    );
    let reclaimed = [left, right]
        .into_iter()
        .filter_map(|result| result.expect("reclaim must succeed"))
        .collect::<Vec<_>>();
    assert_eq!(reclaimed.len(), 1);
    assert_eq!(reclaimed[0].id, inbox.id);
    assert_ne!(
        reclaimed[0].claim_token.as_deref(),
        Some(old_token.as_str())
    );
}

#[tokio::test]
async fn expired_run_recovery_rejects_live_lease_and_is_idempotent() {
    let db = setup_db().await;
    let thread = create_test_thread(&db, "main").await;
    let (_, run) = create_test_run(&db, thread.id, "user").await;
    let claimed = a4n_db::agent::claim_runnable_run(&db, "test-worker")
        .await
        .expect("run claim must succeed")
        .expect("run must be claimable");
    let error = recover_expired_run(&db, claimed.id, Some("worker disappeared"))
        .await
        .expect_err("live lease must not be recovered");
    assert!(matches!(error, AgentDbError::Conflict(_)));

    let mut active: run::ActiveModel = claimed.into();
    active.lease_expires_at = Set(Some(Utc::now().naive_utc() - Duration::seconds(1)));
    active
        .update(&db)
        .await
        .expect("lease expiry must be updated");
    let recovered = recover_expired_run(&db, run.id, Some("worker disappeared"))
        .await
        .expect("expired run must require recovery");
    assert_eq!(recovered.status, "recovery_required");
    assert!(!recovered.is_active);
    assert!(recovered.completed_at.is_some());
    let replayed = recover_expired_run(&db, run.id, Some("ignored on replay"))
        .await
        .expect("recovery replay must be idempotent");
    assert_eq!(replayed.status, "recovery_required");
}

#[tokio::test]
async fn expired_worker_cannot_commit_run_state() {
    let db = setup_db().await;
    let thread = create_test_thread(&db, "main").await;
    let (_, run) = create_test_run(&db, thread.id, "user").await;
    let claimed = claim_runnable_run(&db, "test-worker")
        .await
        .expect("run claim must succeed")
        .expect("run must be claimable");
    let token = claimed
        .lease_token
        .as_deref()
        .expect("claimed run must have a lease token")
        .to_string();
    let mut active: run::ActiveModel = claimed.into();
    active.lease_expires_at = Set(Some(Utc::now().naive_utc() - Duration::seconds(1)));
    active
        .update(&db)
        .await
        .expect("lease expiry must be updated");

    let error = update_run_state(&db, run.id, Some(&token), "completed", None, None)
        .await
        .expect_err("expired lease must not commit terminal state");
    assert!(matches!(error, AgentDbError::Conflict(_)));
    let unchanged = run::Entity::find_by_id(run.id)
        .one(&db)
        .await
        .expect("run query must succeed")
        .expect("run must exist");
    assert_eq!(unchanged.status, "running");
    assert!(unchanged.is_active);

    let error = renew_run_lease(&db, run.id, &token)
        .await
        .expect_err("expired lease must not renew itself");
    assert!(matches!(error, AgentDbError::Conflict(_)));
    let error = update_run_usage(&db, run.id, &token, json!({}))
        .await
        .expect_err("expired lease must not update usage");
    assert!(matches!(error, AgentDbError::Conflict(_)));
}

#[tokio::test]
async fn usage_rejects_invalid_runtime_snapshot_admission_state() {
    let db = setup_db().await;
    let thread = create_test_thread(&db, "main").await;
    let (_, run) = create_test_run(&db, thread.id, "user").await;
    let claimed = claim_runnable_run(&db, "test-worker")
        .await
        .expect("run claim must succeed")
        .expect("run must be claimable");
    let token = claimed.lease_token.clone().expect("lease token must exist");

    let mut malformed: run::ActiveModel = claimed.clone().into();
    malformed.usage_json = Set(json!({
        "runtime_snapshot_required": "yes"
    }));
    malformed
        .update(&db)
        .await
        .expect("malformed fixture must be persisted");
    let error = update_run_usage(&db, run.id, &token, json!({}))
        .await
        .expect_err("malformed admission marker must fail explicitly");
    assert!(matches!(error, AgentDbError::InvalidArgument(_)));

    let mut valid: run::ActiveModel = run::Entity::find_by_id(run.id)
        .one(&db)
        .await
        .expect("run query must succeed")
        .expect("run must exist")
        .into();
    valid.usage_json = Set(json!({
        "runtime_snapshot_required": false
    }));
    valid
        .update(&db)
        .await
        .expect("valid marker fixture must be persisted");
    let error = update_run_usage(&db, run.id, &token, json!(null))
        .await
        .expect_err("non-object usage must fail explicitly");
    assert!(matches!(error, AgentDbError::InvalidArgument(_)));
}

#[tokio::test]
async fn expired_running_scan_is_bounded_and_fenced_execution_rejects_expired_worker() {
    let db = setup_db().await;
    let thread = create_test_thread(&db, "main").await;
    let (_, run) = create_test_run(&db, thread.id, "user").await;
    let claimed = claim_runnable_run(&db, "test-worker")
        .await
        .expect("run claim must succeed")
        .expect("run must be claimable");
    let token = claimed.lease_token.clone().expect("lease token must exist");

    let found = expired_running_runs(&db, 10)
        .await
        .expect("expired run scan must succeed");
    assert!(found.is_empty(), "a live lease is not recoverable yet");
    assert!(matches!(
        expired_running_runs(&db, 0).await,
        Err(AgentDbError::InvalidArgument(_))
    ));

    let mut active: run::ActiveModel = claimed.into();
    active.lease_expires_at = Set(Some(Utc::now().naive_utc() - Duration::seconds(1)));
    active
        .update(&db)
        .await
        .expect("lease expiry must be updated");
    let found = expired_running_runs(&db, 1)
        .await
        .expect("expired run scan must succeed");
    assert_eq!(
        found.iter().map(|row| row.id).collect::<Vec<_>>(),
        vec![run.id]
    );

    let error = create_execution_for_run_lease(
        &db,
        AgentExecutionInput {
            run_id: run.id,
            execution_key: "expired-write".to_string(),
            execution_kind: "tool_call".to_string(),
            status: "prepared".to_string(),
            tool_name: Some("save_canvas_graph".to_string()),
            effect: Some("write".to_string()),
            replay_policy: Some("reconcile_required".to_string()),
            arguments_json: Some(json!({"graph_id": 1})),
            resource_preconditions_json: None,
        },
        &token,
    )
    .await
    .expect_err("expired worker must be fenced before creating execution");
    assert!(matches!(error, AgentDbError::Conflict(_)));
}

#[tokio::test]
async fn expired_worker_cannot_commit_execution_result() {
    let db = setup_db().await;
    let thread = create_test_thread(&db, "main").await;
    let (_, run) = create_test_run(&db, thread.id, "user").await;
    let claimed = claim_runnable_run(&db, "test-worker")
        .await
        .expect("run claim must succeed")
        .expect("run must be claimable");
    let token = claimed.lease_token.clone().expect("lease token must exist");
    let execution = create_execution_for_run_lease(
        &db,
        AgentExecutionInput {
            run_id: run.id,
            execution_key: "model-call".to_string(),
            execution_kind: "model_call".to_string(),
            status: "prepared".to_string(),
            tool_name: None,
            effect: Some("read".to_string()),
            replay_policy: Some("safe".to_string()),
            arguments_json: Some(json!({"prompt": "test"})),
            resource_preconditions_json: None,
        },
        &token,
    )
    .await
    .expect("execution must be prepared under the live lease");

    let mut active: run::ActiveModel = claimed.into();
    active.lease_expires_at = Set(Some(Utc::now().naive_utc() - Duration::seconds(1)));
    active
        .update(&db)
        .await
        .expect("lease expiry must be updated");

    let error = update_execution_for_run_lease(
        &db,
        run.id,
        &token,
        execution.id,
        "succeeded",
        Some(json!({"text": "stale"})),
        Some("stale".to_string()),
        None,
        None,
    )
    .await
    .expect_err("expired worker must not commit execution results");
    assert!(matches!(error, AgentDbError::Conflict(_)));
    let unchanged = execution::Entity::find_by_id(execution.id)
        .one(&db)
        .await
        .expect("execution query must succeed")
        .expect("execution must exist");
    assert_eq!(unchanged.status, "prepared");
    assert!(unchanged.result_json.is_none());
}

#[tokio::test]
async fn budget_wait_can_resume_with_a_persisted_budget_update() {
    let db = setup_db().await;
    let thread = create_test_thread(&db, "main").await;
    let (_, run) = create_test_run(&db, thread.id, "user").await;

    update_run_state(&db, run.id, None, "waiting", Some("budget"), None)
        .await
        .expect("run must enter budget wait");
    let budget = json!({"max_steps": 64});
    let resumed =
        update_run_state_with_budget(&db, run.id, None, "queued", None, None, budget.clone())
            .await
            .expect("budget update and requeue must succeed");

    assert_eq!(resumed.status, "queued");
    assert_eq!(resumed.waiting_reason, None);
    assert_eq!(resumed.budget_json, budget);
}

#[tokio::test]
async fn approval_is_consumed_only_after_execution_succeeds() {
    let db = setup_db().await;
    let thread = create_test_thread(&db, "main").await;
    let (_, run) = create_test_run(&db, thread.id, "user").await;
    let claimed = claim_runnable_run(&db, "test-worker")
        .await
        .expect("run claim must succeed")
        .expect("run must be claimable");
    let token = claimed.lease_token.clone().expect("lease token must exist");
    let execution = create_execution_for_run_lease(
        &db,
        AgentExecutionInput {
            run_id: run.id,
            execution_key: "approved-write".to_string(),
            execution_kind: "tool_call".to_string(),
            status: "prepared".to_string(),
            tool_name: Some("save_canvas_graph".to_string()),
            effect: Some("write".to_string()),
            replay_policy: Some("reconcile_required".to_string()),
            arguments_json: Some(json!({"graph_id": 1})),
            resource_preconditions_json: None,
        },
        &token,
    )
    .await
    .expect("write execution must be prepared");
    let (approval, _, _) = request_approval_for_run_lease(
        &db,
        &token,
        AgentApprovalInput {
            thread_id: thread.id,
            run_id: run.id,
            execution_id: execution.id,
            operation_digest: "approved-write-digest".to_string(),
            operation_json: json!({"tool": "save_canvas_graph"}),
            resource_preconditions_json: None,
            expires_at: None,
        },
    )
    .await
    .expect("approval request must be durable");
    let mut approval_active: approval::ActiveModel = approval.into();
    approval_active.status = Set("approved".to_string());
    approval_active.decided_at = Set(Some(Utc::now().naive_utc()));
    approval_active
        .update(&db)
        .await
        .expect("approval must be approved for the fixture");
    let resumed = update_run_state(&db, run.id, None, "queued", None, None)
        .await
        .expect("approved waiting run must requeue");
    assert_eq!(resumed.status, "queued");
    let reclaimed = claim_runnable_run(&db, "test-worker-2")
        .await
        .expect("run reclaim must succeed")
        .expect("run must be reclaimable");
    let resumed_token = reclaimed.lease_token.clone().expect("new lease must exist");

    update_execution_for_run_lease(
        &db,
        run.id,
        &resumed_token,
        execution.id,
        "dispatching",
        None,
        None,
        None,
        None,
    )
    .await
    .expect("approved execution must enter dispatching");
    let still_approved = approval::Entity::find()
        .filter(approval::Column::ExecutionId.eq(execution.id))
        .one(&db)
        .await
        .expect("approval query must succeed")
        .expect("approval must exist");
    assert_eq!(still_approved.status, "approved");
    assert!(still_approved.consumed_at.is_none());

    update_execution_for_run_lease(
        &db,
        run.id,
        &resumed_token,
        execution.id,
        "succeeded",
        Some(json!({"saved": true})),
        Some("saved".to_string()),
        None,
        None,
    )
    .await
    .expect("successful execution must consume approval");
    let consumed = approval::Entity::find()
        .filter(approval::Column::ExecutionId.eq(execution.id))
        .one(&db)
        .await
        .expect("consumed approval query must succeed")
        .expect("consumed approval must exist");
    assert_eq!(consumed.status, "consumed");
    assert!(consumed.consumed_at.is_some());
}

#[tokio::test]
async fn expired_approval_rejects_execution_and_requeues_run() {
    let db = setup_db().await;
    let thread = create_test_thread(&db, "main").await;
    let (_, run) = create_test_run(&db, thread.id, "user").await;
    let claimed = claim_runnable_run(&db, "test-worker")
        .await
        .expect("run claim must succeed")
        .expect("run must be claimable");
    let token = claimed.lease_token.clone().expect("lease token must exist");
    let execution = create_execution_for_run_lease(
        &db,
        AgentExecutionInput {
            run_id: run.id,
            execution_key: "expiring-write".to_string(),
            execution_kind: "tool_call".to_string(),
            status: "prepared".to_string(),
            tool_name: Some("save_canvas_graph".to_string()),
            effect: Some("write".to_string()),
            replay_policy: Some("reconcile_required".to_string()),
            arguments_json: Some(json!({"graph_id": 1})),
            resource_preconditions_json: None,
        },
        &token,
    )
    .await
    .expect("write execution must be prepared");
    request_approval_for_run_lease(
        &db,
        &token,
        AgentApprovalInput {
            thread_id: thread.id,
            run_id: run.id,
            execution_id: execution.id,
            operation_digest: "expiring-write-digest".to_string(),
            operation_json: json!({"tool": "save_canvas_graph"}),
            resource_preconditions_json: None,
            expires_at: Some(Utc::now().naive_utc() - Duration::seconds(1)),
        },
    )
    .await
    .expect("expired approval fixture must be persisted");

    let expired = expire_pending_approvals(&db, 10)
        .await
        .expect("expiry scan must succeed");
    assert_eq!(expired.len(), 1);
    assert_eq!(expired[0].status, "expired");
    let execution = execution::Entity::find_by_id(execution.id)
        .one(&db)
        .await
        .expect("execution query must succeed")
        .expect("execution must exist");
    assert_eq!(execution.status, "rejected");
    let run = run::Entity::find_by_id(run.id)
        .one(&db)
        .await
        .expect("run query must succeed")
        .expect("run must exist");
    assert_eq!(run.status, "queued");
    assert!(run.is_active);
    assert!(run.lease_token.is_none());
}

#[tokio::test]
async fn stale_run_lease_cannot_activate_turn_or_consume_inbox() {
    let db = setup_db().await;
    let thread = create_test_thread(&db, "main").await;
    let (turn, run) = create_test_run(&db, thread.id, "user").await;
    let inbox = enqueue_test_inbox(&db, thread.id, turn.id, run.id).await;
    let claimed_run = claim_runnable_run(&db, "test-worker")
        .await
        .expect("run claim must succeed")
        .expect("run must be claimable");
    let run_token = claimed_run
        .lease_token
        .clone()
        .expect("run lease must exist");
    let claimed_inbox = claim_inbox(&db, Some(thread.id))
        .await
        .expect("inbox claim must succeed")
        .expect("inbox must be claimable");
    let inbox_token = claimed_inbox
        .claim_token
        .clone()
        .expect("inbox claim token must exist");
    let mut active: run::ActiveModel = claimed_run.into();
    active.lease_expires_at = Set(Some(Utc::now().naive_utc() - Duration::seconds(1)));
    active
        .update(&db)
        .await
        .expect("run lease must expire for fixture");

    assert!(matches!(
        mark_turn_active_for_run_lease(&db, run.id, &run_token).await,
        Err(AgentDbError::Conflict(_))
    ));
    assert!(matches!(
        consume_inbox_with_message(
            &db,
            inbox.id,
            &inbox_token,
            &run_token,
            AgentMessageInput {
                thread_id: thread.id,
                turn_id: Some(turn.id),
                run_id: Some(run.id),
                role: "user".to_string(),
                sender_kind: "user".to_string(),
                sender_thread_id: None,
                client_message_id: Some(format!("agent-inbox:{}", inbox.id)),
                content: "后续消息".to_string(),
                status: "complete".to_string(),
                payload_json: None,
            },
        )
        .await,
        Err(AgentDbError::Conflict(_))
    ));
    let unchanged_turn = turn::Entity::find_by_id(turn.id)
        .one(&db)
        .await
        .expect("turn query must succeed")
        .expect("turn must exist");
    assert_eq!(unchanged_turn.status, "queued");
    let unchanged_inbox = inbox::Entity::find_by_id(inbox.id)
        .one(&db)
        .await
        .expect("inbox query must succeed")
        .expect("inbox must exist");
    assert_eq!(unchanged_inbox.status, "claimed");
}

#[tokio::test]
async fn finish_run_with_message_closes_message_turn_and_run_atomically() {
    let db = setup_db().await;
    let thread = create_test_thread(&db, "main").await;
    let (turn, run) = create_test_run(&db, thread.id, "user").await;
    let claimed = claim_runnable_run(&db, "test-worker")
        .await
        .expect("run claim must succeed")
        .expect("run must be claimable");
    let token = claimed.lease_token.expect("lease token must exist");

    let (completed, final_message) = finish_run_with_message(
        &db,
        run.id,
        &token,
        AgentMessageInput {
            thread_id: thread.id,
            turn_id: Some(turn.id),
            run_id: Some(run.id),
            role: "assistant".to_string(),
            sender_kind: "agent".to_string(),
            sender_thread_id: None,
            client_message_id: Some(format!("agent-final:{}", run.id)),
            content: "完成".to_string(),
            status: "complete".to_string(),
            payload_json: None,
        },
        Some(json!([{"kind": "chapter", "id": "1"}])),
        turn.input_message_id,
    )
    .await
    .expect("run completion must succeed")
    .expect("no new input should defer completion");

    assert_eq!(completed.status, "completed");
    assert!(!completed.is_active);
    let completed_turn = turn::Entity::find_by_id(turn.id)
        .one(&db)
        .await
        .expect("turn query must succeed")
        .expect("turn must exist");
    assert_eq!(completed_turn.status, "completed");
    assert_eq!(completed_turn.final_message_id, Some(final_message.id));
    let projected_thread = thread::Entity::find_by_id(thread.id)
        .one(&db)
        .await
        .expect("thread query must succeed")
        .expect("thread must exist");
    assert_eq!(projected_thread.status, "idle");
}

#[tokio::test]
async fn recursive_cancel_updates_runs_turns_delegation_and_inbox() {
    let db = setup_db().await;
    let parent_thread = create_test_thread(&db, "main").await;
    let child_thread = create_test_thread(&db, "canvas").await;
    let (parent_turn, parent_run) = create_test_run(&db, parent_thread.id, "user").await;
    let (child_turn, child_run) = create_test_run(&db, child_thread.id, "delegation").await;
    let delegation = create_delegation(
        &db,
        AgentDelegationInput {
            parent_thread_id: parent_thread.id,
            parent_run_id: parent_run.id,
            parent_execution_id: None,
            child_thread_id: child_thread.id,
            child_turn_id: None,
            child_run_id: None,
            delegation_key: "canvas-edit".to_string(),
            capability: "canvas".to_string(),
            objective: "调整画布".to_string(),
            success_criteria_json: json!([]),
            scope_json: json!({}),
            budget_json: json!({}),
            status: "pending".to_string(),
        },
    )
    .await
    .expect("delegation must be created");
    let delegation = bind_delegation_child(
        &db,
        delegation.id,
        parent_run.id,
        child_turn.id,
        child_run.id,
    )
    .await
    .expect("child binding must set lineage");
    let delegation = update_delegation(&db, delegation.id, "running", None, None, None)
        .await
        .expect("delegation must enter running state");
    let parent_claimed = claim_runnable_run(&db, "parent-worker")
        .await
        .expect("parent run claim must succeed")
        .expect("parent run must be claimable");
    assert_eq!(parent_claimed.id, parent_run.id);
    let child_claimed = claim_runnable_run(&db, "child-worker")
        .await
        .expect("child run claim must succeed")
        .expect("child run must be claimable");
    assert_eq!(child_claimed.id, child_run.id);
    let child_lease = child_claimed
        .lease_token
        .clone()
        .expect("child run lease token must exist");
    let execution = create_execution_for_run_lease(
        &db,
        AgentExecutionInput {
            run_id: child_run.id,
            execution_key: "canvas-save".to_string(),
            execution_kind: "tool_call".to_string(),
            status: "prepared".to_string(),
            tool_name: Some("save_canvas_graph".to_string()),
            effect: Some("write".to_string()),
            replay_policy: Some("reconcile_required".to_string()),
            arguments_json: Some(json!({"graph_id": 1})),
            resource_preconditions_json: None,
        },
        &child_lease,
    )
    .await
    .expect("execution must be created");
    let (approval, execution, _) = request_approval_for_run_lease(
        &db,
        &child_lease,
        AgentApprovalInput {
            thread_id: child_thread.id,
            run_id: child_run.id,
            execution_id: execution.id,
            operation_digest: "digest".to_string(),
            operation_json: json!({"command": "save_canvas_graph"}),
            resource_preconditions_json: None,
            expires_at: None,
        },
    )
    .await
    .expect("approval must be created");
    let child_inbox = enqueue_test_inbox(&db, child_thread.id, child_turn.id, child_run.id).await;
    let claimed = claim_inbox(&db, Some(child_thread.id))
        .await
        .expect("inbox claim must succeed")
        .expect("child inbox must be claimable");
    assert_eq!(claimed.id, child_inbox.id);

    let cancelled = cancel_run_tree(&db, parent_run.id, Some("user cancelled parent"))
        .await
        .expect("run tree cancellation must succeed");
    assert_eq!(cancelled.len(), 2);
    assert!(cancelled.iter().all(|row| row.status == "cancelled"));
    assert!(cancelled.iter().all(|row| !row.is_active));

    for turn_id in [parent_turn.id, child_turn.id] {
        let turn = turn::Entity::find_by_id(turn_id)
            .one(&db)
            .await
            .expect("turn query must succeed")
            .expect("turn must exist");
        assert_eq!(turn.status, "cancelled");
        assert!(turn.completed_at.is_some());
    }
    let delegation = delegation::Entity::find_by_id(delegation.id)
        .one(&db)
        .await
        .expect("delegation query must succeed")
        .expect("delegation must exist");
    assert_eq!(delegation.status, "cancelled");
    assert!(delegation.completed_at.is_some());
    let inbox = inbox::Entity::find_by_id(child_inbox.id)
        .one(&db)
        .await
        .expect("inbox query must succeed")
        .expect("inbox must exist");
    assert_eq!(inbox.status, "cancelled");
    assert!(inbox.claim_token.is_none());
    assert!(inbox.claim_expires_at.is_none());
    let execution = execution::Entity::find_by_id(execution.id)
        .one(&db)
        .await
        .expect("execution query must succeed")
        .expect("execution must exist");
    assert_eq!(execution.status, "cancelled");
    assert!(execution.completed_at.is_some());
    let approval = approval::Entity::find_by_id(approval.id)
        .one(&db)
        .await
        .expect("approval query must succeed")
        .expect("approval must exist");
    assert_eq!(approval.status, "rejected");
    assert!(approval.decided_at.is_some());

    for thread_id in [parent_thread.id, child_thread.id] {
        let thread = thread::Entity::find_by_id(thread_id)
            .one(&db)
            .await
            .expect("thread query must succeed")
            .expect("thread must exist");
        assert_eq!(thread.status, "idle");
    }

    let replayed = cancel_run_tree(&db, parent_run.id, Some("replay"))
        .await
        .expect("cancellation replay must be idempotent");
    assert_eq!(replayed.len(), 2);
}

#[tokio::test]
async fn delegation_status_contract_and_parent_key_lookup_are_enforced() {
    let db = setup_db().await;
    let parent_thread = create_test_thread(&db, "main").await;
    let child_thread = create_test_thread(&db, "optimize").await;
    let (_, parent_run) = create_test_run(&db, parent_thread.id, "user").await;
    let delegation = create_delegation(
        &db,
        AgentDelegationInput {
            parent_thread_id: parent_thread.id,
            parent_run_id: parent_run.id,
            parent_execution_id: None,
            child_thread_id: child_thread.id,
            child_turn_id: None,
            child_run_id: None,
            delegation_key: "latest-chapter-optimize".to_string(),
            capability: "optimize".to_string(),
            objective: "优化最新章节".to_string(),
            success_criteria_json: json!([]),
            scope_json: json!({}),
            budget_json: json!({}),
            status: "pending".to_string(),
        },
    )
    .await
    .expect("delegation must be created");
    let found = delegation_by_parent_and_key(&db, parent_run.id, "latest-chapter-optimize")
        .await
        .expect("delegation lookup must succeed")
        .expect("delegation must be found");
    assert_eq!(found.id, delegation.id);

    let error = update_delegation(&db, delegation.id, "rejected", None, None, None)
        .await
        .expect_err("unsupported delegation status must be rejected");
    assert!(matches!(error, AgentDbError::InvalidArgument(_)));
    let interrupted = update_delegation(
        &db,
        delegation.id,
        "interrupted",
        None,
        None,
        Some("host shutdown".to_string()),
    )
    .await
    .expect("interrupted must be a valid terminal status");
    assert!(interrupted.completed_at.is_some());
    let error = update_delegation(&db, delegation.id, "running", None, None, None)
        .await
        .expect_err("terminal delegation must not reopen");
    assert!(matches!(error, AgentDbError::Conflict(_)));
}

#[tokio::test]
async fn delegation_child_binding_is_single_assignment() {
    let db = setup_db().await;
    let parent_thread = create_test_thread(&db, "main").await;
    let child_thread = create_test_thread(&db, "canvas").await;
    let (_, parent_run) = create_test_run(&db, parent_thread.id, "user").await;
    let (child_turn, child_run) = create_test_run(&db, child_thread.id, "delegation").await;
    let delegation = create_delegation(
        &db,
        AgentDelegationInput {
            parent_thread_id: parent_thread.id,
            parent_run_id: parent_run.id,
            parent_execution_id: None,
            child_thread_id: child_thread.id,
            child_turn_id: None,
            child_run_id: None,
            delegation_key: "canvas-bind".to_string(),
            capability: "canvas".to_string(),
            objective: "绑定 Canvas 子 Agent".to_string(),
            success_criteria_json: json!([]),
            scope_json: json!({}),
            budget_json: json!({}),
            status: "pending".to_string(),
        },
    )
    .await
    .expect("delegation must be created");
    let bound = bind_delegation_child(
        &db,
        delegation.id,
        parent_run.id,
        child_turn.id,
        child_run.id,
    )
    .await
    .expect("child binding must succeed");
    assert_eq!(bound.child_turn_id, Some(child_turn.id));
    assert_eq!(bound.child_run_id, Some(child_run.id));

    let replayed = bind_delegation_child(
        &db,
        delegation.id,
        parent_run.id,
        child_turn.id,
        child_run.id,
    )
    .await
    .expect("same child binding must be idempotent");
    assert_eq!(replayed.id, delegation.id);

    let other_thread = create_test_thread(&db, "optimize").await;
    let (other_turn, other_run) = create_test_run(&db, other_thread.id, "delegation").await;
    let error = bind_delegation_child(
        &db,
        delegation.id,
        parent_run.id,
        other_turn.id,
        other_run.id,
    )
    .await
    .expect_err("delegation must not be rebound");
    assert!(matches!(error, AgentDbError::Conflict(_)));
}

#[tokio::test]
async fn user_delegation_creates_a_fenced_child_and_is_idempotent() {
    let db = setup_db().await;
    let parent_thread = create_test_thread(&db, "main").await;
    let child_thread = create_test_thread(&db, "research").await;
    let (_, parent_run) = create_test_run(&db, parent_thread.id, "user").await;
    let child_message_id = format!("user-delegation:{}", Uuid::new_v4());
    let input = AgentUserDelegationInput {
        delegation: AgentDelegationInput {
            parent_thread_id: parent_thread.id,
            parent_run_id: parent_run.id,
            parent_execution_id: None,
            child_thread_id: child_thread.id,
            child_turn_id: None,
            child_run_id: None,
            delegation_key: "user-research-once".into(),
            capability: "research".into(),
            objective: "调查世界观资料".into(),
            success_criteria_json: json!(["返回来源"]),
            scope_json: json!({"project_id": null}),
            budget_json: json!({}),
            status: "pending".into(),
        },
        child_turn: AgentTurnInput {
            thread_id: child_thread.id,
            client_message_id: child_message_id.clone(),
            message: AgentMessageInput {
                thread_id: child_thread.id,
                turn_id: None,
                run_id: None,
                role: "user".into(),
                sender_kind: "user".into(),
                sender_thread_id: None,
                client_message_id: Some(child_message_id),
                content: "调查世界观资料".into(),
                status: "complete".into(),
                payload_json: None,
            },
            execution_mode: "autonomous".into(),
            model_override_json: None,
            budget_json: json!({}),
            runtime_snapshot_required: false,
            trigger: "delegation".into(),
            goal: Some("调查世界观资料".into()),
        },
    };
    let (delegation, _, child_run, _) = create_user_delegation(&db, input.clone())
        .await
        .expect("user delegation must be created");
    assert_eq!(delegation.status, "running");
    assert_eq!(child_run.parent_run_id, Some(parent_run.id));
    assert_eq!(child_run.root_run_id, Some(parent_run.id));
    let parent = run::Entity::find_by_id(parent_run.id)
        .one(&db)
        .await
        .expect("parent lookup")
        .expect("parent exists");
    assert_eq!(parent.status, "waiting");
    assert_eq!(parent.waiting_reason.as_deref(), Some("child"));

    let (replayed, _, replayed_run, _) = create_user_delegation(&db, input.clone())
        .await
        .expect("same user delegation must replay");
    assert_eq!(replayed.id, delegation.id);
    assert_eq!(replayed_run.id, child_run.id);

    let mut conflicting_input = input.clone();
    conflicting_input.delegation.objective = "改写为另一项任务".into();
    let error = create_user_delegation(&db, conflicting_input)
        .await
        .expect_err("same delegation key with a different objective must conflict");
    assert!(matches!(error, AgentDbError::Conflict(_)));
}
