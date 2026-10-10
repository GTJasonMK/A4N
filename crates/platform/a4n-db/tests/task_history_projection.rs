use a4n_db::workflow::{persisted_task_by_id, persisted_task_history};
use a4n_db_models::models::task::task_queue;
use chrono::Utc;
use sea_orm::{EntityTrait, QueryOrder, QuerySelect, Set};
use serde_json::json;
use std::time::Instant;

const TASK_COUNT: usize = 2_000;
const PAGE_SIZE: u64 = 200;
const SAMPLES: usize = 9;

fn percentile(samples: &[f64], percentile: f64) -> f64 {
    let mut sorted = samples.to_vec();
    sorted.sort_by(f64::total_cmp);
    sorted[((sorted.len() - 1) as f64 * percentile).round() as usize]
}

async fn fixture(db: &sea_orm::DatabaseConnection) -> String {
    let now = Utc::now().naive_utc();
    let payload = "task payload ".repeat(2_500);
    let mut rows = Vec::with_capacity(250);
    for index in 0..TASK_COUNT {
        rows.push(task_queue::ActiveModel {
            id: Set(format!("task-history-{index:05}")),
            task_type: Set("llm_call".into()),
            priority: Set("medium".into()),
            status: Set(if index % 3 == 0 {
                "completed".into()
            } else {
                "failed".into()
            }),
            project_id: Set(None),
            root_project_id: Set(None),
            source_snapshot_id: Set(None),
            task_scope: Set(Some("workflow".into())),
            workflow_run_id: Set(Some(index as i64 + 1)),
            payload: Set(Some(json!({"prompt": payload, "index": index}))),
            progress: Set(Some(json!({
                "current": 1,
                "total": 1,
                "message": "完成",
            }))),
            error_message: Set((index % 3 != 0).then(|| "模型调用失败".into())),
            dag_id: Set(None),
            created_at: Set(now + chrono::Duration::seconds(index as i64)),
            started_at: Set(Some(now)),
            completed_at: Set(Some(now)),
        });
        if rows.len() == 250 {
            task_queue::Entity::insert_many(std::mem::take(&mut rows))
                .exec(db)
                .await
                .expect("task history fixture rows must be inserted");
        }
    }
    if !rows.is_empty() {
        task_queue::Entity::insert_many(rows)
            .exec(db)
            .await
            .expect("task history fixture rows must be inserted");
    }
    format!("task-history-{:05}", TASK_COUNT - 1)
}

async fn legacy_page(db: &sea_orm::DatabaseConnection) -> Vec<task_queue::Model> {
    task_queue::Entity::find()
        .order_by_desc(task_queue::Column::CreatedAt)
        .limit(PAGE_SIZE)
        .all(db)
        .await
        .expect("legacy task history query must succeed")
}

fn payload_bytes(rows: &[task_queue::Model]) -> usize {
    rows.iter()
        .filter_map(|row| row.payload.as_ref())
        .map(|payload| {
            serde_json::to_vec(payload)
                .expect("payload must serialize")
                .len()
        })
        .sum()
}

