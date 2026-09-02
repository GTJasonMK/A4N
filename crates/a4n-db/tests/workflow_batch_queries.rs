use a4n_core::models::generation::generation_run;
use a4n_db::workflow::{
    GetWorkflowRunInput, UpsertWorkflowCheckpointInput, UpsertWorkflowRunInput,
    WORKFLOW_STATUS_COMPLETED, WORKFLOW_STATUS_RUNNING,
    latest_workflow_run_recovery_records_by_business_keys, latest_workflow_runs_by_business_keys,
    mark_workflow_status, resume_or_upsert_workflow_run, upsert_workflow_checkpoint,
    upsert_workflow_run, workflow_checkpoint_statuses_for_runs, workflow_checkpoint_step_statuses,
    workflow_run_detail, workflow_run_models_by_ids, workflow_run_statuses_by_ids,
};
use chrono::Utc;
use sea_orm::{ActiveModelTrait, Set};
use serde_json::json;

fn workflow_input(business_key: &str, fingerprint: &str, status: &str) -> UpsertWorkflowRunInput {
    UpsertWorkflowRunInput {
        business_key: business_key.to_string(),
        workflow_kind: "video_provider_job".to_string(),
        project_id: None,
        root_project_id: None,
        source_snapshot_id: None,
        scope_table: None,
        scope_id: None,
        chapter_id: None,
        mode: None,
        input_fingerprint_json: Some(serde_json::json!({"fingerprint": fingerprint})),
        status: status.to_string(),
        current_step_key: None,
        active_task_id: None,
        can_resume: status == WORKFLOW_STATUS_RUNNING,
        resume_block_reason: None,
        error_message: None,
    }
}

#[tokio::test]
async fn latest_workflow_runs_by_business_keys_returns_one_latest_row_per_key() {
    let db = a4n_migration::test_support::connect_test_database().await;
    let suffix = uuid::Uuid::new_v4();
    let key = format!("workflow-batch:{suffix}:primary");
    let other_key = format!("workflow-batch:{suffix}:other");

    let old = upsert_workflow_run(&db, workflow_input(&key, "old", WORKFLOW_STATUS_COMPLETED))
        .await
        .expect("old workflow must be inserted");
    let latest = upsert_workflow_run(&db, workflow_input(&key, "new", WORKFLOW_STATUS_RUNNING))
        .await
        .expect("latest workflow must be inserted");
    let other = upsert_workflow_run(
        &db,
        workflow_input(&other_key, "other", WORKFLOW_STATUS_COMPLETED),
    )
    .await
    .expect("second workflow must be inserted");

    let rows =
        latest_workflow_runs_by_business_keys(&db, &[key.clone(), key.clone(), other_key.clone()])
            .await
            .expect("batch workflow query must succeed");

    assert_eq!(rows.len(), 2);
    assert!(rows.iter().any(|row| row.id == latest.id));
    assert!(!rows.iter().any(|row| row.id == old.id));
    assert!(rows.iter().any(|row| row.id == other.id));

    let recovery_rows = latest_workflow_run_recovery_records_by_business_keys(
        &db,
        &[key.clone(), key.clone(), other_key.clone()],
    )
    .await
    .expect("workflow recovery query must succeed");
    assert_eq!(recovery_rows.len(), 2);
    let latest_recovery = recovery_rows
        .iter()
        .find(|row| row.business_key == key)
        .expect("latest recovery row must be present");
    assert_eq!(latest_recovery.id, latest.id);
    assert_eq!(latest_recovery.status, WORKFLOW_STATUS_RUNNING);
    assert_eq!(latest_recovery.error_message, None);

    let models = workflow_run_models_by_ids(&db, &[other.id, latest.id, latest.id])
        .await
        .expect("batch workflow model query must succeed");
    assert_eq!(
        models.iter().map(|run| run.id).collect::<Vec<_>>(),
        vec![latest.id, other.id]
    );

    assert_eq!(
        workflow_run_statuses_by_ids(&db, &[other.id, latest.id, latest.id])
            .await
            .expect("workflow status query must succeed"),
        vec![
            (latest.id, WORKFLOW_STATUS_RUNNING.to_string()),
            (other.id, WORKFLOW_STATUS_COMPLETED.to_string()),
        ]
    );
}

#[tokio::test]
async fn resume_or_upsert_reuses_matching_interrupted_run() {
    let db = a4n_migration::test_support::connect_test_database().await;
    let suffix = uuid::Uuid::new_v4();
    let key = format!("workflow-resume:{suffix}");
    let fingerprint = format!("fingerprint-{suffix}");
    let initial = upsert_workflow_run(
        &db,
        workflow_input(&key, &fingerprint, WORKFLOW_STATUS_RUNNING),
    )
    .await
    .expect("initial workflow must be inserted");
    mark_workflow_status(
        &db,
        initial.id,
        a4n_db::workflow::WORKFLOW_STATUS_INTERRUPTED,
        true,
        Some("应用重启后中断，保留断点"),
    )
    .await
    .expect("workflow must become resumably interrupted");

    let mut resume_input = workflow_input(&key, &fingerprint, WORKFLOW_STATUS_RUNNING);
    resume_input.active_task_id = Some("resume-task".to_string());
    let resumed = resume_or_upsert_workflow_run(&db, resume_input)
        .await
        .expect("matching interrupted workflow must resume");

    assert_eq!(resumed.id, initial.id);
    assert_eq!(resumed.status, WORKFLOW_STATUS_RUNNING);
    assert_eq!(resumed.active_task_id.as_deref(), Some("resume-task"));
    assert!(resumed.completed_at.is_none());
}

