use a4n_db::package::{
    HOST_STATE_DISABLED, HOST_STATE_ENABLED, OPERATION_COMPLETED, OPERATION_DRAINING,
    ensure_host_package_states, finish_package_lifecycle, host_package_state,
    package_admission_block, package_workload_snapshot, start_package_lifecycle,
    update_package_lifecycle_progress,
};
use a4n_db::queue::{QueueDbError, insert_task_queue_record};
use a4n_db_models::models::{generation::generation_task_checkpoint, task::task_queue};
use a4n_domain::task_queue::PersistTaskQueueRecordInput;
use a4n_error::AppError;
use a4n_migration::test_support::connect_test_database;
use chrono::Utc;
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, Set};
use serde_json::json;
use uuid::Uuid;

async fn export_state(
    db: &sea_orm::DatabaseConnection,
) -> a4n_db_models::models::package::host_package_state::Model {
    host_package_state(db, "export")
        .await
        .expect("读取 export 功能包状态")
        .expect("export 功能包状态必须存在")
}

fn queue_record(task_id: &str, task_type: &str) -> PersistTaskQueueRecordInput {
    PersistTaskQueueRecordInput {
        task_id: task_id.to_string(),
        task_type: task_type.to_string(),
        priority: "low".to_string(),
        project_id: None,
        root_project_id: None,
        source_snapshot_id: None,
        task_scope: None,
        workflow_run_id: None,
    }
}

#[tokio::test]
async fn package_disable_is_idempotent_and_versioned() {
    let db = connect_test_database().await;
    ensure_host_package_states(&db, &["export".to_string()])
        .await
        .expect("初始化功能包状态");
    let current = export_state(&db).await;
    let operation_id = Uuid::new_v4().to_string();

    let operation = start_package_lifecycle(
        &db,
        &operation_id,
        "export",
        "disable",
        None,
        7,
        current.status_version,
    )
    .await
    .expect("创建停用操作");
    assert_eq!(operation.state, OPERATION_DRAINING);
    assert_eq!(export_state(&db).await.host_state, "disabling");

    let retry =
        start_package_lifecycle(&db, &operation_id, "export", "disable", None, 99, i64::MAX)
            .await
            .expect("相同 operation_id 应幂等返回");
    assert_eq!(retry, operation);

    let stale = start_package_lifecycle(
        &db,
        &Uuid::new_v4().to_string(),
        "export",
        "disable",
        None,
        7,
        current.status_version,
    )
    .await
    .expect_err("旧状态版本不得覆盖停用操作");
    assert!(matches!(stale, AppError::InvalidState { .. }));

    let completed = finish_package_lifecycle(
        &db,
        &operation_id,
        operation.status_version,
        OPERATION_COMPLETED,
        HOST_STATE_DISABLED,
        None,
        None,
    )
    .await
    .expect("完成停用操作");
    assert_eq!(completed.state, OPERATION_COMPLETED);
    assert_eq!(export_state(&db).await.host_state, HOST_STATE_DISABLED);

    let retry_finish = finish_package_lifecycle(
        &db,
        &operation_id,
        1,
        OPERATION_COMPLETED,
        HOST_STATE_DISABLED,
        None,
        None,
    )
    .await
    .expect("终态重复提交应幂等");
    assert_eq!(retry_finish, completed);
}

#[tokio::test]
async fn package_repeating_the_target_state_does_not_reopen_or_reversion_the_package() {
    let db = connect_test_database().await;
    ensure_host_package_states(&db, &["export".to_string()])
        .await
        .expect("初始化功能包状态");
    let current = export_state(&db).await;

    let already_enabled = start_package_lifecycle(
        &db,
        &Uuid::new_v4().to_string(),
        "export",
        "enable",
        None,
        7,
        current.status_version,
    )
    .await
    .expect("重复启用应生成已完成操作");
    assert_eq!(already_enabled.state, OPERATION_COMPLETED);
    assert_eq!(export_state(&db).await.host_state, HOST_STATE_ENABLED);
    assert_eq!(
        export_state(&db).await.status_version,
        current.status_version
    );

    let disable = start_package_lifecycle(
        &db,
        &Uuid::new_v4().to_string(),
        "export",
        "disable",
        None,
        7,
        current.status_version,
    )
    .await
    .expect("创建停用操作");
    finish_package_lifecycle(
        &db,
        &disable.operation_id,
        disable.status_version,
        OPERATION_COMPLETED,
        HOST_STATE_DISABLED,
        None,
        None,
    )
    .await
    .expect("完成停用操作");
    let disabled = export_state(&db).await;

    let already_disabled = start_package_lifecycle(
        &db,
        &Uuid::new_v4().to_string(),
        "export",
        "disable",
        None,
        7,
        disabled.status_version,
    )
    .await
    .expect("重复停用应生成已完成操作");
    assert_eq!(already_disabled.state, OPERATION_COMPLETED);
    assert_eq!(export_state(&db).await.host_state, HOST_STATE_DISABLED);
    assert_eq!(
        export_state(&db).await.status_version,
        disabled.status_version
    );
}

