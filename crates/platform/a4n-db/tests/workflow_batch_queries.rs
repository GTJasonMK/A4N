use a4n_db::workflow::{
    CompletedLlmCallCheckpoint, CompletedLlmCallCheckpointInput, GetWorkflowRunInput,
    UpsertWorkflowCheckpointInput, UpsertWorkflowRunInput, WorkflowProgressEventInput,
    WorkflowWriteError, cancel_workflow_run_persistence,
    invalidate_chapter_analysis_after_source_change,
    latest_workflow_run_recovery_records_by_business_keys, latest_workflow_runs_by_business_keys,
    load_completed_llm_call_checkpoint, mark_workflow_status, persist_workflow_progress_snapshot,
    record_workflow_progress_event, restart_interrupted_workflow_run,
    resume_or_upsert_workflow_run, resume_workflow_run_with_active_task,
    save_completed_llm_call_checkpoint, upsert_workflow_checkpoint, upsert_workflow_run,
    workflow_checkpoint_statuses_for_runs, workflow_checkpoint_step_statuses,
    workflow_progress_events, workflow_run_detail, workflow_run_model_by_id,
    workflow_run_models_by_ids, workflow_run_statuses_by_ids, workflow_run_summary_by_id,
};
use a4n_db_models::models::{
    generation::{generation_run, workflow_run},
    task::task_queue,
};
use a4n_domain::workflow::{
    WORKFLOW_STATUS_CANCELLED, WORKFLOW_STATUS_COMPLETED, WORKFLOW_STATUS_FAILED,
    WORKFLOW_STATUS_INTERRUPTED, WORKFLOW_STATUS_RUNNING, WORKFLOW_STATUS_SUPERSEDED,
};
use chrono::Utc;
use sea_orm::{ActiveModelTrait, EntityTrait, Set};
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

fn shared_db_handle(db: &sea_orm::DatabaseConnection) -> a4n_db::DbHandle {
    a4n_db::DbHandle::new(std::sync::Arc::new(db.clone()))
}

