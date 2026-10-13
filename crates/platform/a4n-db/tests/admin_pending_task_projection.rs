use a4n_db::admin::{PendingTaskIndexSummary, TaskQueueSummary, pending_task_index, pending_tasks};
use a4n_db_models::models::task::task_queue;
use chrono::Utc;
use sea_orm::{ConnectionTrait, EntityTrait, Set};
use serde_json::json;
use std::time::Instant;

const PENDING_TASK_COUNT: usize = 1_000;
const COMPLETED_TASK_COUNT: usize = 3_000;
const PENDING_PAYLOAD_BYTES: usize = 16 * 1024;
const COMPLETED_PAYLOAD_BYTES: usize = 4 * 1024;
const WARMUPS: usize = 2;
const SAMPLES: usize = 15;

fn task_row(
    id: String,
    status: &str,
    priority: &str,
    created_at: chrono::NaiveDateTime,
    payload_bytes: usize,
) -> task_queue::ActiveModel {
    task_queue::ActiveModel {
        id: Set(id),
        task_type: Set("llm_call".into()),
        priority: Set(priority.into()),
        status: Set(status.into()),
        project_id: Set(Some(1)),
        root_project_id: Set(Some(1)),
        source_snapshot_id: Set(Some(2)),
        task_scope: Set(Some("chapter".into())),
        workflow_run_id: Set(None),
        payload: Set(Some(json!({"body": "x".repeat(payload_bytes)}))),
        progress: Set(Some(json!({"phase": "queued", "completed": 0, "total": 1}))),
        error_message: Set(None),
        dag_id: Set(Some("pending-task-benchmark".into())),
        created_at: Set(created_at),
        started_at: Set(None),
        completed_at: Set((status != "pending").then_some(created_at)),
    }
}

async fn fixture(db: &sea_orm::DatabaseConnection) {
    let now = Utc::now().naive_utc();
    let mut rows = (0..PENDING_TASK_COUNT)
        .map(|index| {
            task_row(
                format!("pending-{index:04}"),
                "pending",
                ["high", "medium", "low"][index % 3],
                now + chrono::Duration::seconds(index as i64),
                PENDING_PAYLOAD_BYTES,
            )
        })
        .collect::<Vec<_>>();
    rows.extend((0..COMPLETED_TASK_COUNT).map(|index| {
        task_row(
            format!("completed-{index:04}"),
            "completed",
            ["high", "medium", "low"][index % 3],
            now - chrono::Duration::seconds(index as i64 + 1),
            COMPLETED_PAYLOAD_BYTES,
        )
    }));
    for batch in rows.chunks(500) {
        task_queue::Entity::insert_many(batch.iter().cloned())
            .exec(db)
            .await
            .expect("pending task benchmark fixture must be inserted");
    }
    db.execute_unprepared("ANALYZE task_queue")
        .await
        .expect("task_queue statistics must be refreshed");
}

fn percentile(samples: &[f64], percentile: f64) -> f64 {
    let mut sorted = samples.to_vec();
    sorted.sort_by(f64::total_cmp);
    sorted[((sorted.len() - 1) as f64 * percentile).round() as usize]
}

async fn full_read_and_serialize(
    db: &sea_orm::DatabaseConnection,
) -> (Vec<TaskQueueSummary>, usize) {
    let tasks = pending_tasks(db)
        .await
        .expect("full pending task list must load");
    let bytes = serde_json::to_vec(&tasks)
        .expect("full task list must serialize")
        .len();
    (tasks, bytes)
}

async fn index_read_and_serialize(
    db: &sea_orm::DatabaseConnection,
) -> (Vec<PendingTaskIndexSummary>, usize) {
    let tasks = pending_task_index(db)
        .await
        .expect("narrow pending task list must load");
    let bytes = serde_json::to_vec(&tasks)
        .expect("narrow task list must serialize")
        .len();
    (tasks, bytes)
}

#[tokio::test]
#[ignore = "manual PostgreSQL performance benchmark; requires isolated A4N_TEST_DATABASE_URL"]
async fn benchmark_admin_pending_task_projection() {
    let db = a4n_migration::test_support::connect_test_database().await;
    fixture(&db).await;
    let (full, full_bytes) = full_read_and_serialize(&db).await;
    let (index, index_bytes) = index_read_and_serialize(&db).await;
    assert_eq!(full.len(), PENDING_TASK_COUNT);
    assert_eq!(index.len(), PENDING_TASK_COUNT);
    for (summary, item) in full.iter().zip(&index) {
        assert_eq!(summary.id, item.id);
        assert_eq!(summary.task_type, item.task_type);
        assert_eq!(summary.priority, item.priority);
        assert_eq!(summary.status, item.status);
        assert_eq!(summary.project_id, item.project_id);
        assert_eq!(summary.root_project_id, item.root_project_id);
        assert_eq!(summary.source_snapshot_id, item.source_snapshot_id);
        assert_eq!(summary.task_scope, item.task_scope);
        assert_eq!(summary.dag_id, item.dag_id);
        assert_eq!(
            summary.created_at.as_deref(),
            Some(item.created_at.as_str())
        );
    }

    for _ in 0..WARMUPS {
        let _ = full_read_and_serialize(&db).await;
        let _ = index_read_and_serialize(&db).await;
    }
    let mut full_samples = Vec::with_capacity(SAMPLES);
    let mut index_samples = Vec::with_capacity(SAMPLES);
    for sample in 0..SAMPLES {
        if sample % 2 == 0 {
            let started = Instant::now();
            let (tasks, bytes) = full_read_and_serialize(&db).await;
            full_samples.push(started.elapsed().as_secs_f64() * 1_000.0);
            assert_eq!(tasks.len(), PENDING_TASK_COUNT);
            assert_eq!(bytes, full_bytes);

            let started = Instant::now();
            let (tasks, bytes) = index_read_and_serialize(&db).await;
            index_samples.push(started.elapsed().as_secs_f64() * 1_000.0);
            assert_eq!(tasks.len(), PENDING_TASK_COUNT);
            assert_eq!(bytes, index_bytes);
        } else {
            let started = Instant::now();
            let (tasks, bytes) = index_read_and_serialize(&db).await;
            index_samples.push(started.elapsed().as_secs_f64() * 1_000.0);
            assert_eq!(tasks.len(), PENDING_TASK_COUNT);
            assert_eq!(bytes, index_bytes);

            let started = Instant::now();
            let (tasks, bytes) = full_read_and_serialize(&db).await;
            full_samples.push(started.elapsed().as_secs_f64() * 1_000.0);
            assert_eq!(tasks.len(), PENDING_TASK_COUNT);
            assert_eq!(bytes, full_bytes);
        }
    }
    let full_p50 = percentile(&full_samples, 0.50);
    let full_p95 = percentile(&full_samples, 0.95);
    let index_p50 = percentile(&index_samples, 0.50);
    let index_p95 = percentile(&index_samples, 0.95);
    println!(
        "{}",
        json!({
            "pending_tasks": PENDING_TASK_COUNT,
            "completed_history_rows": COMPLETED_TASK_COUNT,
            "pending_payload_bytes_each": PENDING_PAYLOAD_BYTES,
            "completed_payload_bytes_each": COMPLETED_PAYLOAD_BYTES,
            "full_response_bytes": full_bytes,
            "index_response_bytes": index_bytes,
            "full_p50_p95_ms": [full_p50, full_p95],
            "index_p50_p95_ms": [index_p50, index_p95],
            "improvement_percent": [(1.0 - index_p50 / full_p50) * 100.0, (1.0 - index_p95 / full_p95) * 100.0],
        })
    );
}