#[tokio::test]
async fn workflow_checkpoint_status_query_returns_only_run_and_status() {
    let db = a4n_migration::test_support::connect_test_database().await;
    let suffix = uuid::Uuid::new_v4();
    let run = upsert_workflow_run(
        &db,
        workflow_input(
            &format!("workflow-checkpoint-status:{suffix}"),
            "checkpoint",
            WORKFLOW_STATUS_RUNNING,
        ),
    )
    .await
    .expect("workflow must be inserted");
    for (step_order, status) in [(2, WORKFLOW_STATUS_COMPLETED), (1, WORKFLOW_STATUS_RUNNING)] {
        upsert_workflow_checkpoint(
            &db,
            UpsertWorkflowCheckpointInput {
                workflow_run_id: run.id,
                step_key: format!("step-{step_order}"),
                step_order,
                status: status.to_string(),
                input_fingerprint_sha256: None,
                input_fingerprint_json: None,
                checkpoint_json: Some(serde_json::json!({"large": "payload"})),
                artifacts_json: Some(serde_json::json!({"artifact": true})),
                generation_run_ids_json: None,
                error_message: None,
                completed: status == WORKFLOW_STATUS_COMPLETED,
            },
        )
        .await
        .expect("checkpoint must be inserted");
    }

    assert_eq!(
        workflow_checkpoint_statuses_for_runs(&db, &[run.id])
            .await
            .expect("checkpoint status query must succeed"),
        vec![
            (run.id, WORKFLOW_STATUS_RUNNING.to_string()),
            (run.id, WORKFLOW_STATUS_COMPLETED.to_string()),
        ]
    );
    assert!(
        workflow_checkpoint_statuses_for_runs(&db, &[])
            .await
            .expect("empty checkpoint status query must succeed")
            .is_empty()
    );
}

#[tokio::test]
async fn workflow_checkpoint_step_status_query_returns_ordered_narrow_rows() {
    let db = a4n_migration::test_support::connect_test_database().await;
    let suffix = uuid::Uuid::new_v4();
    let run = upsert_workflow_run(
        &db,
        workflow_input(
            &format!("workflow-checkpoint-step-status:{suffix}"),
            "checkpoint",
            WORKFLOW_STATUS_RUNNING,
        ),
    )
    .await
    .expect("workflow must be inserted");

    for (step_order, status) in [(2, WORKFLOW_STATUS_COMPLETED), (1, WORKFLOW_STATUS_RUNNING)] {
        upsert_workflow_checkpoint(
            &db,
            UpsertWorkflowCheckpointInput {
                workflow_run_id: run.id,
                step_key: format!("step-{step_order}"),
                step_order,
                status: status.to_string(),
                input_fingerprint_sha256: None,
                input_fingerprint_json: Some(json!({"large": "input"})),
                checkpoint_json: Some(json!({"large": "checkpoint"})),
                artifacts_json: Some(json!({"large": "artifact"})),
                generation_run_ids_json: Some(json!({"large": "runs"})),
                error_message: None,
                completed: status == WORKFLOW_STATUS_COMPLETED,
            },
        )
        .await
        .expect("checkpoint must be inserted");
    }

    assert_eq!(
        workflow_checkpoint_step_statuses(&db, run.id)
            .await
            .expect("narrow checkpoint query must succeed"),
        vec![
            ("step-1".to_string(), WORKFLOW_STATUS_RUNNING.to_string()),
            ("step-2".to_string(), WORKFLOW_STATUS_COMPLETED.to_string()),
        ]
    );
}

#[tokio::test]
async fn workflow_detail_returns_generation_run_summary_fields() {
    let db = a4n_migration::test_support::connect_test_database().await;
    let workflow = upsert_workflow_run(
        &db,
        workflow_input(
            &format!("workflow-detail-summary:{}", uuid::Uuid::new_v4()),
            "detail",
            WORKFLOW_STATUS_COMPLETED,
        ),
    )
    .await
    .expect("workflow run must be inserted");
    let now = Utc::now().naive_utc();
    let generation = generation_run::ActiveModel {
        project_id: Set(1),
        root_project_id: Set(1),
        workflow: Set("detail-workflow".to_string()),
        step_key: Set("detail-step".to_string()),
        title: Set("详情测试".to_string()),
        model_kind: Set("test".to_string()),
        status: Set(WORKFLOW_STATUS_COMPLETED.to_string()),
        workflow_run_id: Set(Some(workflow.id)),
        input_fingerprint_sha256: Set(Some("fingerprint".to_string())),
        input_fingerprint_json: Set(Some(json!({"input": true}))),
        raw_response_json: Set(Some(json!({"raw": true}))),
        parsed_output_json: Set(Some(json!({"value": 1}))),
        validation_json: Set(Some(json!({"valid": true}))),
        artifact_refs_json: Set(Some(json!({"asset_id": 7}))),
        rendered_prompts_json: Set(Some(json!({"unused": "large"}))),
        response_summary_json: Set(Some(json!({"output": "ok"}))),
        attempt: Set(2),
        created_at: Set(now),
        completed_at: Set(Some(now)),
        ..Default::default()
    }
    .insert(&db)
    .await
    .expect("generation run must be inserted");

    let detail = workflow_run_detail(
        &db,
        GetWorkflowRunInput {
            workflow_run_id: workflow.id,
            include_generation_runs: true,
        },
    )
    .await
    .expect("workflow detail query must succeed");
    assert_eq!(detail.generation_runs.len(), 1);
    let summary = &detail.generation_runs[0];
    assert_eq!(summary.id, generation.id);
    assert_eq!(summary.workflow_run_id, Some(workflow.id));
    assert_eq!(summary.workflow, "detail-workflow");
    assert_eq!(summary.parsed_output_json, Some(json!({"value": 1})));
    assert_eq!(summary.response_summary_json, Some(json!({"output": "ok"})));
    assert_eq!(summary.attempt, 2);
}