#[tokio::test]
async fn cancelling_a_failed_resumable_workflow_discards_its_progress() {
    let db = a4n_migration::test_support::connect_test_database().await;
    let run = upsert_workflow_run(
        &db,
        workflow_input(
            &format!("discard-failed:{}", uuid::Uuid::new_v4()),
            "source",
            WORKFLOW_STATUS_RUNNING,
        ),
    )
    .await
    .unwrap();
    let snapshot = json!({"completed_scenes": [1]});
    persist_workflow_progress_snapshot(&db, run.id, snapshot.clone())
        .await
        .unwrap();
    mark_workflow_status(
        &db,
        run.id,
        WORKFLOW_STATUS_FAILED,
        true,
        Some("模型输出无效"),
    )
    .await
    .unwrap();

    let result = cancel_workflow_run_persistence(&db, run.id, "删除未完成生成")
        .await
        .unwrap();
    assert!(!result.already_terminal);
    let cancelled = workflow_run_model_by_id(&db, run.id).await.unwrap();
    assert_eq!(cancelled.status, WORKFLOW_STATUS_CANCELLED);
    assert!(!cancelled.can_resume);
    assert_eq!(cancelled.progress_json.unwrap()["snapshot"], snapshot);
    assert!(
        cancel_workflow_run_persistence(&db, run.id, "重复删除")
            .await
            .unwrap()
            .already_terminal
    );
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
async fn chapter_source_change_invalidates_derived_checkpoints_but_keeps_llm_calls() {
    let db = a4n_migration::test_support::connect_test_database().await;
    let project = a4n_db::project::create_project_record(
        &db,
        a4n_db::project::CreateProjectRecordInput {
            user_id: 1,
            title: format!("workflow invalidation {}", uuid::Uuid::new_v4()),
            description: String::new(),
            project_type: "novel_long".to_string(),
        },
    )
    .await
    .expect("project must be created");
    let chapters = futures::future::try_join_all((1..=2).map(|chapter_number| {
        a4n_db::chapter_content::import_chapter_content_record(
            &db,
            a4n_domain::chapter::ImportChapterContentInput {
                project_id: project.id,
                chapter_number,
                content: format!("第{chapter_number}章正文"),
                title: None,
            },
            "workflow invalidation test",
        )
    }))
    .await
    .expect("chapters must be created");

    let mut runs = Vec::new();
    for chapter in &chapters {
        let run = upsert_workflow_run(
            &db,
            UpsertWorkflowRunInput {
                business_key: format!("chapter_analysis:{}:{}", project.id, chapter.id),
                workflow_kind: "chapter_analysis".to_string(),
                project_id: Some(project.id),
                root_project_id: None,
                source_snapshot_id: None,
                scope_table: Some("chapter".to_string()),
                scope_id: Some(chapter.id),
                chapter_id: Some(chapter.id),
                mode: None,
                input_fingerprint_json: Some(json!({"chapter_id": chapter.id})),
                status: WORKFLOW_STATUS_RUNNING.to_string(),
                current_step_key: Some("chapter_analysis.refresh_status".to_string()),
                active_task_id: None,
                can_resume: true,
                resume_block_reason: None,
                error_message: None,
            },
        )
        .await
        .expect("chapter analysis run must be created");
        for step_key in [
            "chapter_analysis.facts",
            "chapter_analysis.sync_outputs",
            "chapter_analysis.character_sync.profile.7",
            "scene_profile_sync.chapter.2.scenes",
        ] {
            upsert_workflow_checkpoint(
                &shared_db_handle(&db),
                UpsertWorkflowCheckpointInput {
                    workflow_run_id: run.id,
                    step_key: step_key.to_string(),
                    step_order: 1,
                    status: WORKFLOW_STATUS_COMPLETED.to_string(),
                    input_fingerprint_sha256: None,
                    input_fingerprint_json: Some(json!({"chapter_id": chapter.id})),
                    checkpoint_json: Some(json!({"completed": true})),
                    artifacts_json: None,
                    generation_run_ids_json: None,
                    error_message: None,
                    completed: true,
                },
            )
            .await
            .expect("checkpoint must be created");
        }
        runs.push(run);
    }

    invalidate_chapter_analysis_after_source_change(&db, project.id, 1, Some(runs[0].id))
        .await
        .expect("chapter analysis invalidation must succeed");

    let preserved = workflow_run_model_by_id(&db, runs[0].id).await.unwrap();
    assert_eq!(preserved.status, WORKFLOW_STATUS_RUNNING);
    let invalidated = workflow_run_model_by_id(&db, runs[1].id).await.unwrap();
    assert_eq!(invalidated.status, WORKFLOW_STATUS_INTERRUPTED);
    assert!(invalidated.can_resume);
    let remaining = workflow_checkpoint_step_statuses(&db, runs[1].id)
        .await
        .unwrap();
    assert_eq!(
        remaining,
        vec![(
            "chapter_analysis.facts".to_string(),
            WORKFLOW_STATUS_COMPLETED.to_string(),
        )]
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
        WORKFLOW_STATUS_INTERRUPTED,
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
async fn starting_after_interruption_supersedes_old_run_and_creates_a_new_one() {
    let db = a4n_migration::test_support::connect_test_database().await;
    let suffix = uuid::Uuid::new_v4();
    let key = format!("workflow-restart:{suffix}");
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
        WORKFLOW_STATUS_INTERRUPTED,
        true,
        Some("应用重启后中断"),
    )
    .await
    .expect("workflow must become interrupted");

    let mut input = workflow_input(&key, &fingerprint, WORKFLOW_STATUS_RUNNING);
    input.active_task_id = Some("fresh-start-task".to_string());
    let restarted = restart_interrupted_workflow_run(&db, initial.id, input)
        .await
        .expect("explicit start must replace an interrupted workflow");
    let superseded = workflow_run_model_by_id(&db, initial.id)
        .await
        .expect("superseded workflow must be readable");

    assert_ne!(restarted.id, initial.id);
    assert_eq!(superseded.status, WORKFLOW_STATUS_SUPERSEDED);
    assert!(!superseded.can_resume);
    assert!(superseded.active_task_id.is_none());
    assert_eq!(restarted.status, WORKFLOW_STATUS_RUNNING);
    assert_eq!(
        restarted.active_task_id.as_deref(),
        Some("fresh-start-task")
    );
}

#[tokio::test]
async fn resume_workflow_binds_unowned_task_and_rejects_completed_other_owner() {
    let db = a4n_migration::test_support::connect_test_database().await;
    let suffix = uuid::Uuid::new_v4();
    let key = format!("workflow-resume-task:{suffix}");
    let run = upsert_workflow_run(
        &db,
        workflow_input(&key, "fingerprint", WORKFLOW_STATUS_RUNNING),
    )
    .await
    .expect("initial workflow must be inserted");
    mark_workflow_status(
        &db,
        run.id,
        WORKFLOW_STATUS_INTERRUPTED,
        true,
        Some("应用重启后中断"),
    )
    .await
    .expect("workflow must become resumably interrupted");

    let task_id = uuid::Uuid::new_v4().to_string();
    task_queue::ActiveModel {
        id: Set(task_id.clone()),
        task_type: Set("llm_call".to_string()),
        priority: Set("medium".to_string()),
        status: Set("pending".to_string()),
        project_id: Set(None),
        root_project_id: Set(None),
        source_snapshot_id: Set(None),
        task_scope: Set(None),
        workflow_run_id: Set(None),
        payload: Set(None),
        progress: Set(None),
        error_message: Set(None),
        dag_id: Set(None),
        created_at: Set(Utc::now().naive_utc()),
        started_at: Set(None),
        completed_at: Set(None),
    }
    .insert(&db)
    .await
    .expect("task queue row must be inserted");

    let resumed = resume_workflow_run_with_active_task(&db, run.id, &task_id)
        .await
        .expect("workflow must resume with its active task");
    let task = task_queue::Entity::find_by_id(&task_id)
        .one(&db)
        .await
        .expect("task query must succeed")
        .expect("task must exist");

    assert_eq!(resumed.active_task_id.as_deref(), Some(task.id.as_str()));
    assert_eq!(task.workflow_run_id, Some(run.id));

    mark_workflow_status(
        &db,
        run.id,
        WORKFLOW_STATUS_INTERRUPTED,
        true,
        Some("再次中断以验证任务所有权"),
    )
    .await
    .expect("workflow must become resumably interrupted again");
    let previous = upsert_workflow_run(
        &db,
        workflow_input(
            &format!("workflow-previous-owner:{suffix}"),
            "previous-owner",
            WORKFLOW_STATUS_COMPLETED,
        ),
    )
    .await
    .expect("previous completed workflow must be inserted");
    let mut owned_by_previous: task_queue::ActiveModel = task.into();
    owned_by_previous.workflow_run_id = Set(Some(previous.id));
    owned_by_previous.update(&db).await.unwrap();

    let error = resume_workflow_run_with_active_task(&db, run.id, &task_id)
        .await
        .expect_err("an active task must not be stolen from another workflow");
    assert!(matches!(error, WorkflowWriteError::StatusConflict { .. }));
    let unchanged_task = task_queue::Entity::find_by_id(task_id)
        .one(&db)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(unchanged_task.workflow_run_id, Some(previous.id));
    let unchanged_run = workflow_run_model_by_id(&db, run.id).await.unwrap();
    assert_eq!(unchanged_run.status, WORKFLOW_STATUS_INTERRUPTED);
    assert!(unchanged_run.active_task_id.is_none());
}

#[tokio::test]
async fn interrupted_workflow_preserves_cancellation_task_without_progress() {
    let db = a4n_migration::test_support::connect_test_database().await;
    let key = format!("workflow-cancel-task:{}", uuid::Uuid::new_v4());
    let task_id = uuid::Uuid::new_v4().to_string();
    let mut input = workflow_input(&key, "fingerprint", WORKFLOW_STATUS_RUNNING);
    input.active_task_id = Some(task_id.clone());
    let run = upsert_workflow_run(&db, input).await.unwrap();

    mark_workflow_status(
        &db,
        run.id,
        WORKFLOW_STATUS_INTERRUPTED,
        true,
        Some("应用重启后中断"),
    )
    .await
    .unwrap();
    let interrupted = workflow_run_summary_by_id(&db, run.id).await.unwrap();
    assert!(interrupted.progress.is_none());

    let cancelled = cancel_workflow_run_persistence(&db, run.id, "用户取消工作流")
        .await
        .unwrap();
    assert_eq!(cancelled.active_task_id.as_deref(), Some(task_id.as_str()));
}

fn completed_call(run_id: i64, call_key: &str) -> CompletedLlmCallCheckpointInput {
    CompletedLlmCallCheckpointInput {
        workflow_run_id: run_id,
        call_key: call_key.to_string(),
        call_order: 1,
        input_fingerprint_json: json!({"request": "stable"}),
        checkpoint: CompletedLlmCallCheckpoint {
            parsed_output: json!({"events": []}),
            response_content: "{\"events\":[]}".to_string(),
            usage: None,
            finish_reason: Some("stop".to_string()),
            artifacts_json: None,
        },
    }
}

#[tokio::test]
async fn workflow_snapshot_survives_progress_reads_updates_and_cancellation() {
    let db = a4n_migration::test_support::connect_test_database().await;
    for with_progress in [false, true] {
        let key = format!("workflow-snapshot:{}", uuid::Uuid::new_v4());
        let active_task_id = uuid::Uuid::new_v4().to_string();
        let mut run_input = workflow_input(&key, "snapshot", WORKFLOW_STATUS_RUNNING);
        run_input.active_task_id = Some(active_task_id.clone());
        let run = upsert_workflow_run(&db, run_input).await.unwrap();
        upsert_workflow_checkpoint(
            &shared_db_handle(&db),
            UpsertWorkflowCheckpointInput {
                workflow_run_id: run.id,
                step_key: "video-generation".to_string(),
                step_order: 1,
                status: WORKFLOW_STATUS_INTERRUPTED.to_string(),
                input_fingerprint_sha256: None,
                input_fingerprint_json: Some(json!({"generated_video_id": 501})),
                checkpoint_json: None,
                artifacts_json: Some(json!({"generated_video_id": 502})),
                generation_run_ids_json: None,
                error_message: None,
                completed: false,
            },
        )
        .await
        .unwrap();
        let event = WorkflowProgressEventInput {
            workflow_run_id: run.id,
            task_id: None,
            current: 1,
            total: 4,
            message: "处理中",
            meta: None,
        };
        if with_progress {
            record_workflow_progress_event(&db, event.clone())
                .await
                .unwrap();
        }
        let snapshot = json!({"version": 1, "completed_units": [7]});
        persist_workflow_progress_snapshot(&db, run.id, snapshot.clone())
            .await
            .unwrap();
        let summary = workflow_run_summary_by_id(&db, run.id).await.unwrap();
        assert_eq!(summary.progress.is_some(), with_progress);
        if let Some(progress) = summary.progress {
            assert_eq!(progress.current, 1);
            assert_eq!(progress.percent, 25.0);
        }

        if with_progress {
            record_workflow_progress_event(
                &db,
                WorkflowProgressEventInput {
                    current: 2,
                    ..event.clone()
                },
            )
            .await
            .unwrap();
            let stored = workflow_run_model_by_id(&db, run.id).await.unwrap();
            assert_eq!(stored.progress_json.unwrap()["snapshot"], snapshot);
        }
        mark_workflow_status(
            &db,
            run.id,
            WORKFLOW_STATUS_INTERRUPTED,
            true,
            Some("场景边界暂停"),
        )
        .await
        .unwrap();
        let snapshot = json!({"version": 1, "completed_units": [7, 8]});
        persist_workflow_progress_snapshot(&db, run.id, snapshot.clone())
            .await
            .unwrap();
        let detail = workflow_run_detail(
            &db,
            GetWorkflowRunInput {
                workflow_run_id: run.id,
                include_generation_runs: false,
                include_generation_run_count: false,
                include_checkpoint_details: true,
            },
        )
        .await
        .unwrap();
        assert_eq!(detail.run.status, WORKFLOW_STATUS_INTERRUPTED);
        assert_eq!(detail.run.progress.is_some(), with_progress);
        if let Some(progress) = detail.run.progress {
            assert_eq!(progress.current, 2);
            assert_eq!(progress.percent, 50.0);
        }

        let result = cancel_workflow_run_persistence(&db, run.id, "用户取消工作流")
            .await
            .unwrap();
        assert!(!result.already_terminal);
        assert!(result.cancelled);
        assert_eq!(
            result.active_task_id.as_deref(),
            Some(active_task_id.as_str())
        );
        assert_eq!(result.interrupted_generated_video_ids, vec![501, 502]);
        let cancelled = workflow_run_summary_by_id(&db, run.id).await.unwrap();
        assert_eq!(cancelled.status, WORKFLOW_STATUS_CANCELLED);
        assert!(!cancelled.can_resume);
        assert!(cancelled.active_task_id.is_none());
        assert_eq!(cancelled.progress.unwrap().message, "用户取消工作流");
        let stored = workflow_run_model_by_id(&db, run.id).await.unwrap();
        let mut progress_json = stored.progress_json.clone().unwrap();
        assert_eq!(progress_json["snapshot"], snapshot);
        let _ = progress_json
            .as_object_mut()
            .unwrap()
            .remove("cancellation_active_task_id");
        let mut stored: workflow_run::ActiveModel = stored.into();
        stored.progress_json = Set(Some(progress_json));
        stored.update(&db).await.unwrap();
        assert!(matches!(
            persist_workflow_progress_snapshot(&db, run.id, snapshot).await,
            Err(WorkflowWriteError::StatusConflict { .. })
        ));
        assert!(matches!(
            record_workflow_progress_event(&db, event).await,
            Err(WorkflowWriteError::StatusConflict { .. })
        ));
        let retry = cancel_workflow_run_persistence(&db, run.id, "用户取消工作流")
            .await
            .unwrap();
        assert!(retry.already_terminal);
        assert!(retry.cancelled);
        assert_eq!(
            retry.active_task_id.as_deref(),
            Some(active_task_id.as_str())
        );
        assert_eq!(retry.interrupted_generated_video_ids, vec![501, 502]);

        let mut stored: a4n_db_models::models::generation::workflow_run::ActiveModel =
            workflow_run_model_by_id(&db, run.id).await.unwrap().into();
        let mut progress = stored
            .progress_json
            .clone()
            .take()
            .flatten()
            .expect("cancelled workflow progress must exist");
        progress
            .as_object_mut()
            .expect("workflow progress must be an object")
            .remove("cancellation_active_task_id");
        stored.progress_json = Set(Some(progress));
        stored.update(&db).await.unwrap();
        let historical = cancel_workflow_run_persistence(&db, run.id, "用户取消工作流")
            .await
            .unwrap();
        assert_eq!(
            historical.active_task_id.as_deref(),
            Some(active_task_id.as_str())
        );
    }
}

#[tokio::test]
async fn resume_binds_activity_task_to_workflow() {
    let db = a4n_migration::test_support::connect_test_database().await;
    let key = format!("workflow-activity-task:{}", uuid::Uuid::new_v4());
    let mut input = workflow_input(&key, "activity", WORKFLOW_STATUS_RUNNING);
    input.active_task_id = None;
    let run = upsert_workflow_run(&db, input).await.unwrap();
    mark_workflow_status(
        &db,
        run.id,
        WORKFLOW_STATUS_INTERRUPTED,
        true,
        Some("应用重启后中断"),
    )
    .await
    .unwrap();

    let task_id = format!("activity-task:{}", uuid::Uuid::new_v4());
    task_queue::ActiveModel {
        id: Set(task_id.clone()),
        task_type: Set("llm_call".to_string()),
        priority: Set("medium".to_string()),
        status: Set("pending".to_string()),
        project_id: Set(None),
        root_project_id: Set(None),
        source_snapshot_id: Set(None),
        task_scope: Set(Some("activity".to_string())),
        workflow_run_id: Set(None),
        payload: Set(None),
        progress: Set(None),
        error_message: Set(None),
        dag_id: Set(None),
        created_at: Set(Utc::now().naive_utc()),
        started_at: Set(None),
        completed_at: Set(None),
    }
    .insert(&db)
    .await
    .unwrap();

    let resumed = resume_workflow_run_with_active_task(&db, run.id, &task_id)
        .await
        .unwrap();
    assert_eq!(resumed.active_task_id.as_deref(), Some(task_id.as_str()));
    let task = task_queue::Entity::find_by_id(task_id)
        .one(&db)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(task.workflow_run_id, Some(run.id));
}

#[tokio::test]
async fn cancelled_retry_inherits_completed_calls_without_reopening_cancelled_run() {
    let db = a4n_migration::test_support::connect_test_database().await;
    let db_handle = a4n_db::DbHandle::new(std::sync::Arc::new(db.clone()));
    let key = format!("character-call-resume:{}", uuid::Uuid::new_v4());
    let mut input = workflow_input(&key, "same", WORKFLOW_STATUS_RUNNING);
    input.workflow_kind = "character_profile_sync".to_string();
    let original = resume_or_upsert_workflow_run(&db, input.clone())
        .await
        .unwrap();
    save_completed_llm_call_checkpoint(
        &shared_db_handle(&db),
        completed_call(original.id, "evidence"),
    )
    .await
    .unwrap();
    for (step_key, status, payload) in [
        (
            "business-write",
            WORKFLOW_STATUS_COMPLETED,
            json!({"completed": true}),
        ),
        (
            "in-flight",
            WORKFLOW_STATUS_RUNNING,
            json!({"response_content": "partial"}),
        ),
    ] {
        upsert_workflow_checkpoint(
            &shared_db_handle(&db),
            UpsertWorkflowCheckpointInput {
                workflow_run_id: original.id,
                step_key: step_key.to_string(),
                step_order: 2,
                status: status.to_string(),
                input_fingerprint_sha256: None,
                input_fingerprint_json: None,
                checkpoint_json: Some(payload),
                artifacts_json: None,
                generation_run_ids_json: None,
                error_message: None,
                completed: status == WORKFLOW_STATUS_COMPLETED,
            },
        )
        .await
        .unwrap();
    }
    cancel_workflow_run_persistence(&db, original.id, "用户取消工作流")
        .await
        .unwrap();
    input.active_task_id = Some("new-attempt".to_string());
    let restarted = resume_or_upsert_workflow_run(&db, input.clone())
        .await
        .unwrap();
    assert_ne!(restarted.id, original.id);
    assert_eq!(
        workflow_checkpoint_step_statuses(&db, restarted.id)
            .await
            .unwrap(),
        vec![(
            "evidence".to_string(),
            WORKFLOW_STATUS_COMPLETED.to_string()
        )]
    );
    let restored = load_completed_llm_call_checkpoint(
        &db_handle,
        restarted.id,
        "evidence",
        &json!({"request": "stable"}),
    )
    .await
    .unwrap()
    .expect("successful model response must survive explicit cancellation");
    assert_eq!(restored.response_content, "{\"events\":[]}");
    assert!(
        load_completed_llm_call_checkpoint(
            &db_handle,
            restarted.id,
            "evidence",
            &json!({"request": "changed"})
        )
        .await
        .unwrap()
        .is_none()
    );
    let cancelled = workflow_run_model_by_id(&db, original.id).await.unwrap();
    assert_eq!(cancelled.status, WORKFLOW_STATUS_CANCELLED);
    assert!(!cancelled.can_resume);
    assert!(
        matches!(
            save_completed_llm_call_checkpoint(
                &shared_db_handle(&db),
                completed_call(original.id, "late-response"),
            )
                .await,
            Err(WorkflowWriteError::StatusConflict { current, .. })
                if current == WORKFLOW_STATUS_CANCELLED
        ),
        "late responses must not write into the cancelled owner"
    );
    assert!(
        matches!(
            resume_or_upsert_workflow_run(&db, input).await,
            Err(WorkflowWriteError::StatusConflict { current, .. })
                if current == WORKFLOW_STATUS_RUNNING
        ),
        "concurrent admission must remain fenced"
    );
}

#[tokio::test]
async fn cancelled_retry_does_not_inherit_unrelated_or_fresh_start_calls() {
    let db = a4n_migration::test_support::connect_test_database().await;
    for mode in [
        "changed_input",
        "changed_kind",
        "resume_disabled",
        "already_completed",
    ] {
        let key = format!("call-restart-{mode}:{}", uuid::Uuid::new_v4());
        let mut input = workflow_input(&key, "same", WORKFLOW_STATUS_RUNNING);
        let original = resume_or_upsert_workflow_run(&db, input.clone())
            .await
            .unwrap();
        save_completed_llm_call_checkpoint(
            &shared_db_handle(&db),
            completed_call(original.id, "saved-call"),
        )
        .await
        .unwrap();
        if mode == "already_completed" {
            mark_workflow_status(&db, original.id, WORKFLOW_STATUS_COMPLETED, false, None)
                .await
                .unwrap();
        } else {
            cancel_workflow_run_persistence(&db, original.id, "用户取消工作流")
                .await
                .unwrap();
        }
        match mode {
            "changed_input" => {
                input.input_fingerprint_json = Some(json!({"fingerprint": "changed"}))
            }
            "changed_kind" => input.workflow_kind = "other_workflow".to_string(),
            "resume_disabled" => input.can_resume = false,
            "already_completed" => {}
            _ => unreachable!(),
        }
        let next = resume_or_upsert_workflow_run(&db, input).await.unwrap();
        assert_ne!(next.id, original.id);
        assert!(
            workflow_checkpoint_step_statuses(&db, next.id)
                .await
                .unwrap()
                .is_empty(),
            "{mode}"
        );
    }
}

#[tokio::test]
async fn new_attempt_after_failure_inherits_validated_calls() {
    let db = a4n_migration::test_support::connect_test_database().await;
    let db_handle = a4n_db::DbHandle::new(std::sync::Arc::new(db.clone()));
    let key = format!("failed-call-retry:{}", uuid::Uuid::new_v4());
    let input = workflow_input(&key, "same", WORKFLOW_STATUS_RUNNING);
    let original = upsert_workflow_run(&db, input.clone()).await.unwrap();
    let mut call = completed_call(original.id, "first-call");
    call.input_fingerprint_json = json!({"request": {
        "messages": [{"role": "user", "content": "stable input"}],
        "trace_context": {"workflow_run_id": original.id}
    }});
    let mut expected_fingerprint = call.input_fingerprint_json.clone();
    save_completed_llm_call_checkpoint(&shared_db_handle(&db), call)
        .await
        .unwrap();
    mark_workflow_status(
        &db,
        original.id,
        WORKFLOW_STATUS_FAILED,
        true,
        Some("second call validation failed"),
    )
    .await
    .unwrap();
    let retry = upsert_workflow_run(&db, input.clone()).await.unwrap();
    assert_ne!(retry.id, original.id);
    expected_fingerprint["request"]["trace_context"]["workflow_run_id"] = json!(retry.id);
    assert!(
        load_completed_llm_call_checkpoint(
            &db_handle,
            retry.id,
            "first-call",
            &expected_fingerprint,
        )
        .await
        .unwrap()
        .is_some()
    );
    assert_eq!(
        workflow_run_model_by_id(&db, original.id)
            .await
            .unwrap()
            .status,
        WORKFLOW_STATUS_FAILED
    );
    assert!(
        save_completed_llm_call_checkpoint(
            &shared_db_handle(&db),
            completed_call(original.id, "late-call"),
        )
        .await
        .is_err()
    );

    mark_workflow_status(&db, retry.id, WORKFLOW_STATUS_COMPLETED, false, None)
        .await
        .unwrap();
    let fresh = upsert_workflow_run(&db, input).await.unwrap();
    assert!(
        workflow_checkpoint_step_statuses(&db, fresh.id)
            .await
            .unwrap()
            .is_empty()
    );
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
            &shared_db_handle(&db),
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
            &shared_db_handle(&db),
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
            include_generation_run_count: false,
            include_checkpoint_details: true,
        },
    )
    .await
    .expect("workflow detail query must succeed");
    assert_eq!(detail.generation_runs.len(), 1);
    assert_eq!(detail.generation_run_count, 1);
    let summary = &detail.generation_runs[0];
    assert_eq!(summary.id, generation.id);
    assert_eq!(summary.workflow_run_id, Some(workflow.id));
    assert_eq!(summary.workflow, "detail-workflow");
    assert_eq!(summary.parsed_output_json, Some(json!({"value": 1})));
    assert_eq!(summary.response_summary_json, Some(json!({"output": "ok"})));
    assert_eq!(summary.attempt, 2);

    let additional_runs: Vec<_> = (0..300)
        .map(|index| generation_run::ActiveModel {
            project_id: Set(1),
            root_project_id: Set(1),
            workflow: Set("detail-workflow".to_string()),
            step_key: Set(format!("detail-step-{index}")),
            title: Set(format!("Generation run {index}")),
            model_kind: Set("test".to_string()),
            status: Set(WORKFLOW_STATUS_COMPLETED.to_string()),
            workflow_run_id: Set(Some(workflow.id)),
            attempt: Set(1),
            created_at: Set(now + chrono::Duration::seconds(index)),
            completed_at: Set(Some(now + chrono::Duration::seconds(index))),
            ..Default::default()
        })
        .collect();
    generation_run::Entity::insert_many(additional_runs)
        .exec(&db)
        .await
        .expect("additional generation runs must be inserted");

    let count_only = workflow_run_detail(
        &db,
        GetWorkflowRunInput {
            workflow_run_id: workflow.id,
            include_generation_runs: false,
            include_generation_run_count: true,
            include_checkpoint_details: true,
        },
    )
    .await
    .expect("count-only workflow detail query must succeed");
    assert!(count_only.generation_runs.is_empty());
    assert_eq!(count_only.generation_run_count, 300);

    let full_detail_at_limit = workflow_run_detail(
        &db,
        GetWorkflowRunInput {
            workflow_run_id: workflow.id,
            include_generation_runs: true,
            include_generation_run_count: false,
            include_checkpoint_details: true,
        },
    )
    .await
    .expect("full workflow detail query must remain available");
    assert_eq!(full_detail_at_limit.generation_runs.len(), 300);
    assert_eq!(full_detail_at_limit.generation_run_count, 300);

    let without_generation_data = workflow_run_detail(
        &db,
        GetWorkflowRunInput {
            workflow_run_id: workflow.id,
            include_generation_runs: false,
            include_generation_run_count: false,
            include_checkpoint_details: true,
        },
    )
    .await
    .expect("workflow detail without generation data must succeed");
    assert_eq!(without_generation_data.generation_run_count, 0);
}

