use a4n_db::{
    generation::RecordStartedGenerationRunInput,
    project::{CreateCanvasProjectParams, create_canvas_project_record},
    video::create_generated_video_record_with_generation_run,
    workflow::{UpsertWorkflowRunInput, upsert_workflow_run},
};
use a4n_db_models::models::{
    ai::generated_video,
    generation::{generation_run, generation_run_target, workflow_run},
    task::task_queue,
};
use a4n_domain::media::CreateGeneratedVideoRecordDraftInput;
use a4n_domain::video_workflow::WORKFLOW_MODE_CANVAS_VIDEO;
use a4n_domain::workflow::{WORKFLOW_STATUS_INTERRUPTED, WORKFLOW_STATUS_RUNNING};
use sea_orm::{ActiveModelTrait, Set};
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};

use super::super::super::PersistentTaskQueue;
use super::super::super::test_support::{insert_task_row, setup_db};

#[tokio::test]
async fn interrupts_standalone_active_tasks() {
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
        assert!(row.completed_at.is_some());
    }
}

#[tokio::test]
async fn interrupts_historical_dag_tasks() {
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
async fn interrupts_stale_in_process_workflows() {
    let db = setup_db().await;
    let chapter = upsert_workflow_run(
        db.as_ref(),
        resumable_workflow_input("chapter-analysis-recovery", "chapter_analysis"),
    )
    .await
    .unwrap();
    let scene = upsert_workflow_run(
        db.as_ref(),
        workflow_input("scene-sync-recovery", "scene_profile_sync"),
    )
    .await
    .unwrap();
    let character_sync = upsert_workflow_run(
        db.as_ref(),
        resumable_workflow_input("character-sync-recovery", "character_profile_sync"),
    )
    .await
    .unwrap();
    let unrelated = upsert_workflow_run(
        db.as_ref(),
        resumable_workflow_input("unrelated-recovery", "video_provider_job"),
    )
    .await
    .unwrap();
    bind_task_to_workflow(db.as_ref(), "unrelated-recovery-task", unrelated.id).await;

    let _queue = PersistentTaskQueue::new(db.clone()).await.unwrap();

    let chapter = workflow_run::Entity::find_by_id(chapter.id)
        .one(db.as_ref())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(chapter.status, WORKFLOW_STATUS_INTERRUPTED);
    assert!(chapter.can_resume);
    assert!(chapter.completed_at.is_some());

    let scene = workflow_run::Entity::find_by_id(scene.id)
        .one(db.as_ref())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(scene.status, WORKFLOW_STATUS_INTERRUPTED);
    assert!(!scene.can_resume);

    let character_sync = workflow_run::Entity::find_by_id(character_sync.id)
        .one(db.as_ref())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(character_sync.status, WORKFLOW_STATUS_INTERRUPTED);
    assert!(character_sync.can_resume);
    assert!(character_sync.active_task_id.is_none());

    let unrelated = workflow_run::Entity::find_by_id(unrelated.id)
        .one(db.as_ref())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(unrelated.status, WORKFLOW_STATUS_RUNNING);
    assert!(unrelated.can_resume);
    assert!(unrelated.active_task_id.is_none());
    assert_eq!(
        task_queue::Entity::find_by_id("unrelated-recovery-task".to_string())
            .one(db.as_ref())
            .await
            .unwrap()
            .unwrap()
            .status,
        WORKFLOW_STATUS_INTERRUPTED
    );
}

#[tokio::test]
async fn interrupts_stale_video_before_provider_submission() {
    let db = setup_db().await;
    let project = create_canvas_project_record(
        db.as_ref(),
        1,
        CreateCanvasProjectParams {
            title: "视频恢复测试".to_string(),
            description: String::new(),
        },
    )
    .await
    .unwrap();
    let stale = insert_video_with_run(db.as_ref(), project.id, None).await;
    let stale_second = insert_video_with_run(db.as_ref(), project.id, None).await;
    let durable = insert_video_with_run(db.as_ref(), project.id, Some("provider-job")).await;

    let _queue = PersistentTaskQueue::new(db.clone()).await.unwrap();

    let stale = generated_video::Entity::find_by_id(stale.id)
        .one(db.as_ref())
        .await
        .unwrap()
        .unwrap();
    let stale_second = generated_video::Entity::find_by_id(stale_second.id)
        .one(db.as_ref())
        .await
        .unwrap()
        .unwrap();
    let durable = generated_video::Entity::find_by_id(durable.id)
        .one(db.as_ref())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(stale.status, WORKFLOW_STATUS_INTERRUPTED);
    assert!(
        stale
            .error_message
            .as_deref()
            .is_some_and(|message| message.contains("尚未提交"))
    );
    assert_eq!(stale_second.status, WORKFLOW_STATUS_INTERRUPTED);
    assert!(
        stale_second
            .error_message
            .as_deref()
            .is_some_and(|message| message.contains("尚未提交"))
    );
    assert_eq!(durable.status, WORKFLOW_STATUS_RUNNING);
    assert_eq!(
        video_generation_run_status(db.as_ref(), stale.id).await,
        WORKFLOW_STATUS_INTERRUPTED
    );
    assert_eq!(
        video_generation_run_status(db.as_ref(), stale_second.id).await,
        WORKFLOW_STATUS_INTERRUPTED
    );
    assert_eq!(
        video_generation_run_status(db.as_ref(), durable.id).await,
        WORKFLOW_STATUS_RUNNING
    );
}

async fn insert_video_with_run(
    db: &sea_orm::DatabaseConnection,
    project_id: i64,
    provider_job_id: Option<&str>,
) -> generated_video::Model {
    create_generated_video_record_with_generation_run(
        db,
        CreateGeneratedVideoRecordDraftInput {
            project_id,
            chapter_id: None,
            video_type: a4n_domain::video_workflow::VIDEO_TYPE_CANVAS.to_string(),
            workflow_mode: Some(WORKFLOW_MODE_CANVAS_VIDEO.to_string()),
            prompt: "prompt".to_string(),
            negative_prompt: None,
            model: "test".to_string(),
            width: 1280,
            height: 720,
            duration_secs: 5.0,
            fps: 24,
            source_video_id: None,
            source_image_id: None,
            provider_job_id: provider_job_id.map(str::to_string),
            metadata_json: Some(serde_json::json!({"config_id": 1})),
        },
        RecordStartedGenerationRunInput {
            project_id,
            workflow: "video_generation".to_string(),
            step_key: "provider_request".to_string(),
            title: "视频生成".to_string(),
            model_kind: "video".to_string(),
            provider: None,
            model: Some("test".to_string()),
            prompt_template_names_json: None,
            prompt_template_hashes_json: None,
            rendered_messages_json: None,
            rendered_prompts_json: None,
            input_variables_json: None,
            source_refs_json: None,
            media_refs_json: None,
            request_json: None,
            targets: Vec::new(),
        },
    )
    .await
    .unwrap()
}

async fn video_generation_run_status(db: &sea_orm::DatabaseConnection, video_id: i64) -> String {
    let target = generation_run_target::Entity::find()
        .filter(generation_run_target::Column::TargetTable.eq("generated_video"))
        .filter(generation_run_target::Column::TargetId.eq(video_id))
        .one(db)
        .await
        .unwrap()
        .unwrap();
    generation_run::Entity::find_by_id(target.generation_run_id)
        .one(db)
        .await
        .unwrap()
        .unwrap()
        .status
}

fn workflow_input(business_key: &str, workflow_kind: &str) -> UpsertWorkflowRunInput {
    UpsertWorkflowRunInput {
        business_key: business_key.to_string(),
        workflow_kind: workflow_kind.to_string(),
        project_id: None,
        root_project_id: None,
        source_snapshot_id: None,
        scope_table: None,
        scope_id: None,
        chapter_id: None,
        mode: None,
        input_fingerprint_json: Some(serde_json::json!({"business_key": business_key})),
        status: WORKFLOW_STATUS_RUNNING.to_string(),
        current_step_key: Some("test.running".to_string()),
        active_task_id: Some(format!("{business_key}-task")),
        can_resume: false,
        resume_block_reason: None,
        error_message: None,
    }
}

fn resumable_workflow_input(business_key: &str, workflow_kind: &str) -> UpsertWorkflowRunInput {
    UpsertWorkflowRunInput {
        can_resume: true,
        ..workflow_input(business_key, workflow_kind)
    }
}

async fn bind_task_to_workflow(
    db: &sea_orm::DatabaseConnection,
    task_id: &str,
    workflow_run_id: i64,
) {
    super::super::super::test_support::insert_task_row(db, task_id, "running", None).await;
    let task = task_queue::Entity::find_by_id(task_id.to_string())
        .one(db)
        .await
        .unwrap()
        .unwrap();
    let mut active: task_queue::ActiveModel = task.into();
    active.workflow_run_id = Set(Some(workflow_run_id));
    active.update(db).await.unwrap();

    let workflow = workflow_run::Entity::find_by_id(workflow_run_id)
        .one(db)
        .await
        .unwrap()
        .unwrap();
    let mut active: workflow_run::ActiveModel = workflow.into();
    active.active_task_id = Set(Some(task_id.to_string()));
    active.update(db).await.unwrap();
}
