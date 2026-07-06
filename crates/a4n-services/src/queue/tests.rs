use super::*;
use a4n_core::models::ai::{image_gen_config, llm_config, video_gen_config};
use a4n_core::models::task::task_queue;
use a4n_engine::queue::priority::{TaskPriority, TaskQueue, TaskStatus, TaskType};
use chrono::Utc;
use sea_orm::{DatabaseConnection, EntityTrait, Set};

async fn setup_db() -> Arc<DatabaseConnection> {
    Arc::new(a4n_migration::test_support::connect_test_database().await)
}

async fn insert_task_row(db: &DatabaseConnection, id: &str, status: &str, dag_id: Option<&str>) {
    let now = Utc::now().naive_utc();
    task_queue::Entity::insert(task_queue::ActiveModel {
        id: Set(id.to_string()),
        task_type: Set("llm_call".to_string()),
        priority: Set("high".to_string()),
        status: Set(status.to_string()),
        project_id: Set(Some(7)),
        root_project_id: Set(None),
        source_snapshot_id: Set(None),
        task_scope: Set(None),
        workflow_run_id: Set(None),
        payload: Set(Some(serde_json::json!({"id": id}))),
        progress: Set(None),
        error_message: Set(None),
        dag_id: Set(dag_id.map(ToString::to_string)),
        created_at: Set(now),
        started_at: Set((status == "running").then_some(now)),
        completed_at: Set((status == "completed").then_some(now)),
    })
    .exec(db)
    .await
    .unwrap();
}

