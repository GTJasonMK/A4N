use a4n_core::models::task::task_queue;
use a4n_queue::{
    priority::{TaskPriority, TaskQueue, TaskType},
    queue::{TaskFamilyContext, TaskWorkflowContext},
};
use sea_orm::EntityTrait;

use super::super::super::PersistentTaskQueue;
use super::super::super::test_support::setup_db;

#[tokio::test]
async fn family_context_persists_project_family_fields() {
    let db = setup_db().await;
    let queue = PersistentTaskQueue::new(db.clone()).await.unwrap();
    let context = TaskFamilyContext::new(21, 7, Some(3), "video_prompt").unwrap();

    let task_id = queue
        .submit_internal(
            TaskType::VideoGeneration,
            TaskPriority::Medium,
            Some(context.project_id),
            Some(&context),
            None,
        )
        .await
        .unwrap();

    let row = task_queue::Entity::find_by_id(task_id.clone())
        .one(db.as_ref())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(row.project_id, Some(21));
    assert_eq!(row.root_project_id, Some(7));
    assert_eq!(row.source_snapshot_id, Some(3));
    assert_eq!(row.task_scope.as_deref(), Some("video_prompt"));

    let info = queue.status(&task_id).await.unwrap();
    assert_eq!(info.project_id, Some(21));
    assert_eq!(info.root_project_id, Some(7));
    assert_eq!(info.source_snapshot_id, Some(3));
    assert_eq!(info.task_scope.as_deref(), Some("video_prompt"));
}

#[tokio::test]
async fn workflow_context_persists_workflow_run_id() {
    let db = setup_db().await;
    let queue = PersistentTaskQueue::new(db.clone()).await.unwrap();
    let task_id = queue
        .submit_with_workflow_context(
            TaskType::VideoPoll,
            TaskPriority::Low,
            Some(21),
            TaskWorkflowContext::new(99).unwrap(),
        )
        .await
        .unwrap();

    let row = task_queue::Entity::find_by_id(task_id)
        .one(db.as_ref())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(row.workflow_run_id, Some(99));
    assert_eq!(row.project_id, Some(21));
}

#[tokio::test]
async fn project_family_supports_source_sync_and_assembly_tasks() {
    let db = setup_db().await;
    let queue = PersistentTaskQueue::new(db).await.unwrap();
    let source_sync_id = queue
        .submit_internal(
            TaskType::SourceSync,
            TaskPriority::Medium,
            Some(21),
            Some(&TaskFamilyContext::new(21, 7, Some(3), "source_sync").unwrap()),
            None,
        )
        .await
        .unwrap();
    let assembly_id = queue
        .submit_internal(
            TaskType::Assembly,
            TaskPriority::Medium,
            Some(22),
            Some(&TaskFamilyContext::new(22, 7, Some(4), "assembly").unwrap()),
            None,
        )
        .await
        .unwrap();

    let source_sync = queue.status(&source_sync_id).await.unwrap();
    assert_eq!(source_sync.task_type, TaskType::SourceSync);
    assert_eq!(source_sync.project_id, Some(21));
    assert_eq!(source_sync.root_project_id, Some(7));
    assert_eq!(source_sync.source_snapshot_id, Some(3));
    assert_eq!(source_sync.task_scope.as_deref(), Some("source_sync"));

    let assembly = queue.status(&assembly_id).await.unwrap();
    assert_eq!(assembly.task_type, TaskType::Assembly);
    assert_eq!(assembly.project_id, Some(22));
    assert_eq!(assembly.root_project_id, Some(7));
    assert_eq!(assembly.source_snapshot_id, Some(4));
    assert_eq!(assembly.task_scope.as_deref(), Some("assembly"));
}

#[tokio::test]
async fn history_returns_project_family_fields() {
    let db = setup_db().await;
    let queue = PersistentTaskQueue::new(db).await.unwrap();
    let context = TaskFamilyContext::new(31, 7, Some(5), "media_generation").unwrap();
    let task_id = queue
        .submit_internal(
            TaskType::ImageGeneration,
            TaskPriority::High,
            Some(context.project_id),
            Some(&context),
            None,
        )
        .await
        .unwrap();

    let history = queue.history(Some(31), 0, 10).await.unwrap();
    let task = history
        .iter()
        .find(|task| task.id == task_id)
        .expect("history should include submitted task");
    assert_eq!(task.root_project_id, Some(7));
    assert_eq!(task.source_snapshot_id, Some(5));
    assert_eq!(task.task_scope.as_deref(), Some("media_generation"));
}