#[tokio::test]
#[ignore = "manual PostgreSQL performance benchmark; requires isolated A4N_TEST_DATABASE_URL"]
async fn benchmark_task_history_projection() {
    let db = a4n_migration::test_support::connect_test_database().await;
    let newest_task_id = fixture(&db).await;

    let legacy = legacy_page(&db).await;
    let current = persisted_task_history(&db, None, 0, PAGE_SIZE)
        .await
        .expect("projected task history query must succeed");
    assert_eq!(legacy.len(), PAGE_SIZE as usize);
    assert_eq!(current.len(), PAGE_SIZE as usize);
    assert_eq!(current[0].task.id, newest_task_id);
    let by_id = persisted_task_by_id(&db, &newest_task_id)
        .await
        .expect("projected task detail query must succeed")
        .expect("newest task must exist");
    assert_eq!(
        serde_json::to_value(&by_id).expect("projected task detail must serialize"),
        serde_json::to_value(&current[0]).expect("projected task history row must serialize"),
    );
    for (legacy, current) in legacy.iter().zip(&current) {
        assert_eq!(legacy.id, current.task.id);
        assert_eq!(legacy.task_type, current.task.task_type.as_str());
        assert_eq!(legacy.priority, current.task.priority.as_str());
        assert_eq!(legacy.status, current.task.status.as_str());
        match (&current.task.status, legacy.error_message.as_deref()) {
            (a4n_queue::priority::TaskStatus::Failed { error }, Some(expected)) => {
                assert_eq!(error, expected);
            }
            (a4n_queue::priority::TaskStatus::Interrupted { reason }, expected) => {
                assert_eq!(reason.as_deref(), expected);
            }
            (a4n_queue::priority::TaskStatus::Failed { .. }, None) => {
                panic!("failed task must preserve its error message");
            }
            _ => {}
        }
        assert_eq!(legacy.workflow_run_id, current.workflow_run_id);
        assert_eq!(legacy.project_id, current.task.project_id);
        assert_eq!(legacy.root_project_id, current.task.root_project_id);
        assert_eq!(legacy.source_snapshot_id, current.task.source_snapshot_id);
        assert_eq!(legacy.task_scope, current.task.task_scope);
        assert_eq!(
            legacy.progress.as_ref(),
            current
                .task
                .progress
                .as_ref()
                .map(|progress| serde_json::to_value(progress).expect("progress must serialize"))
                .as_ref(),
        );
        assert_eq!(legacy.created_at, current.task.created_at);
        assert_eq!(legacy.started_at, current.task.started_at);
        assert_eq!(legacy.completed_at, current.task.completed_at);
    }

    let mut legacy_samples = Vec::with_capacity(SAMPLES);
    let mut current_samples = Vec::with_capacity(SAMPLES);
    for sample in 0..SAMPLES {
        if sample % 2 == 0 {
            let started = Instant::now();
            let legacy = legacy_page(&db).await;
            legacy_samples.push(started.elapsed().as_secs_f64() * 1000.0);
            assert_eq!(legacy.len(), PAGE_SIZE as usize);

            let started = Instant::now();
            let current = persisted_task_history(&db, None, 0, PAGE_SIZE)
                .await
                .expect("projected task history query must succeed");
            current_samples.push(started.elapsed().as_secs_f64() * 1000.0);
            assert_eq!(current.len(), PAGE_SIZE as usize);
        } else {
            let started = Instant::now();
            let current = persisted_task_history(&db, None, 0, PAGE_SIZE)
                .await
                .expect("projected task history query must succeed");
            current_samples.push(started.elapsed().as_secs_f64() * 1000.0);
            assert_eq!(current.len(), PAGE_SIZE as usize);

            let started = Instant::now();
            let legacy = legacy_page(&db).await;
            legacy_samples.push(started.elapsed().as_secs_f64() * 1000.0);
            assert_eq!(legacy.len(), PAGE_SIZE as usize);
        }
    }

    let legacy_p50 = percentile(&legacy_samples, 0.50);
    let legacy_p95 = percentile(&legacy_samples, 0.95);
    let current_p50 = percentile(&current_samples, 0.50);
    let current_p95 = percentile(&current_samples, 0.95);
    let projected_summary_bytes = current
        .iter()
        .map(|task| {
            serde_json::to_vec(task)
                .expect("projected task must serialize")
                .len()
        })
        .sum::<usize>();
    eprintln!(
        "task_history_projection tasks={TASK_COUNT} page_size={PAGE_SIZE} legacy_rows={} projected_rows={} legacy_payload_bytes={} projected_summary_bytes={projected_summary_bytes} legacy_p50_p95_ms={legacy_p50:.3}/{legacy_p95:.3} projected_p50_p95_ms={current_p50:.3}/{current_p95:.3} improvement={:.1}%/{:.1}%",
        legacy.len(),
        current.len(),
        payload_bytes(&legacy),
        (1.0 - current_p50 / legacy_p50) * 100.0,
        (1.0 - current_p95 / legacy_p95) * 100.0,
    );
}