#[tokio::test]
async fn package_finish_rejects_a_stale_operation_instead_of_overwriting_state() {
    let db = connect_test_database().await;
    ensure_host_package_states(&db, &["export".to_string()])
        .await
        .expect("初始化功能包状态");
    let current = export_state(&db).await;
    let operation = start_package_lifecycle(
        &db,
        &Uuid::new_v4().to_string(),
        "export",
        "disable",
        None,
        7,
        current.status_version,
    )
    .await
    .expect("创建停用操作");

    let mut state: a4n_db_models::models::package::host_package_state::ActiveModel =
        export_state(&db).await.into();
    state.host_state = Set(HOST_STATE_ENABLED.to_string());
    state.update(&db).await.expect("制造并发状态变化");

    let error = finish_package_lifecycle(
        &db,
        &operation.operation_id,
        operation.status_version,
        OPERATION_COMPLETED,
        HOST_STATE_DISABLED,
        None,
        None,
    )
    .await
    .expect_err("过期停用操作不得覆盖新的 Host 状态");
    assert!(matches!(error, AppError::InvalidState { .. }));
    assert_eq!(export_state(&db).await.host_state, HOST_STATE_ENABLED);
}

#[tokio::test]
async fn package_finish_rejects_a_stale_operation_version_after_progress_update() {
    let db = connect_test_database().await;
    ensure_host_package_states(&db, &["export".to_string()])
        .await
        .expect("初始化功能包状态");
    let current = export_state(&db).await;
    let operation = start_package_lifecycle(
        &db,
        &Uuid::new_v4().to_string(),
        "export",
        "disable",
        None,
        7,
        current.status_version,
    )
    .await
    .expect("创建停用操作");

    let progressed = update_package_lifecycle_progress(
        &db,
        &operation.operation_id,
        operation.status_version,
        OPERATION_DRAINING,
        Some("work_pending"),
        None,
    )
    .await
    .expect("推进停用操作");
    assert_eq!(progressed.status_version, operation.status_version + 1);

    let error = finish_package_lifecycle(
        &db,
        &operation.operation_id,
        operation.status_version,
        OPERATION_COMPLETED,
        HOST_STATE_DISABLED,
        None,
        None,
    )
    .await
    .expect_err("旧操作版本不得完成停用");
    assert!(matches!(error, AppError::InvalidState { .. }));

    let state = export_state(&db).await;
    assert_eq!(state.host_state, "disabling");
    assert_eq!(state.status_version, current.status_version + 1);
}

#[tokio::test]
async fn package_snapshot_keeps_unknown_work_and_persistence_gate_rejects_new_work() {
    let db = connect_test_database().await;
    let db_handle = a4n_db::DbHandle::new(std::sync::Arc::new(db.clone()));
    ensure_host_package_states(&db, &["export".to_string()])
        .await
        .expect("初始化功能包状态");
    let known_id = format!("known-{}", Uuid::new_v4());
    let unknown_id = format!("legacy-{}", Uuid::new_v4());
    insert_task_queue_record(&db_handle, queue_record(&known_id, "export_generation"))
        .await
        .expect("写入 export 任务");
    insert_task_queue_record(&db_handle, queue_record(&unknown_id, "legacy_background"))
        .await
        .expect("写入未知旧任务");

    let snapshot = package_workload_snapshot(&db, "export")
        .await
        .expect("读取停用快照");
    assert_eq!(snapshot.queue_task_ids, vec![known_id.clone()]);
    assert_eq!(snapshot.unknown_task_ids, vec![unknown_id.clone()]);
    assert!(!snapshot.is_empty());

    let operation_id = Uuid::new_v4().to_string();
    let current = export_state(&db).await;
    let operation = start_package_lifecycle(
        &db,
        &operation_id,
        "export",
        "disable",
        None,
        7,
        current.status_version,
    )
    .await
    .expect("创建停用操作");
    let block = package_admission_block(&db, "export_generation", None, None)
        .await
        .expect("检查新任务准入");
    assert_eq!(
        block,
        Some(("export".to_string(), "operation_in_progress".to_string()))
    );

    let rejected =
        insert_task_queue_record(&db_handle, queue_record("new-export", "export_generation"))
            .await
            .expect_err("停用中的包不得登记新任务");
    assert!(matches!(rejected, QueueDbError::AdmissionBlocked { .. }));

    // Unknown legacy work deliberately keeps the operation draining; it is
    // not silently treated as completed.
    let after = package_workload_snapshot(&db, "export")
        .await
        .expect("重新读取停用快照");
    assert!(!after.unknown_task_ids.is_empty());
    assert_eq!(export_state(&db).await.host_state, "disabling");
    assert_eq!(operation.state, OPERATION_DRAINING);

    let queue_row = a4n_db_models::models::task::task_queue::Entity::find()
        .filter(a4n_db_models::models::task::task_queue::Column::Id.eq("new-export"))
        .one(&db)
        .await
        .expect("查询被拒任务");
    assert!(queue_row.is_none());
}

