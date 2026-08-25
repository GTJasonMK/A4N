use a4n_core::models::generation::workflow_run;
use a4n_core::models::task::task_queue;
use a4n_db::workflow::{UpsertWorkflowRunInput, WORKFLOW_STATUS_RUNNING, upsert_workflow_run};
use a4n_queue::TaskWorkflowContext;
use a4n_queue::priority::{TaskKind, TaskPriority};
use sea_orm::EntityTrait;
use sea_orm::{ActiveModelTrait, Set};

use super::super::super::PersistentTaskQueue;
use super::super::super::test_support::setup_db;

#[tokio::test]
async fn mark_done_updates_database_status() {
    let db = setup_db().await;
    let queue = PersistentTaskQueue::new(db.clone()).await.unwrap();
    let task_id = queue
        .submit_internal(TaskKind::LlmCall, TaskPriority::High, None, None, None)
        .await
        .unwrap();

    queue
        .update_status(&task_id, "completed", None)
        .await
        .unwrap();

    let row = task_queue::Entity::find_by_id(task_id)
        .one(db.as_ref())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(row.status, "completed");
    assert!(row.completed_at.is_some());
}

#[tokio::test]
async fn mark_failed_records_error() {
    let db = setup_db().await;
    let queue = PersistentTaskQueue::new(db.clone()).await.unwrap();
    let task_id = queue
        .submit_internal(TaskKind::LlmCall, TaskPriority::Medium, None, None, None)
        .await
        .unwrap();

    queue
        .update_status(&task_id, "failed", Some("LLM 调用超时"))
        .await
        .unwrap();

    let row = task_queue::Entity::find_by_id(task_id)
        .one(db.as_ref())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(row.status, "failed");
    assert_eq!(row.error_message.as_deref(), Some("LLM 调用超时"));
}

#[tokio::test]
async fn task_status_and_workflow_active_task_update_together() {
    let db = setup_db().await;
    let queue = PersistentTaskQueue::new(db.clone()).await.unwrap();
    let workflow = upsert_workflow_run(
        db.as_ref(),
        UpsertWorkflowRunInput {
            business_key: "queue-status-workflow".to_string(),
            workflow_kind: "test_queue_status".to_string(),
            project_id: Some(7),
            root_project_id: None,
            source_snapshot_id: None,
            scope_table: None,
            scope_id: None,
            chapter_id: None,
            mode: None,
            input_fingerprint_json: Some(serde_json::json!({"test": true})),
            status: WORKFLOW_STATUS_RUNNING.to_string(),
            current_step_key: Some("queue.running".to_string()),
            active_task_id: None,
            can_resume: false,
            resume_block_reason: None,
            error_message: None,
        },
    )
    .await
    .unwrap();
    let task_id = queue
        .submit_internal(
            TaskKind::LlmCall,
            TaskPriority::Medium,
            Some(7),
            None,
            Some(&TaskWorkflowContext::new(workflow.id).unwrap()),
        )
        .await
        .unwrap();

    let permit = queue
        .acquire_task_with_cancel(&task_id, tokio_util::sync::CancellationToken::new())
        .await
        .unwrap()
        .unwrap();
    let running = workflow_run::Entity::find_by_id(workflow.id)
        .one(db.as_ref())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(running.active_task_id.as_deref(), Some(task_id.as_str()));

    permit.complete();
    queue
        .update_status(&task_id, "completed", None)
        .await
        .unwrap();
    let completed = workflow_run::Entity::find_by_id(workflow.id)
        .one(db.as_ref())
        .await
        .unwrap()
        .unwrap();
    assert!(completed.active_task_id.is_none());
}

#[tokio::test]
async fn missing_workflow_rolls_back_task_status_update() {
    let db = setup_db().await;
    let queue = PersistentTaskQueue::new(db.clone()).await.unwrap();
    let task_id = queue
        .submit_internal(TaskKind::LlmCall, TaskPriority::Medium, None, None, None)
        .await
        .unwrap();
    let task = task_queue::Entity::find_by_id(task_id.clone())
        .one(db.as_ref())
        .await
        .unwrap()
        .unwrap();
    let mut active: task_queue::ActiveModel = task.into();
    active.workflow_run_id = Set(Some(i64::MAX));
    active.update(db.as_ref()).await.unwrap();

    let error = queue
        .update_status(&task_id, "running", None)
        .await
        .unwrap_err();
    assert!(error.to_string().contains("队列任务关联工作流不存在"));

    let unchanged = task_queue::Entity::find_by_id(task_id)
        .one(db.as_ref())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(unchanged.status, "pending");
    assert!(unchanged.started_at.is_none());
}
