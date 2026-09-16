use a4n_db::workflow::{UpsertWorkflowRunInput, upsert_workflow_run};
use a4n_db_models::models::generation::workflow_run;
use a4n_db_models::models::task::task_queue;
use a4n_domain::workflow::WORKFLOW_STATUS_RUNNING;
use a4n_queue::TaskWorkflowContext;
use a4n_queue::priority::{TaskKind, TaskPriority};
use sea_orm::EntityTrait;

use super::super::PersistentTaskQueue;
use super::super::test_support::setup_db;

#[tokio::test]
async fn dropped_pending_task_is_cancelled() {
    let db = setup_db().await;
    let queue = PersistentTaskQueue::new(db.clone()).await.unwrap();
    let task_id = queue
        .submit_internal(TaskKind::LlmCall, TaskPriority::High, Some(11), None, None)
        .await
        .unwrap();

    let guard = queue.submitted_task_guard(task_id.clone());
    drop(guard);
    tokio::time::timeout(std::time::Duration::from_secs(5), async {
        loop {
            let status = task_queue::Entity::find_by_id(task_id.clone())
                .one(db.as_ref())
                .await
                .unwrap()
                .unwrap()
                .status;
            if status == "cancelled" {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
    })
    .await
    .expect("dropped task cancellation was not persisted before timeout");

    let row = task_queue::Entity::find_by_id(task_id)
        .one(db.as_ref())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(row.status, "cancelled");
    assert_eq!(row.project_id, Some(11));
}

#[tokio::test]
async fn dropped_pending_workflow_task_clears_active_task() {
    let db = setup_db().await;
    let queue = PersistentTaskQueue::new(db.clone()).await.unwrap();
    let workflow = upsert_workflow_run(
        db.as_ref(),
        UpsertWorkflowRunInput {
            business_key: "dropped-queue-workflow".to_string(),
            workflow_kind: "test_queue_drop".to_string(),
            project_id: Some(11),
            root_project_id: None,
            source_snapshot_id: None,
            scope_table: None,
            scope_id: None,
            chapter_id: None,
            mode: None,
            input_fingerprint_json: Some(serde_json::json!({"test": true})),
            status: WORKFLOW_STATUS_RUNNING.to_string(),
            current_step_key: Some("queue.pending".to_string()),
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
            TaskKind::TextGeneration,
            TaskPriority::High,
            Some(11),
            None,
            Some(&TaskWorkflowContext::new(workflow.id).unwrap()),
        )
        .await
        .unwrap();
    queue
        .update_status(&task_id, "pending", None)
        .await
        .unwrap();
    let pending = workflow_run::Entity::find_by_id(workflow.id)
        .one(db.as_ref())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(pending.active_task_id.as_deref(), Some(task_id.as_str()));

    drop(queue.submitted_task_guard(task_id));
    tokio::time::timeout(std::time::Duration::from_secs(5), async {
        loop {
            let run = workflow_run::Entity::find_by_id(workflow.id)
                .one(db.as_ref())
                .await
                .unwrap()
                .unwrap();
            if run.active_task_id.is_none() {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
    })
    .await
    .expect("dropped workflow task did not clear active_task_id");
}