#[tokio::test]
async fn submit_with_family_context_persists_project_family_fields() {
    let db = setup_db().await;
    let queue = PersistentTaskQueue::new(db.clone()).await.unwrap();
    let context = TaskFamilyContext::new(21, 7, Some(3), "video_prompt").unwrap();

    let task_id = queue
        .submit_with_family_context(TaskType::VideoGeneration, TaskPriority::Medium, context)
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
async fn submit_with_workflow_context_persists_workflow_run_id() {
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
async fn task_family_context_rejects_invalid_scope() {
    let err = TaskFamilyContext::new(21, 7, Some(3), "unknown_scope")
        .expect_err("invalid task scope should fail visibly");
    assert!(err.to_string().contains("task_scope 无效"));
}

#[tokio::test]
async fn task_workflow_context_rejects_invalid_workflow_run_id() {
    let err = TaskWorkflowContext::new(0).expect_err("zero workflow_run_id should fail visibly");
    assert!(err.to_string().contains("workflow_run_id 必须大于 0"));
}

#[tokio::test]
async fn project_family_queue_supports_source_sync_and_assembly_tasks() {
    let db = setup_db().await;
    let queue = PersistentTaskQueue::new(db.clone()).await.unwrap();

    let source_sync_id = queue
        .submit_with_family_context(
            TaskType::SourceSync,
            TaskPriority::Medium,
            TaskFamilyContext::new(21, 7, Some(3), "source_sync").unwrap(),
        )
        .await
        .unwrap();
    let assembly_id = queue
        .submit_with_family_context(
            TaskType::Assembly,
            TaskPriority::Medium,
            TaskFamilyContext::new(22, 7, Some(4), "assembly").unwrap(),
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
async fn history_returns_project_family_fields_from_database() {
    let db = setup_db().await;
    let queue = PersistentTaskQueue::new(db.clone()).await.unwrap();
    let context = TaskFamilyContext::new(31, 7, Some(5), "media_generation").unwrap();
    let task_id = queue
        .submit_with_family_context(TaskType::ImageGeneration, TaskPriority::High, context)
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

async fn insert_invalid_default_llm_config(db: &DatabaseConnection) {
    let now = Utc::now().naive_utc();
    llm_config::Entity::insert(llm_config::ActiveModel {
        name: Set("invalid-llm".to_string()),
        provider: Set("openai".to_string()),
        base_url: Set("https://api.openai.com/v1".to_string()),
        api_key_encrypted: Set(Some("sk-test".to_string())),
        model: Set("gpt-4.1".to_string()),
        max_concurrent: Set(0),
        timeout_secs: Set(60),
        is_default: Set(true),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    })
    .exec(db)
    .await
    .unwrap();
}

async fn insert_invalid_default_image_config(db: &DatabaseConnection) {
    let now = Utc::now().naive_utc();
    image_gen_config::Entity::insert(image_gen_config::ActiveModel {
        name: Set("invalid-image".to_string()),
        backend: Set("openai-compatible-image".to_string()),
        base_url: Set(Some("https://api.openai.com/v1".to_string())),
        api_key_encrypted: Set(Some("sk-test".to_string())),
        model: Set("dall-e-3".to_string()),
        max_concurrent: Set(0),
        default_width: Set(1024),
        default_height: Set(1024),
        is_default: Set(true),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    })
    .exec(db)
    .await
    .unwrap();
}

async fn insert_invalid_default_video_config(db: &DatabaseConnection) {
    let now = Utc::now().naive_utc();
    video_gen_config::Entity::insert(video_gen_config::ActiveModel {
        name: Set("invalid-video".to_string()),
        backend: Set("openai-compatible-video".to_string()),
        base_url: Set(Some("https://api.example.com/v1".to_string())),
        api_key_encrypted: Set(Some("sk-test".to_string())),
        model: Set("video-model".to_string()),
        max_concurrent: Set(0),
        default_width: Set(1280),
        default_height: Set(720),
        default_duration_secs: Set(5),
        min_duration_secs: Set(1),
        max_duration_secs: Set(10),
        duration_mode: Set("range".to_string()),
        supported_durations_json: Set(None),
        default_fps: Set(24),
        is_default: Set(true),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    })
    .exec(db)
    .await
    .unwrap();
}

#[tokio::test]
async fn queue_init_rejects_invalid_default_llm_concurrency() {
    let db = setup_db().await;
    insert_invalid_default_llm_config(db.as_ref()).await;

    let err = match PersistentTaskQueue::new(db).await {
        Ok(_) => panic!("invalid default LLM concurrency should fail visibly"),
        Err(err) => err,
    };

    assert!(matches!(err, AppError::ConfigError(_)));
    assert!(err.to_string().contains("默认 LLM 配置 max_concurrent"));
}

#[tokio::test]
async fn queue_init_rejects_invalid_default_image_concurrency() {
    let db = setup_db().await;
    insert_invalid_default_image_config(db.as_ref()).await;

    let err = match PersistentTaskQueue::new(db).await {
        Ok(_) => panic!("invalid default image concurrency should fail visibly"),
        Err(err) => err,
    };

    assert!(matches!(err, AppError::ConfigError(_)));
    assert!(err.to_string().contains("默认图片配置 max_concurrent"));
}

#[tokio::test]
async fn queue_init_rejects_invalid_default_video_concurrency() {
    let db = setup_db().await;
    insert_invalid_default_video_config(db.as_ref()).await;

    let err = match PersistentTaskQueue::new(db).await {
        Ok(_) => panic!("invalid default video concurrency should fail visibly"),
        Err(err) => err,
    };

    assert!(matches!(err, AppError::ConfigError(_)));
    assert!(err.to_string().contains("默认视频配置 max_concurrent"));
}

#[tokio::test]
async fn recover_pending_interrupts_standalone_active_task() {
    let db = setup_db().await;
    insert_task_row(db.as_ref(), "recover-running", "running", None).await;
    insert_task_row(db.as_ref(), "recover-pending", "pending", None).await;

    let _queue = PersistentTaskQueue::new(db.clone()).await.unwrap();

    for task_id in ["recover-running", "recover-pending"] {
        let row = task_queue::Entity::find_by_id(task_id.to_string())
            .one(db.as_ref())
            .await
            .unwrap()
            .unwrap();
        assert_eq!(row.status, "interrupted", "{task_id} should be interrupted");
        assert!(
            row.completed_at.is_some(),
            "{task_id} should record interruption time"
        );
    }
}

#[tokio::test]
async fn recover_pending_interrupts_historical_dag_task() {
    let db = setup_db().await;
    insert_task_row(db.as_ref(), "dag-child", "pending", Some("historical-dag")).await;

    let _queue = PersistentTaskQueue::new(db.clone()).await.unwrap();

    let row = task_queue::Entity::find_by_id("dag-child".to_string())
        .one(db.as_ref())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(row.status, "interrupted");
    assert_eq!(row.dag_id.as_deref(), Some("historical-dag"));
    assert!(row.completed_at.is_some());
}

#[tokio::test]
async fn cancel_completed_task_preserves_memory_and_db_terminal_status() {
    let db = setup_db().await;
    let queue = PersistentTaskQueue::new(db.clone()).await.unwrap();
    let task_id = queue
        .submit(TaskType::LlmCall, TaskPriority::High, Some(9))
        .await
        .unwrap();

    let permit = queue.acquire(TaskType::LlmCall).await.unwrap();
    assert_eq!(permit.task_id(), &task_id);
    permit.complete();
    queue.mark_done(&task_id).await.unwrap();

    queue.cancel(&task_id).await.unwrap();

    let info = queue.status(&task_id).await.unwrap();
    assert!(matches!(info.status, TaskStatus::Completed));
    let row = task_queue::Entity::find_by_id(task_id)
        .one(db.as_ref())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(row.status, "completed");
    assert_eq!(row.project_id, Some(9));
}

#[tokio::test]
async fn mark_done_after_running_cancel_preserves_cancelled_memory_and_db_status() {
    let db = setup_db().await;
    let queue = PersistentTaskQueue::new(db.clone()).await.unwrap();
    let task_id = queue
        .submit(TaskType::LlmCall, TaskPriority::High, Some(10))
        .await
        .unwrap();
    let permit = queue.acquire(TaskType::LlmCall).await.unwrap();
    assert_eq!(permit.task_id(), &task_id);

    queue.cancel(&task_id).await.unwrap();
    permit.complete();
    queue.mark_done(&task_id).await.unwrap();

    let info = queue.status(&task_id).await.unwrap();
    assert!(matches!(info.status, TaskStatus::Cancelled));
    let row = task_queue::Entity::find_by_id(task_id)
        .one(db.as_ref())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(row.status, "cancelled");
    assert_eq!(row.project_id, Some(10));
}

#[tokio::test]
async fn mark_done_updates_db_status() {
    let db = setup_db().await;
    let queue = PersistentTaskQueue::new(db.clone()).await.unwrap();

    let task_id = queue
        .submit(TaskType::LlmCall, TaskPriority::High, None)
        .await
        .unwrap();

    queue.mark_done(&task_id).await.unwrap();
    let row = task_queue::Entity::find_by_id(task_id)
        .one(db.as_ref())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(row.status, "completed");
    assert!(row.completed_at.is_some());
}

#[tokio::test]
async fn submitted_task_guard_cancels_dropped_pending_task() {
    let db = setup_db().await;
    let queue = PersistentTaskQueue::new(db.clone()).await.unwrap();
    let task_id = queue
        .submit(TaskType::LlmCall, TaskPriority::High, Some(11))
        .await
        .unwrap();

    let guard = queue.submitted_task_guard(task_id.clone());
    drop(guard);
    tokio::time::sleep(std::time::Duration::from_millis(20)).await;

    let info = queue.status(&task_id).await.unwrap();
    assert!(matches!(info.status, TaskStatus::Cancelled));
    let row = task_queue::Entity::find_by_id(task_id)
        .one(db.as_ref())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(row.status, "cancelled");
    assert_eq!(row.project_id, Some(11));
}

#[tokio::test]
async fn mark_failed_records_error() {
    let db = setup_db().await;
    let queue = PersistentTaskQueue::new(db.clone()).await.unwrap();

    let task_id = queue
        .submit(TaskType::LlmCall, TaskPriority::Medium, None)
        .await
        .unwrap();

    queue.mark_failed(&task_id, "LLM 调用超时").await.unwrap();

    let row = task_queue::Entity::find_by_id(task_id)
        .one(db.as_ref())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(row.status, "failed");
    assert_eq!(row.error_message.as_deref(), Some("LLM 调用超时"));
}
