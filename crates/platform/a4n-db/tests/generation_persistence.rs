use a4n_db::cleanup_outbox::{
    GenerationArtifactUnbindPayload, GenerationCleanupScope,
    insert_generation_artifact_unbind_intent,
};
use a4n_db::generation::{
    GenerationTaskPersistenceError, PersistGenerationCheckpointInput, ReserveGenerationTaskInput,
    begin_project_lifecycle_gate, bind_generation_task_to_workflow, canonical_json_sha256,
    expire_generation_snapshot_bodies, extend_generation_sequence_lease,
    list_active_generation_task_checkpoints, list_generation_task_history_checkpoints,
    load_generation_task, persist_generation_cancel_requested_batch, persist_generation_checkpoint,
    persist_generation_status_changed, persist_generation_terminal,
    purge_expired_generation_task_trees, reserve_generation_task, restore_project_lifecycle_gate,
};
use a4n_db::workflow::{
    ClaimExclusiveWorkflowRunInput, UpsertWorkflowRunInput, claim_exclusive_workflow_run,
    mark_workflow_status,
};
use a4n_db_models::models::{
    chapter::chapter,
    generation::generation_start_request,
    project::project_member,
    task::{task_dependency, task_queue},
};
use a4n_domain::generation::{
    GenerationTaskListCursor, LifecycleGateClaimState, ProjectLifecycleOperation,
};
use a4n_domain::workflow::{WORKFLOW_STATUS_COMPLETED, WORKFLOW_STATUS_FAILED};
use a4n_migration::test_support::connect_test_database;
use a4n_queue::priority::{TaskKind, TaskPriority};
use a4n_service_protocol::{
    DraftDisposition, GENERATION_EVENT_SCHEMA_VERSION, GenerationDeliveryMode, GenerationEvent,
    GenerationTaskEvent, GenerationTaskStatus, InterruptedEvent, InterruptionReason,
};
use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, DatabaseConnection, DbBackend, EntityTrait,
    QueryFilter, Set, Statement, TransactionTrait,
};
use serde_json::json;
use uuid::Uuid;

async fn connect_generation_test_database() -> DatabaseConnection {
    let db = connect_test_database().await;
    a4n_db::package::ensure_host_package_states(&db, &["ai-workflow".to_string()])
        .await
        .unwrap();
    db
}

fn reservation_input(
    user_id: i64,
    project_id: i64,
    start_request_id: String,
) -> ReserveGenerationTaskInput {
    ReserveGenerationTaskInput {
        requested_by_user_id: user_id,
        start_request_id,
        business_key_json: json!({
            "version": 1,
            "project_id": project_id,
            "operation": "generation-persistence-test"
        }),
        input_fingerprint_json: json!({"prompt": "真实 PostgreSQL reservation"}),
        task_kind: TaskKind::TextGeneration,
        priority: TaskPriority::High,
        delivery_mode: GenerationDeliveryMode::Stream,
        batch_policy: None,
        project_id: Some(project_id),
        root_project_id: None,
        source_snapshot_id: None,
        task_scope: Some("generation_persistence_test".to_string()),
        workflow_run_id: None,
        generation_run_id: None,
        target: None,
        root_task_id: None,
        parent_task_id: None,
        item_key: None,
        retry_of_task_id: None,
    }
}

fn list_reservation_input(
    user_id: i64,
    project_id: i64,
    operation: &str,
) -> ReserveGenerationTaskInput {
    let mut input = reservation_input(user_id, project_id, Uuid::new_v4().to_string());
    input.business_key_json = json!({
        "version": 1,
        "project_id": project_id,
        "operation": operation,
    });
    input.input_fingerprint_json = json!({"prompt": operation});
    input
}

async fn interrupt_generation_task(db: &DatabaseConnection, task_id: &str) {
    let loaded = load_generation_task(db, task_id).await.unwrap();
    let mut snapshot = loaded.public_snapshot.unwrap();
    snapshot.covers_through_sequence += 1;
    snapshot.status = GenerationTaskStatus::Interrupted;
    snapshot.updated_at_ms = Utc::now().timestamp_millis();
    snapshot.terminal_event = Some(GenerationTaskEvent {
        schema_version: GENERATION_EVENT_SCHEMA_VERSION,
        task_id: snapshot.task_id.clone(),
        attempt: snapshot.attempt,
        sequence: snapshot.covers_through_sequence,
        emitted_at_ms: snapshot.updated_at_ms,
        event: GenerationEvent::Interrupted(InterruptedEvent {
            reason: InterruptionReason::HostRestartRecovery,
            retryable: true,
            draft_disposition: DraftDisposition::RetainedUntilExpiry,
            summary: Some("generation list authorization test".to_string()),
            batch_summary: None,
        }),
    });
    persist_generation_terminal(
        db,
        a4n_db::generation::PersistGenerationLifecycleInput {
            task_id: snapshot.task_id.clone(),
            attempt: snapshot.attempt,
            expected_checkpoint_revision: loaded.checkpoint.checkpoint_revision,
            expected_status: GenerationTaskStatus::Reserved,
            public_snapshot: snapshot,
        },
    )
    .await
    .unwrap();
}