#[tokio::test]
async fn package_admission_rejects_known_package_without_state_row() {
    let db = connect_test_database().await;
    let db_handle = a4n_db::DbHandle::new(std::sync::Arc::new(db.clone()));

    let block = package_admission_block(&db, "export_generation", None, None)
        .await
        .expect("缺少功能包状态也必须返回明确准入结果");
    assert_eq!(
        block,
        Some(("export".to_string(), "not_installed".to_string()))
    );

    let rejected = insert_task_queue_record(
        &db_handle,
        queue_record("missing-export", "export_generation"),
    )
    .await
    .expect_err("缺少功能包状态不得登记任务");
    assert!(matches!(rejected, QueueDbError::AdmissionBlocked { .. }));
}

#[tokio::test]
async fn package_snapshot_waits_for_terminal_provider_cleanup() {
    let db = connect_test_database().await;
    let db_handle = a4n_db::DbHandle::new(std::sync::Arc::new(db.clone()));
    ensure_host_package_states(&db, &["video".to_string()])
        .await
        .expect("初始化 video 功能包状态");

    let task_id = Uuid::new_v4().to_string();
    insert_task_queue_record(&db_handle, queue_record(&task_id, "video_generation"))
        .await
        .expect("写入已终态 generation 的 queue 记录");
    let now = Utc::now().naive_utc();
    let task = task_queue::Entity::find_by_id(task_id.clone())
        .one(&db)
        .await
        .expect("查询 generation queue 记录")
        .expect("generation queue 记录必须存在");
    let mut task = task_queue::ActiveModel::from(task);
    task.status = Set("completed".to_string());
    task.completed_at = Set(Some(now));
    task.update(&db).await.expect("结束 generation queue 记录");

    let public_snapshot = json!({"status": "completed"});
    generation_task_checkpoint::Entity::insert(generation_task_checkpoint::ActiveModel {
        task_id: Set(task_id.clone()),
        business_key: Set(format!("package-provider-cleanup-{task_id}")),
        attempt: Set(1),
        requested_by_user_id: Set(7),
        task_kind: Set("video_generation".to_string()),
        delivery_mode: Set("stream".to_string()),
        batch_policy: Set(None),
        project_id: Set(None),
        root_project_id: Set(None),
        workflow_run_id: Set(None),
        generation_run_id: Set(None),
        target_kind: Set(None),
        target_id: Set(None),
        target_revision: Set(None),
        root_task_id: Set(task_id.clone()),
        parent_task_id: Set(None),
        item_key: Set(None),
        retry_of_task_id: Set(None),
        status: Set("completed".to_string()),
        checkpoint_revision: Set(1),
        covers_through_sequence: Set(1),
        reserved_through_sequence: Set(1),
        public_snapshot_json: Set(Some(public_snapshot.clone())),
        public_snapshot_sha256: Set(Some(
            a4n_db::generation::canonical_json_sha256(&public_snapshot)
                .expect("计算 snapshot hash"),
        )),
        step_key: Set(None),
        call_key: Set(None),
        provider_kind: Set(Some("openai-compatible-video".to_string())),
        provider_model: Set(Some("video-model".to_string())),
        provider_job_id: Set(Some("remote-video-job".to_string())),
        provider_idempotency_key_sha256: Set(Some("a".repeat(64))),
        resume_metadata_version: Set(None),
        execution_checkpoint_json: Set(None),
        execution_checkpoint_sha256: Set(None),
        input_fingerprint_sha256: Set("b".repeat(64)),
        result_kind: Set(None),
        result_id: Set(None),
        result_revision: Set(None),
        expires_at: Set(None),
        created_at: Set(now),
        updated_at: Set(now),
        completed_at: Set(Some(now)),
    })
    .exec(&db)
    .await
    .expect("写入终态 generation checkpoint");

    a4n_db::cleanup_outbox::insert_generation_provider_job_cleanup_intent(
        &db,
        format!("package-provider-cleanup:{task_id}"),
        &a4n_db::cleanup_outbox::GenerationCleanupScope {
            requested_by_user_id: 7,
            project_id: None,
            task_id: task_id.clone(),
            attempt: 1,
        },
        a4n_db::cleanup_outbox::GenerationProviderJobCleanupPayload::new(
            a4n_db::cleanup_outbox::GenerationProviderJobCleanupOperation::Reconcile,
            "openai-compatible-video".to_string(),
            "remote-video-job".to_string(),
            "a".repeat(64),
        )
        .expect("构造 provider cleanup payload"),
    )
    .await
    .expect("写入 provider cleanup intent");

    let snapshot = package_workload_snapshot(&db, "video")
        .await
        .expect("读取 provider cleanup 停用快照");
    assert_eq!(snapshot.generation_tasks, Vec::new());
    assert_eq!(snapshot.queue_task_ids, Vec::<String>::new());
    assert_eq!(snapshot.provider_job_ids, vec!["remote-video-job"]);
    assert!(!snapshot.is_empty());
    assert!(snapshot.has_external_work());
}
