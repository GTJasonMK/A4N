use a4n_db::workflow::{UpsertWorkflowRunInput, mark_workflow_status, upsert_workflow_run};
use a4n_db_models::models::generation::workflow_run;
use a4n_domain::task_queue::LlmConcurrencyLimit;
use a4n_error::AppError;
use a4n_queue::{
    TaskWorkflowContext,
    priority::{TaskKind, TaskPriority},
};
use sea_orm::EntityTrait;
use tokio_util::sync::CancellationToken;

use super::super::{PersistentTaskQueue, test_support::setup_db};

#[tokio::test]
async fn cancelling_a_workflow_stops_calls_and_rejects_late_admission() {
    let db = setup_db().await;
    let queue = PersistentTaskQueue::new(a4n_db::DbHandle::new(db.clone()))
        .await
        .unwrap();
    let parent = queue
        .submit_internal(
            TaskKind::TextGeneration,
            TaskPriority::Medium,
            None,
            None,
            None,
        )
        .await
        .unwrap();
    let parent_permit = queue
        .acquire_task_with_cancel(&parent, CancellationToken::new())
        .await
        .unwrap()
        .unwrap();
    let workflow = upsert_workflow_run(
        db.as_ref(),
        UpsertWorkflowRunInput {
            business_key: "llm-cancellation-test".into(),
            workflow_kind: "video_canvas_graph".into(),
            project_id: None,
            root_project_id: None,
            source_snapshot_id: None,
            scope_table: None,
            scope_id: None,
            chapter_id: None,
            mode: None,
            input_fingerprint_json: None,
            status: "running".into(),
            current_step_key: None,
            active_task_id: Some(parent.clone()),
            can_resume: true,
            resume_block_reason: None,
            error_message: None,
        },
    )
    .await
    .unwrap();
    let scope = Some(TaskWorkflowContext::new(workflow.id).unwrap());
    let limit = || {
        Some(LlmConcurrencyLimit {
            key: "test-model".into(),
            max_concurrent: 1,
        })
    };
    let task = a4n_queue::with_llm_slot(queue.start_task_with_limit(
        TaskKind::LlmCall,
        TaskPriority::Medium,
        None,
        scope.clone(),
        CancellationToken::new(),
        limit(),
    ))
    .await
    .unwrap();
    let cancel = task.cancellation_token();
    let current = workflow_run::Entity::find_by_id(workflow.id)
        .one(db.as_ref())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(current.active_task_id.as_deref(), Some(parent.as_str()));
    let pending = queue
        .submit_internal(
            TaskKind::LlmCall,
            TaskPriority::Medium,
            None,
            None,
            scope.as_ref(),
        )
        .await
        .unwrap();
    mark_workflow_status(
        db.as_ref(),
        workflow.id,
        "cancelled",
        true,
        Some("stop test"),
    )
    .await
    .unwrap();
    queue.cancel_workflow_llm_calls(workflow.id).await.unwrap();
    assert!(cancel.is_cancelled());
    assert!(
        queue
            .inner
            .acquire_llm_call_with_cancel(&pending, CancellationToken::new(), limit().unwrap())
            .await
            .unwrap()
            .is_none()
    );
    task.cancel().await.unwrap();
    assert!(matches!(
        queue
            .start_task_with_limit(
                TaskKind::LlmCall,
                TaskPriority::Medium,
                None,
                scope,
                CancellationToken::new(),
                limit()
            )
            .await,
        Err(AppError::Cancelled)
    ));
    queue.cancel(&parent).await.unwrap();
    parent_permit.cancel();
}

#[tokio::test]
async fn persisted_cancellation_wins_over_a_stale_memory_pending_task() {
    let db = setup_db().await;
    let queue = PersistentTaskQueue::new(a4n_db::DbHandle::new(db.clone()))
        .await
        .unwrap();
    let id = queue
        .submit_internal(TaskKind::LlmCall, TaskPriority::Medium, None, None, None)
        .await
        .unwrap();
    a4n_db::queue::update_task_status(
        &a4n_db::DbHandle::new(db.clone()),
        &id,
        "cancelled",
        Some("cancel during handoff"),
    )
    .await
    .unwrap();
    assert!(matches!(
        queue
            .acquire_task_with_cancel(&id, CancellationToken::new())
            .await,
        Err(AppError::Cancelled)
    ));
    assert_eq!(queue.queue_stats().llm_queue.running, 0);
}