#[tokio::test]
async fn workflow_detail_can_omit_checkpoint_payloads_for_progress_polling() {
    let db = a4n_migration::test_support::connect_test_database().await;
    let workflow = upsert_workflow_run(
        &db,
        workflow_input(
            &format!(
                "workflow-detail-checkpoint-projection:{}",
                uuid::Uuid::new_v4()
            ),
            "checkpoint-projection",
            WORKFLOW_STATUS_RUNNING,
        ),
    )
    .await
    .expect("workflow run must be inserted");
    let artifacts = json!({ "provider_job_id": "job-123", "generated_video_id": 456 });
    let checkpoint_json = json!({ "payload": "large checkpoint data" });
    let input_fingerprint = json!({ "graph_version": 7 });
    let generation_run_ids = json!([81, 82]);
    upsert_workflow_checkpoint(
        &shared_db_handle(&db),
        UpsertWorkflowCheckpointInput {
            workflow_run_id: workflow.id,
            step_key: "canvas_node:7".to_string(),
            step_order: 7,
            status: WORKFLOW_STATUS_FAILED.to_string(),
            input_fingerprint_sha256: Some("checkpoint-fingerprint".to_string()),
            input_fingerprint_json: Some(input_fingerprint.clone()),
            checkpoint_json: Some(checkpoint_json.clone()),
            artifacts_json: Some(artifacts.clone()),
            generation_run_ids_json: Some(generation_run_ids.clone()),
            error_message: Some("provider failed".to_string()),
            completed: true,
        },
    )
    .await
    .expect("workflow checkpoint must be inserted");
    upsert_workflow_checkpoint(
        &shared_db_handle(&db),
        UpsertWorkflowCheckpointInput {
            workflow_run_id: workflow.id,
            step_key: "canvas_node:3".to_string(),
            step_order: 3,
            status: WORKFLOW_STATUS_COMPLETED.to_string(),
            input_fingerprint_sha256: None,
            input_fingerprint_json: None,
            checkpoint_json: None,
            artifacts_json: None,
            generation_run_ids_json: None,
            error_message: None,
            completed: true,
        },
    )
    .await
    .expect("second workflow checkpoint must be inserted");

    let progress_detail = workflow_run_detail(
        &db,
        GetWorkflowRunInput {
            workflow_run_id: workflow.id,
            include_generation_runs: false,
            include_generation_run_count: false,
            include_checkpoint_details: false,
        },
    )
    .await
    .expect("progress detail query must succeed");
    assert!(!progress_detail.checkpoint_details_included);
    assert_eq!(progress_detail.checkpoints.len(), 2);
    assert_eq!(progress_detail.checkpoints[0].step_key, "canvas_node:3");
    assert_eq!(progress_detail.checkpoints[1].step_key, "canvas_node:7");
    let progress_checkpoint = &progress_detail.checkpoints[1];
    assert_eq!(progress_checkpoint.step_key, "canvas_node:7");
    assert_eq!(progress_checkpoint.status, WORKFLOW_STATUS_FAILED);
    assert_eq!(progress_checkpoint.artifacts_json, Some(artifacts.clone()));
    assert_eq!(
        progress_checkpoint.error_message.as_deref(),
        Some("provider failed")
    );
    assert!(progress_checkpoint.input_fingerprint_sha256.is_none());
    assert!(progress_checkpoint.input_fingerprint_json.is_none());
    assert!(progress_checkpoint.checkpoint_json.is_none());
    assert!(progress_checkpoint.generation_run_ids_json.is_none());

    let full_detail = workflow_run_detail(
        &db,
        GetWorkflowRunInput {
            workflow_run_id: workflow.id,
            include_generation_runs: false,
            include_generation_run_count: false,
            include_checkpoint_details: true,
        },
    )
    .await
    .expect("full detail query must remain available");
    assert!(full_detail.checkpoint_details_included);
    assert_eq!(
        full_detail
            .checkpoints
            .iter()
            .map(|checkpoint| checkpoint.step_key.as_str())
            .collect::<Vec<_>>(),
        ["canvas_node:3", "canvas_node:7"]
    );
    assert_eq!(
        full_detail.checkpoints[1].input_fingerprint_json,
        Some(input_fingerprint)
    );
    assert_eq!(
        full_detail.checkpoints[1].checkpoint_json,
        Some(checkpoint_json)
    );
    assert_eq!(
        full_detail.checkpoints[1].generation_run_ids_json,
        Some(generation_run_ids)
    );
}

