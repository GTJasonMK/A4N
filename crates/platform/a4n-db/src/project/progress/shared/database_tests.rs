use super::*;
use sea_orm::{ActiveModelTrait, Set};
use serde_json::json;

async fn fixture(project_count: usize, snapshot_repeats: usize) -> (DatabaseConnection, Vec<i64>) {
    let db = a4n_migration::test_support::connect_test_database().await;
    let now = chrono::Utc::now().naive_utc();
    let snapshot = json!({"text": "已冻结的镜头正文。".repeat(snapshot_repeats)});
    let mut ids = Vec::new();
    for index in 0..project_count {
        let project = crate::project::create_project_record(
            &db,
            crate::project::CreateProjectRecordInput {
                user_id: 1,
                title: format!("性能测试 {index}"),
                description: String::new(),
                project_type: "novel_long".into(),
            },
        )
        .await
        .unwrap();
        ids.push(project.id);
        for status in [WORKFLOW_STATUS_FAILED, WORKFLOW_STATUS_RUNNING] {
            let message = match index % 4 {
                0 => json!("  处理中🙂  "),
                1 => Value::Null,
                2 => json!({"unexpected": true}),
                _ => json!(123),
            };
            workflow_run::ActiveModel {
                business_key: Set(format!("pipeline-perf:{}", project.id)),
                workflow_kind: Set("pipeline-perf".into()),
                project_id: Set(Some(project.id)),
                root_project_id: Set(None),
                source_snapshot_id: Set(None),
                scope_table: Set(None),
                scope_id: Set(None),
                chapter_id: Set(None),
                mode: Set(None),
                input_fingerprint_json: Set(None),
                status: Set(status.into()),
                current_step_key: Set(None),
                progress_json: Set(Some(json!({"message": message, "snapshot": snapshot}))),
                active_task_id: Set(None),
                can_resume: Set(false),
                resume_block_reason: Set(None),
                error_message: Set((index == 4).then(|| "  优先错误  ".into())),
                created_at: Set(now),
                updated_at: Set(now),
                completed_at: Set(None),
                ..Default::default()
            }
            .insert(&db)
            .await
            .unwrap();
        }
    }
    (db, ids)
}

#[tokio::test]
async fn pipeline_query_preserves_latest_scope_and_message_semantics() {
    let (db, ids) = fixture(6, 2).await;
    let states = latest_pipeline_states_by_project(&db, &ids[..5], "pipeline-perf", "test")
        .await
        .unwrap();
    assert_eq!(states.len(), 5);
    for id in &ids[..5] {
        assert!(states[id].running);
        assert!(
            !states[id].failed,
            "same timestamp must select the newest run ID"
        );
    }
    assert_eq!(states[&ids[0]].message.as_deref(), Some("处理中🙂"));
    for id in &ids[1..4] {
        assert_eq!(states[id].message, None, "non-string JSON is not a message");
    }
    assert_eq!(states[&ids[4]].message.as_deref(), Some("优先错误"));
    assert!(!states.contains_key(&ids[5]));
    assert!(
        latest_pipeline_states_by_project(&db, &ids, "other-kind", "test")
            .await
            .unwrap()
            .is_empty()
    );
    assert!(
        latest_pipeline_states_by_project(
            &DatabaseConnection::Disconnected,
            &[],
            "pipeline-perf",
            "test"
        )
        .await
        .unwrap()
        .is_empty()
    );
    let run = workflow_run::Entity::find()
        .one(&db)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        run.progress_json.unwrap()["snapshot"]["text"],
        "已冻结的镜头正文。".repeat(2)
    );
}

#[tokio::test]
#[ignore = "manual PostgreSQL benchmark; needs isolated A4N_TEST_DATABASE_URL"]
async fn performance_project_pipeline_query() {
    let (db, ids) = fixture(12, 20_000).await;
    let mut samples = Vec::new();
    for sample in 0..12 {
        let start = std::time::Instant::now();
        let states = latest_pipeline_states_by_project(&db, &ids, "pipeline-perf", "benchmark")
            .await
            .unwrap();
        let elapsed = start.elapsed().as_secs_f64() * 1000.0;
        assert_eq!(states.len(), 12);
        assert!(states.values().all(|state| state.running && !state.failed));
        if sample >= 3 {
            samples.push(elapsed);
        }
    }
    samples.sort_by(f64::total_cmp);
    println!(
        "PERF pipeline_query projects=12 snapshot_bytes_per_run={} median_ms={:.3} samples_ms={samples:?}",
        "已冻结的镜头正文。".len() * 20_000,
        samples[4]
    );
}