async fn set_generation_task_requester(
    db: &DatabaseConnection,
    task_id: &str,
    requested_by_user_id: i64,
) {
    db.execute(Statement::from_sql_and_values(
        DbBackend::Postgres,
        "UPDATE generation_task_checkpoint SET requested_by_user_id = $2 WHERE task_id = $1",
        [task_id.to_string().into(), requested_by_user_id.into()],
    ))
    .await
    .unwrap();
}

#[tokio::test]
async fn generation_reservation_resolves_derived_project_root() {
    let db = connect_generation_test_database().await;
    let suffix = Uuid::new_v4().simple().to_string();
    let user = a4n_db::auth::create_user(
        &db,
        &format!("generation_derived_{suffix}"),
        &format!("generation_derived_{suffix}@example.invalid"),
        "generation-test-password",
        "user",
    )
    .await
    .unwrap();
    let root = a4n_db::project::create_project_record(
        &db,
        a4n_db::project::CreateProjectRecordInput {
            user_id: user.id,
            title: format!("generation derived root {suffix}"),
            description: String::new(),
            project_type: "novel_long".to_string(),
        },
    )
    .await
    .unwrap();
    let now = Utc::now().naive_utc();
    let chapter = chapter::ActiveModel {
        project_id: Set(root.id),
        chapter_number: Set(1),
        title: Set(Some("第一章".to_string())),
        content: Set("章节正文".to_string()),
        word_count: Set(4),
        status: Set(chapter::STATUS_IMPORTED.to_string()),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(&db)
    .await
    .unwrap();
    let derived = a4n_db::project::create_derived_project_record(
        &db,
        user.id,
        a4n_db::project::CreateDerivedProjectParams {
            root_project_id: root.id,
            derived_kind: a4n_db::project::PROJECT_KIND_VIDEO.to_string(),
            title: format!("generation derived video {suffix}"),
            description: String::new(),
            chapter_scope: a4n_db::project::DerivedChapterScope {
                start_chapter: 1,
                end_chapter: 1,
            },
        },
    )
    .await
    .unwrap();

    let reserved = reserve_generation_task(
        &db,
        reservation_input(user.id, derived.id, Uuid::new_v4().to_string()),
    )
    .await
    .unwrap();
    let task_id = reserved.task.checkpoint.task_id.clone();
    assert_eq!(reserved.task.checkpoint.project_id, Some(derived.id));
    assert_eq!(reserved.task.checkpoint.root_project_id, Some(root.id));
    let snapshot = reserved.task.public_snapshot.unwrap();
    assert_eq!(snapshot.started.project_id, Some(derived.id));
    assert_eq!(snapshot.started.root_project_id, Some(root.id));

    let workflow_input = UpsertWorkflowRunInput {
        business_key: format!(
            "video_prompt:video_prompts:{}:{}:storyboard",
            derived.id, chapter.id
        ),
        workflow_kind: "video_prompts".to_string(),
        project_id: Some(derived.id),
        root_project_id: Some(root.id),
        source_snapshot_id: None,
        scope_table: Some("chapter".to_string()),
        scope_id: Some(chapter.id),
        chapter_id: Some(chapter.id),
        mode: Some("storyboard".to_string()),
        input_fingerprint_json: Some(json!({ "version": 1 })),
        status: "running".to_string(),
        current_step_key: Some("video_prompts".to_string()),
        active_task_id: Some(task_id.clone()),
        can_resume: false,
        resume_block_reason: None,
        error_message: None,
    };
    let workflow = claim_exclusive_workflow_run(
        &db,
        ClaimExclusiveWorkflowRunInput {
            run: workflow_input.clone(),
            resume_from_task_id: None,
        },
    )
    .await
    .unwrap();
    assert_eq!(workflow.root_project_id, Some(root.id));
    assert_eq!(workflow.active_task_id.as_deref(), Some(task_id.as_str()));
    let queue_task = task_queue::Entity::find_by_id(task_id.clone())
        .one(&db)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(queue_task.root_project_id, Some(root.id));
    assert_eq!(queue_task.workflow_run_id, Some(workflow.id));
    let bound_task = load_generation_task(&db, &task_id).await.unwrap();
    assert_eq!(bound_task.checkpoint.workflow_run_id, Some(workflow.id));
    assert_eq!(
        bound_task.public_snapshot.unwrap().started.workflow_run_id,
        Some(workflow.id)
    );

    let mut unlinked_task: task_queue::ActiveModel = queue_task.into();
    unlinked_task.workflow_run_id = Set(None);
    unlinked_task.update(&db).await.unwrap();
    let tx = db.begin().await.unwrap();
    bind_generation_task_to_workflow(&tx, &task_id, workflow.id)
        .await
        .unwrap();
    tx.commit().await.unwrap();
    let repaired_task = task_queue::Entity::find_by_id(task_id.clone())
        .one(&db)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(repaired_task.workflow_run_id, Some(workflow.id));

    mark_workflow_status(&db, workflow.id, WORKFLOW_STATUS_COMPLETED, false, None)
        .await
        .unwrap();
    let next_workflow = claim_exclusive_workflow_run(
        &db,
        ClaimExclusiveWorkflowRunInput {
            run: workflow_input.clone(),
            resume_from_task_id: None,
        },
    )
    .await
    .unwrap();
    assert_ne!(next_workflow.id, workflow.id);
    let queue_task = task_queue::Entity::find_by_id(task_id.clone())
        .one(&db)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(queue_task.root_project_id, Some(root.id));
    assert_eq!(queue_task.workflow_run_id, Some(next_workflow.id));

    mark_workflow_status(
        &db,
        next_workflow.id,
        WORKFLOW_STATUS_FAILED,
        true,
        Some("retry test"),
    )
    .await
    .unwrap();
    interrupt_generation_task(&db, &task_id).await;
    let mut retry_input = reservation_input(user.id, derived.id, Uuid::new_v4().to_string());
    retry_input.retry_of_task_id = Some(task_id.clone());
    let retry = reserve_generation_task(&db, retry_input).await.unwrap();
    let retry_task_id = retry.task.checkpoint.task_id;
    let mut retry_workflow_input = workflow_input;
    retry_workflow_input.active_task_id = Some(retry_task_id.clone());
    let resumed = claim_exclusive_workflow_run(
        &db,
        ClaimExclusiveWorkflowRunInput {
            run: retry_workflow_input,
            resume_from_task_id: Some(task_id.clone()),
        },
    )
    .await
    .unwrap();
    assert_eq!(resumed.id, next_workflow.id);
    assert_eq!(
        resumed.active_task_id.as_deref(),
        Some(retry_task_id.as_str())
    );
    let retry_queue_task = task_queue::Entity::find_by_id(retry_task_id)
        .one(&db)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(retry_queue_task.workflow_run_id, Some(next_workflow.id));

    let mut mismatched = reservation_input(user.id, derived.id, Uuid::new_v4().to_string());
    mismatched.root_project_id = Some(derived.id);
    let error = reserve_generation_task(&db, mismatched).await.unwrap_err();
    assert!(matches!(
        error,
        GenerationTaskPersistenceError::InvalidArgument(message)
            if message.contains("规范 root")
    ));
}

#[tokio::test]
async fn generation_persistence_reservation_lease_checkpoint_and_gate_use_postgres() {
    let db = connect_generation_test_database().await;
    let suffix = Uuid::new_v4().simple().to_string();
    let user = a4n_db::auth::create_user(
        &db,
        &format!("generation_{suffix}"),
        &format!("generation_{suffix}@example.invalid"),
        "generation-test-password",
        "user",
    )
    .await
    .unwrap();
    let project = a4n_db::project::create_canvas_project_record(
        &db,
        user.id,
        a4n_db::project::CreateCanvasProjectParams {
            title: format!("generation persistence {suffix}"),
            description: String::new(),
        },
    )
    .await
    .unwrap();

    let start_request_id = Uuid::new_v4().to_string();
    let input = reservation_input(user.id, project.id, start_request_id.clone());
    let first = reserve_generation_task(&db, input.clone()).await.unwrap();
    assert!(first.created);
    assert_eq!(first.task.checkpoint.status, "reserved");
    assert_eq!(first.task.checkpoint.covers_through_sequence, 1);
    assert_eq!(first.task.checkpoint.reserved_through_sequence, 257);
    assert!(matches!(
        first.queue_task.status,
        a4n_queue::priority::TaskStatus::Pending
    ));

    let second = reserve_generation_task(&db, input.clone()).await.unwrap();
    assert!(!second.created);
    assert_eq!(
        second.task.checkpoint.task_id,
        first.task.checkpoint.task_id
    );

    let alias = reservation_input(user.id, project.id, Uuid::new_v4().to_string());
    let aliased = reserve_generation_task(&db, alias).await.unwrap();
    assert!(!aliased.created);
    let mappings = generation_start_request::Entity::find()
        .filter(generation_start_request::Column::TaskId.eq(first.task.checkpoint.task_id.clone()))
        .all(&db)
        .await
        .unwrap();
    assert_eq!(mappings.len(), 2);

    let lease = extend_generation_sequence_lease(
        &db,
        &first.task.checkpoint.task_id,
        first.task.checkpoint.checkpoint_revision,
        first.task.checkpoint.reserved_through_sequence,
    )
    .await
    .unwrap();
    assert_eq!(lease.checkpoint_revision, 2);
    assert_eq!(lease.reserved_through_sequence, 513);

    let mut snapshot = first.task.public_snapshot.clone().unwrap();
    snapshot.updated_at_ms = Utc::now().timestamp_millis();
    let execution = json!({"cursor": "opaque", "page": 1});
    let checkpoint = persist_generation_checkpoint(
        &db,
        PersistGenerationCheckpointInput {
            task_id: first.task.checkpoint.task_id.clone(),
            attempt: 1,
            expected_checkpoint_revision: lease.checkpoint_revision,
            public_snapshot: snapshot.clone(),
            step_key: Some("provider_wait".to_string()),
            call_key: Some("call-1".to_string()),
            provider_kind: Some("video".to_string()),
            provider_model: Some("test".to_string()),
            provider_job_id: Some("job-1".to_string()),
            provider_idempotency_key_sha256: Some("a".repeat(64)),
            resume_metadata_version: Some(1),
            execution_checkpoint_json: Some(execution.clone()),
        },
    )
    .await
    .unwrap();
    assert_eq!(checkpoint.checkpoint_revision, 3);
    assert_eq!(checkpoint.reserved_through_sequence, 513);

    let stale = persist_generation_checkpoint(
        &db,
        PersistGenerationCheckpointInput {
            task_id: first.task.checkpoint.task_id.clone(),
            attempt: 1,
            expected_checkpoint_revision: lease.checkpoint_revision,
            public_snapshot: snapshot,
            step_key: None,
            call_key: None,
            provider_kind: None,
            provider_model: None,
            provider_job_id: None,
            provider_idempotency_key_sha256: None,
            resume_metadata_version: None,
            execution_checkpoint_json: None,
        },
    )
    .await
    .unwrap_err();
    assert!(matches!(
        stale,
        GenerationTaskPersistenceError::CheckpointConflict
    ));

    let loaded = load_generation_task(&db, &first.task.checkpoint.task_id)
        .await
        .unwrap();
    assert_eq!(loaded.checkpoint.provider_job_id.as_deref(), Some("job-1"));
    assert_eq!(
        loaded.checkpoint.execution_checkpoint_sha256.as_deref(),
        Some(canonical_json_sha256(&execution).unwrap().as_str())
    );
    assert_eq!(loaded.checkpoint.execution_checkpoint_json, Some(execution));
    let mut running_snapshot = loaded.public_snapshot.unwrap();
    assert_eq!(running_snapshot.status, GenerationTaskStatus::Reserved);
    running_snapshot.covers_through_sequence += 1;
    running_snapshot.status = GenerationTaskStatus::Running;
    running_snapshot.updated_at_ms = Utc::now().timestamp_millis();
    persist_generation_status_changed(
        &db,
        a4n_db::generation::PersistGenerationLifecycleInput {
            task_id: running_snapshot.task_id.clone(),
            attempt: running_snapshot.attempt,
            expected_checkpoint_revision: loaded.checkpoint.checkpoint_revision,
            expected_status: GenerationTaskStatus::Reserved,
            public_snapshot: running_snapshot,
        },
    )
    .await
    .unwrap();
    let running_queue = task_queue::Entity::find_by_id(first.task.checkpoint.task_id.clone())
        .one(&db)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(running_queue.status, "running");
    assert!(running_queue.started_at.is_some());
    let claim =
        begin_project_lifecycle_gate(&db, project.id, user.id, ProjectLifecycleOperation::Reset)
            .await
            .unwrap();
    assert!(matches!(claim.state, LifecycleGateClaimState::Started));
    let blocked = reserve_generation_task(
        &db,
        reservation_input(user.id, project.id, Uuid::new_v4().to_string()),
    )
    .await
    .unwrap_err();
    assert!(matches!(
        blocked,
        GenerationTaskPersistenceError::Rejected {
            code: "generation_owner_lifecycle_blocked",
            ..
        }
    ));
    restore_project_lifecycle_gate(&db, project.id, ProjectLifecycleOperation::Reset)
        .await
        .unwrap();

    let loaded = load_generation_task(&db, &first.task.checkpoint.task_id)
        .await
        .unwrap();
    let mut terminal_snapshot = loaded.public_snapshot.unwrap();
    terminal_snapshot.covers_through_sequence += 1;
    terminal_snapshot.status = GenerationTaskStatus::Interrupted;
    terminal_snapshot.updated_at_ms = Utc::now().timestamp_millis();
    terminal_snapshot.terminal_event = Some(GenerationTaskEvent {
        schema_version: GENERATION_EVENT_SCHEMA_VERSION,
        task_id: terminal_snapshot.task_id.clone(),
        attempt: terminal_snapshot.attempt,
        sequence: terminal_snapshot.covers_through_sequence,
        emitted_at_ms: terminal_snapshot.updated_at_ms,
        event: GenerationEvent::Interrupted(InterruptedEvent {
            reason: InterruptionReason::HostRestartRecovery,
            retryable: true,
            draft_disposition: DraftDisposition::RetainedUntilExpiry,
            summary: Some("TTL persistence test".to_string()),
            batch_summary: None,
        }),
    });
    let terminal_hash =
        canonical_json_sha256(&serde_json::to_value(&terminal_snapshot).unwrap()).unwrap();
    let terminal = persist_generation_terminal(
        &db,
        a4n_db::generation::PersistGenerationLifecycleInput {
            task_id: terminal_snapshot.task_id.clone(),
            attempt: terminal_snapshot.attempt,
            expected_checkpoint_revision: loaded.checkpoint.checkpoint_revision,
            expected_status: GenerationTaskStatus::Running,
            public_snapshot: terminal_snapshot,
        },
    )
    .await
    .unwrap();
    let interrupted_queue = task_queue::Entity::find_by_id(first.task.checkpoint.task_id.clone())
        .one(&db)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(interrupted_queue.status, "interrupted");
    assert_eq!(
        interrupted_queue.error_message.as_deref(),
        Some("TTL persistence test")
    );
    db.execute(Statement::from_sql_and_values(
        DbBackend::Postgres,
        "UPDATE generation_task_checkpoint SET expires_at = NOW() WHERE task_id = $1",
        [first.task.checkpoint.task_id.clone().into()],
    ))
    .await
    .unwrap();
    assert_eq!(
        expire_generation_snapshot_bodies(&db, 100).await.unwrap(),
        1
    );
    let expired = load_generation_task(&db, &first.task.checkpoint.task_id)
        .await
        .unwrap();
    assert!(expired.public_snapshot.is_none());
    assert_eq!(
        expired.checkpoint.public_snapshot_sha256,
        Some(terminal_hash)
    );
    assert_eq!(
        expired.checkpoint.checkpoint_revision,
        terminal.checkpoint_revision + 1
    );
    assert!(
        generation_start_request::Entity::find()
            .filter(
                generation_start_request::Column::TaskId.eq(first.task.checkpoint.task_id.clone())
            )
            .all(&db)
            .await
            .unwrap()
            .len()
            == 2
    );
    assert!(
        a4n_db_models::models::task::task_queue::Entity::find_by_id(first.task.checkpoint.task_id)
            .one(&db)
            .await
            .unwrap()
            .is_some()
    );
    assert_eq!(
        expire_generation_snapshot_bodies(&db, 100).await.unwrap(),
        0
    );
}

#[tokio::test]
async fn generation_cancel_batch_updates_checkpoints_and_queue_rows() {
    let db = connect_generation_test_database().await;
    let suffix = Uuid::new_v4().simple().to_string();
    let user = a4n_db::auth::create_user(
        &db,
        &format!("cancel_batch_{suffix}"),
        &format!("cancel_batch_{suffix}@example.invalid"),
        "generation-test-password",
        "user",
    )
    .await
    .unwrap();
    let project = a4n_db::project::create_canvas_project_record(
        &db,
        user.id,
        a4n_db::project::CreateCanvasProjectParams {
            title: format!("generation cancel batch {suffix}"),
            description: String::new(),
        },
    )
    .await
    .unwrap();

    let mut reservations = Vec::new();
    for operation in ["cancel-batch-a", "cancel-batch-b"] {
        let reserved =
            reserve_generation_task(&db, list_reservation_input(user.id, project.id, operation))
                .await
                .unwrap();
        reservations.push((
            reserved.task.checkpoint.task_id,
            reserved.task.checkpoint.checkpoint_revision,
            reserved.task.public_snapshot.unwrap(),
        ));
    }
    reservations.sort_by(|left, right| left.0.cmp(&right.0));

    let expected_revisions = reservations
        .iter()
        .map(|(task_id, revision, snapshot)| {
            let mut snapshot = snapshot.clone();
            snapshot.covers_through_sequence += 1;
            snapshot.status = GenerationTaskStatus::Cancelling;
            snapshot.cancel_reason = Some(a4n_service_protocol::CancellationReason::UserRequested);
            snapshot.updated_at_ms = Utc::now().timestamp_millis();
            (
                task_id.clone(),
                *revision,
                a4n_db::generation::PersistGenerationLifecycleInput {
                    task_id: task_id.clone(),
                    attempt: snapshot.attempt,
                    expected_checkpoint_revision: *revision,
                    expected_status: GenerationTaskStatus::Reserved,
                    public_snapshot: snapshot,
                },
            )
        })
        .collect::<Vec<_>>();
    let inputs = expected_revisions
        .iter()
        .map(|(_, _, input)| input.clone())
        .collect::<Vec<_>>();

    let persisted = persist_generation_cancel_requested_batch(&db, inputs)
        .await
        .unwrap();
    assert_eq!(persisted.len(), expected_revisions.len());
    for ((task_id, revision, _), persisted) in expected_revisions.iter().zip(persisted) {
        assert_eq!(persisted.checkpoint_revision, revision + 1);
        let loaded = load_generation_task(&db, task_id).await.unwrap();
        assert_eq!(loaded.checkpoint.status, "cancelling");
        assert_eq!(loaded.checkpoint.checkpoint_revision, revision + 1);
        let queue = task_queue::Entity::find_by_id(task_id.clone())
            .one(&db)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(queue.status, "running");
        assert!(queue.completed_at.is_none());
    }
}

#[tokio::test]
async fn generation_task_tree_ttl_waits_for_descendants_and_audit_before_deleting() {
    let db = connect_generation_test_database().await;
    let suffix = Uuid::new_v4().simple().to_string();
    let user = a4n_db::auth::create_user(
        &db,
        &format!("tree_{suffix}"),
        &format!("tree_{suffix}@example.invalid"),
        "generation-test-password",
        "user",
    )
    .await
    .unwrap();
    let project = a4n_db::project::create_canvas_project_record(
        &db,
        user.id,
        a4n_db::project::CreateCanvasProjectParams {
            title: format!("generation tree {suffix}"),
            description: String::new(),
        },
    )
    .await
    .unwrap();

    let root = reserve_generation_task(
        &db,
        reservation_input(user.id, project.id, Uuid::new_v4().to_string()),
    )
    .await
    .unwrap();
    let root_task_id = root.task.checkpoint.task_id.clone();
    let mut child_input = reservation_input(user.id, project.id, Uuid::new_v4().to_string());
    child_input.business_key_json = json!({
        "version": 1,
        "project_id": project.id,
        "operation": "generation-persistence-child"
    });
    child_input.input_fingerprint_json = json!({"prompt": "child"});
    child_input.root_task_id = Some(root_task_id.clone());
    child_input.parent_task_id = Some(root_task_id.clone());
    child_input.item_key = Some("child-1".to_string());
    let child = reserve_generation_task(&db, child_input).await.unwrap();
    let child_task_id = child.task.checkpoint.task_id.clone();

    db.execute(Statement::from_sql_and_values(
        DbBackend::Postgres,
        "INSERT INTO task_dependency (task_id, depends_on_task_id) VALUES ($1, $2)",
        [child_task_id.clone().into(), root_task_id.clone().into()],
    ))
    .await
    .unwrap();
    db.execute(Statement::from_sql_and_values(
        DbBackend::Postgres,
        r#"
UPDATE generation_task_checkpoint
SET status = 'completed',
    created_at = NOW() - INTERVAL '3 hours',
    updated_at = NOW() - INTERVAL '2 hours',
    expires_at = CASE WHEN task_id = $1 THEN NOW() - INTERVAL '1 hour' ELSE NOW() + INTERVAL '1 day' END
WHERE task_id IN ($1, $2)
"#,
        [root_task_id.clone().into(), child_task_id.clone().into()],
    ))
    .await
    .unwrap();
    db.execute(Statement::from_sql_and_values(
        DbBackend::Postgres,
        "UPDATE task_queue SET status = 'completed', completed_at = NOW() WHERE id IN ($1, $2)",
        [root_task_id.clone().into(), child_task_id.clone().into()],
    ))
    .await
    .unwrap();

    // root 已到期但 child 仍保留，整棵树不能提前删。
    assert_eq!(
        purge_expired_generation_task_trees(&db, 100).await.unwrap(),
        0
    );
    assert!(
        task_queue::Entity::find_by_id(root_task_id.clone())
            .one(&db)
            .await
            .unwrap()
            .is_some()
    );

    db.execute(Statement::from_sql_and_values(
        DbBackend::Postgres,
        "UPDATE generation_task_checkpoint SET expires_at = NOW() - INTERVAL '1 hour' WHERE task_id IN ($1, $2)",
        [root_task_id.clone().into(), child_task_id.clone().into()],
    ))
    .await
    .unwrap();
    let audit = insert_generation_artifact_unbind_intent(
        &db,
        format!("generation-tree-audit-{suffix}"),
        &GenerationCleanupScope {
            requested_by_user_id: user.id,
            project_id: Some(project.id),
            task_id: child_task_id.clone(),
            attempt: 1,
        },
        GenerationArtifactUnbindPayload {
            version: 1,
            artifact_id: "artifact-tree".to_string(),
            revision: "revision-1".to_string(),
        },
    )
    .await
    .unwrap();

    // cleanup 审计未满 30 天时仍不能删除。
    assert_eq!(
        purge_expired_generation_task_trees(&db, 100).await.unwrap(),
        0
    );
    assert!(
        generation_start_request::Entity::find()
            .filter(generation_start_request::Column::TaskId.eq(child_task_id.clone()))
            .one(&db)
            .await
            .unwrap()
            .is_some()
    );

    db.execute(Statement::from_sql_and_values(
        DbBackend::Postgres,
        r#"
UPDATE project_cleanup_outbox
SET status = 'completed',
    outcome = 'unbound',
    completed_at = NOW() - INTERVAL '31 days',
    updated_at = NOW() - INTERVAL '31 days'
WHERE id = $1
"#,
        [audit.id.into()],
    ))
    .await
    .unwrap();
    assert_eq!(
        purge_expired_generation_task_trees(&db, 100).await.unwrap(),
        2
    );
    assert!(
        generation_start_request::Entity::find()
            .filter(
                generation_start_request::Column::TaskId
                    .is_in([root_task_id.clone(), child_task_id.clone()]),
            )
            .all(&db)
            .await
            .unwrap()
            .is_empty()
    );
    assert!(
        task_dependency::Entity::find()
            .filter(task_dependency::Column::TaskId.eq(child_task_id.clone()))
            .one(&db)
            .await
            .unwrap()
            .is_none()
    );
    assert!(
        task_queue::Entity::find_by_id(root_task_id.clone())
            .one(&db)
            .await
            .unwrap()
            .is_none()
    );
    assert!(
        task_queue::Entity::find_by_id(child_task_id.clone())
            .one(&db)
            .await
            .unwrap()
            .is_none()
    );
    assert!(load_generation_task(&db, &root_task_id).await.is_err());
    assert!(load_generation_task(&db, &child_task_id).await.is_err());
    assert!(
        a4n_db_models::models::task::cleanup_outbox::Entity::find_by_id(audit.id)
            .one(&db)
            .await
            .unwrap()
            .is_some()
    );
}

#[tokio::test]
async fn generation_task_lists_filter_access_before_cursor_and_limit() {
    let db = connect_generation_test_database().await;
    let suffix = Uuid::new_v4().simple().to_string();
    let owner = a4n_db::auth::create_user(
        &db,
        &format!("list_owner_{suffix}"),
        &format!("list_owner_{suffix}@example.invalid"),
        "generation-test-password",
        "user",
    )
    .await
    .unwrap();
    let member = a4n_db::auth::create_user(
        &db,
        &format!("list_member_{suffix}"),
        &format!("list_member_{suffix}@example.invalid"),
        "generation-test-password",
        "user",
    )
    .await
    .unwrap();
    let outsider = a4n_db::auth::create_user(
        &db,
        &format!("list_outsider_{suffix}"),
        &format!("list_outsider_{suffix}@example.invalid"),
        "generation-test-password",
        "user",
    )
    .await
    .unwrap();
    let shared_project = a4n_db::project::create_canvas_project_record(
        &db,
        owner.id,
        a4n_db::project::CreateCanvasProjectParams {
            title: format!("generation list shared {suffix}"),
            description: String::new(),
        },
    )
    .await
    .unwrap();
    let private_project = a4n_db::project::create_canvas_project_record(
        &db,
        outsider.id,
        a4n_db::project::CreateCanvasProjectParams {
            title: format!("generation list private {suffix}"),
            description: String::new(),
        },
    )
    .await
    .unwrap();

    let now = Utc::now().naive_utc();
    project_member::ActiveModel {
        project_id: Set(shared_project.id),
        user_id: Set(member.id),
        role: Set("viewer".to_string()),
        invited_by_user_id: Set(owner.id),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(&db)
    .await
    .unwrap();

    let visible_active = reserve_generation_task(
        &db,
        list_reservation_input(owner.id, shared_project.id, "active-visible"),
    )
    .await
    .unwrap();
    // reservation 写侧要求 root owner；这里仅改 checkpoint owner 来构造跨用户列表夹具，
    // 验证 list 查询不会把 task 行本身当成项目访问凭证。
    set_generation_task_requester(&db, &visible_active.task.checkpoint.task_id, member.id).await;
    let owner_active = reserve_generation_task(
        &db,
        list_reservation_input(owner.id, shared_project.id, "active-owner"),
    )
    .await
    .unwrap();
    let private_active = reserve_generation_task(
        &db,
        list_reservation_input(outsider.id, private_project.id, "active-private"),
    )
    .await
    .unwrap();
    set_generation_task_requester(&db, &private_active.task.checkpoint.task_id, member.id).await;

    let active = list_active_generation_task_checkpoints(&db, member.id, Some(shared_project.id))
        .await
        .unwrap();
    assert_eq!(active.items.len(), 1);
    assert_eq!(
        active.items[0].task_id,
        visible_active.task.checkpoint.task_id
    );
    assert!(
        !active
            .items
            .iter()
            .any(|item| item.task_id == owner_active.task.checkpoint.task_id)
    );
    assert!(
        !active
            .items
            .iter()
            .any(|item| item.task_id == private_active.task.checkpoint.task_id)
    );

    let mut history_ids = Vec::new();
    for operation in ["history-a", "history-b", "history-c"] {
        let task = reserve_generation_task(
            &db,
            list_reservation_input(owner.id, shared_project.id, operation),
        )
        .await
        .unwrap();
        set_generation_task_requester(&db, &task.task.checkpoint.task_id, member.id).await;
        history_ids.push(task.task.checkpoint.task_id.clone());
        interrupt_generation_task(&db, &task.task.checkpoint.task_id).await;
    }
    let private_history = reserve_generation_task(
        &db,
        list_reservation_input(outsider.id, private_project.id, "history-private"),
    )
    .await
    .unwrap();
    set_generation_task_requester(&db, &private_history.task.checkpoint.task_id, member.id).await;
    interrupt_generation_task(&db, &private_history.task.checkpoint.task_id).await;

    db.execute(Statement::from_sql_and_values(
        DbBackend::Postgres,
        "UPDATE generation_task_checkpoint SET updated_at = NOW() WHERE task_id IN ($1, $2, $3)",
        history_ids
            .iter()
            .cloned()
            .map(Into::into)
            .collect::<Vec<_>>(),
    ))
    .await
    .unwrap();

    let first_page =
        list_generation_task_history_checkpoints(&db, member.id, Some(shared_project.id), 2, None)
            .await
            .unwrap();
    assert_eq!(first_page.items.len(), 2);
    assert!(first_page.has_more);
    assert!(first_page.items[0].updated_at >= first_page.items[1].updated_at);
    assert!(
        first_page
            .items
            .iter()
            .all(|item| history_ids.contains(&item.task_id))
    );
    assert!(
        !first_page
            .items
            .iter()
            .any(|item| item.task_id == private_history.task.checkpoint.task_id)
    );

    let cursor = GenerationTaskListCursor {
        updated_at_ms: first_page.items[1].updated_at.and_utc().timestamp_millis(),
        task_id: first_page.items[1].task_id.clone(),
    };
    let second_page = list_generation_task_history_checkpoints(
        &db,
        member.id,
        Some(shared_project.id),
        2,
        Some(cursor),
    )
    .await
    .unwrap();
    assert_eq!(second_page.items.len(), 1);
    assert!(!second_page.has_more);
    assert!(
        !first_page
            .items
            .iter()
            .any(|item| item.task_id == second_page.items[0].task_id)
    );
    assert!(history_ids.contains(&second_page.items[0].task_id));
}