#[tokio::test]
async fn workflow_progress_events_after_event_id_returns_only_new_events() {
    let db = a4n_migration::test_support::connect_test_database().await;
    let workflow = upsert_workflow_run(
        &db,
        workflow_input(
            &format!("workflow-progress-cursor:{}", uuid::Uuid::new_v4()),
            "progress-cursor",
            WORKFLOW_STATUS_RUNNING,
        ),
    )
    .await
    .expect("workflow run must be inserted");
    for current in 1..=3 {
        let message = format!("progress-{current}");
        record_workflow_progress_event(
            &db,
            WorkflowProgressEventInput {
                workflow_run_id: workflow.id,
                task_id: Some("task-progress-cursor"),
                current,
                total: 3,
                message: &message,
                meta: None,
            },
        )
        .await
        .expect("workflow progress event must be recorded");
    }

    let full_history = workflow_progress_events(&db, workflow.id, 200, None)
        .await
        .expect("full workflow event history must load");
    let cursor = full_history[0]
        .id
        .parse::<i64>()
        .expect("workflow event id must be numeric");
    let incremental = workflow_progress_events(&db, workflow.id, 1, Some(cursor))
        .await
        .expect("incremental workflow events must load");
    assert_eq!(
        incremental
            .iter()
            .map(|event| event.id.as_str())
            .collect::<Vec<_>>(),
        [full_history[1].id.as_str()]
    );
    assert_eq!(incremental[0].message, "progress-2");
    let all_new_events = workflow_progress_events(&db, workflow.id, 200, Some(cursor))
        .await
        .expect("all new workflow events must preserve sequence");
    assert_eq!(
        all_new_events
            .iter()
            .map(|event| event.id.as_str())
            .collect::<Vec<_>>(),
        [full_history[1].id.as_str(), full_history[2].id.as_str()]
    );
    let latest_cursor = full_history[2]
        .id
        .parse::<i64>()
        .expect("workflow event id must be numeric");
    let unchanged = workflow_progress_events(&db, workflow.id, 200, Some(latest_cursor))
        .await
        .expect("empty incremental response must load");
    assert!(unchanged.is_empty());
}
