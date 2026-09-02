use a4n_core::models::agent::{approval, delegation, execution, inbox, run, thread, turn};
use a4n_db::agent::{
    AgentApprovalInput, AgentDbError, AgentDelegationInput, AgentExecutionInput, AgentInboxInput,
    AgentMessageInput, AgentThreadCreateInput, AgentTurnInput, AgentUserDelegationInput,
    append_message, bind_delegation_child, cancel_run_tree, claim_inbox, claim_runnable_run,
    consume_inbox_with_message, create_delegation, create_execution_for_run_lease, create_thread,
    create_turn_with_run, create_user_delegation, delegation_by_parent_and_key, enqueue_inbox,
    expire_pending_approvals, expired_running_runs, finish_run_with_message,
    mark_turn_active_for_run_lease, recover_expired_run, release_inbox_claim, renew_run_lease,
    request_approval_for_run_lease, supersede_inbox, update_delegation,
    update_execution_for_run_lease, update_run_state, update_run_usage,
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
    )
    .await
    .expect("run completion must succeed");

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
