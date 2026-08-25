use a4n_core::models::generation::{generation_run, generation_run_target};
use a4n_db::generation::{
    attach_runs_to_workflow_for_target, cancel_runs_for_target, cancel_runs_for_targets,
    generation_run_exists_for_target_output,
};
use a4n_db::project::{CreateProjectRecordInput, create_project_record};
use a4n_db::workflow::{UpsertWorkflowRunInput, WORKFLOW_STATUS_RUNNING, upsert_workflow_run};
use chrono::Utc;
use sea_orm::{ActiveModelTrait, EntityTrait, Set};
use serde_json::json;

const RUN_STATUS_CANCELLED: &str = "cancelled";

#[tokio::test]
async fn cancel_runs_for_targets_updates_all_targets_and_preserves_counts() {
    let db = a4n_migration::test_support::connect_test_database().await;
    let project = create_project_record(
        &db,
        CreateProjectRecordInput {
            user_id: 1,
            title: format!("批量取消 generation run {}", uuid::Uuid::new_v4()),
            description: String::new(),
            project_type: "novel_long".to_string(),
        },
    )
    .await
    .expect("test project must be created");
    let now = Utc::now().naive_utc();
    let target_ids = [
        project.id * 10 + 1,
        project.id * 10 + 2,
        project.id * 10 + 3,
    ];

    let mut run_ids = Vec::new();
    for (index, target_id) in target_ids.iter().copied().enumerate() {
        let run = generation_run::ActiveModel {
            project_id: Set(project.id),
            root_project_id: Set(project.id),
            workflow: Set("batch-cancel-test".to_string()),
            step_key: Set(format!("video-{index}")),
            title: Set("批量取消测试".to_string()),
            model_kind: Set("test".to_string()),
            status: Set(if index == 2 {
                RUN_STATUS_CANCELLED.to_string()
            } else {
                "running".to_string()
            }),
            attempt: Set(1),
            created_at: Set(now),
            ..Default::default()
        }
        .insert(&db)
        .await
        .expect("generation run must be inserted");
        generation_run_target::ActiveModel {
            generation_run_id: Set(run.id),
            target_kind: Set("generated_video".to_string()),
            target_table: Set("generated_video".to_string()),
            target_id: Set(target_id),
            role: Set("output".to_string()),
            created_at: Set(now),
            ..Default::default()
        }
        .insert(&db)
        .await
        .expect("generation run target must be inserted");
        run_ids.push(run.id);
    }

    let workflow = upsert_workflow_run(
        &db,
        UpsertWorkflowRunInput {
            business_key: format!("generation-attach:{}", uuid::Uuid::new_v4()),
            workflow_kind: "video_provider_job".to_string(),
            project_id: Some(project.id),
            root_project_id: Some(project.id),
            source_snapshot_id: None,
            scope_table: None,
            scope_id: None,
            chapter_id: None,
            mode: None,
            input_fingerprint_json: Some(json!({"provider": "test"})),
            status: WORKFLOW_STATUS_RUNNING.to_string(),
            current_step_key: Some("provider_job".to_string()),
            active_task_id: None,
            can_resume: false,
            resume_block_reason: None,
            error_message: None,
        },
    )
    .await
    .expect("workflow run must be inserted");
    assert_eq!(
        attach_runs_to_workflow_for_target(
            &db,
            a4n_db::generation::AttachGenerationRunsToWorkflowInput {
                target_table: "generated_video".to_string(),
                target_id: target_ids[0],
                workflow_run_id: workflow.id,
                call_key: Some("provider_job".to_string()),
                input_fingerprint_sha256: Some("fingerprint".to_string()),
                input_fingerprint_json: Some(json!({"provider": "test", "attached": true})),
                artifact_refs_json: Some(json!({"video_id": target_ids[0]})),
            },
        )
        .await
        .expect("workflow attach must succeed"),
        1
    );
    let attached_run = generation_run::Entity::find_by_id(run_ids[0])
        .one(&db)
        .await
        .expect("attached run query must succeed")
        .expect("attached run must exist");
    assert_eq!(attached_run.workflow_run_id, Some(workflow.id));
    assert_eq!(attached_run.call_key.as_deref(), Some("provider_job"));

    let counts = cancel_runs_for_targets(
        &db,
        "generated_video",
        &target_ids,
        "workflow cancelled".to_string(),
    )
    .await
    .expect("batch cancellation must succeed");

    assert_eq!(
        counts,
        vec![(target_ids[0], 1), (target_ids[1], 1), (target_ids[2], 1)]
    );
    let runs = generation_run::Entity::find_by_id(run_ids[0])
        .one(&db)
        .await
        .expect("run query must succeed")
        .expect("run must exist");
    assert_eq!(runs.status, RUN_STATUS_CANCELLED);
    assert_eq!(runs.error_message.as_deref(), Some("workflow cancelled"));

    assert_eq!(
        cancel_runs_for_target(
            &db,
            "generated_video",
            target_ids[2],
            "retry cancel".to_string()
        )
        .await
        .expect("single cancellation must preserve associated-run count"),
        1
    );
}

#[tokio::test]
async fn generation_run_target_output_match_uses_response_summary() {
    let db = a4n_migration::test_support::connect_test_database().await;
    let project = create_project_record(
        &db,
        CreateProjectRecordInput {
            user_id: 1,
            title: format!("generation target output {}", uuid::Uuid::new_v4()),
            description: String::new(),
            project_type: "novel_long".to_string(),
        },
    )
    .await
    .expect("test project must be created");
    let now = Utc::now().naive_utc();
    let run = generation_run::ActiveModel {
        project_id: Set(project.id),
        root_project_id: Set(project.id),
        workflow: Set("target-output-test".to_string()),
        step_key: Set("prompt-step".to_string()),
        title: Set("目标输出测试".to_string()),
        model_kind: Set("test".to_string()),
        status: Set("completed".to_string()),
        response_summary_json: Set(Some(json!({"output": "命中结果"}))),
        attempt: Set(1),
        created_at: Set(now),
        ..Default::default()
    }
    .insert(&db)
    .await
    .expect("generation run must be inserted");
    generation_run_target::ActiveModel {
        generation_run_id: Set(run.id),
        target_kind: Set("chapter_video_prompt".to_string()),
        target_table: Set("chapter_video_prompt".to_string()),
        target_id: Set(7001),
        role: Set("output".to_string()),
        created_at: Set(now),
        ..Default::default()
    }
    .insert(&db)
    .await
    .expect("generation run target must be inserted");

    assert!(
        generation_run_exists_for_target_output(
            &db,
            "chapter_video_prompt",
            7001,
            "prompt-step",
            "命中结果",
        )
        .await
        .expect("target output query must succeed")
    );
    assert!(
        !generation_run_exists_for_target_output(
            &db,
            "chapter_video_prompt",
            7001,
            "prompt-step",
            "其它结果",
        )
        .await
        .expect("target output query must succeed")
    );
}
