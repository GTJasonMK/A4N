use a4n_db::import_review::import_reviews_by_project;
use a4n_db_models::models::project::import_review_queue;
use chrono::Utc;
use sea_orm::{
    ColumnTrait, ConnectionTrait, EntityTrait, FromQueryResult, QueryFilter, QueryOrder,
    QuerySelect, Set,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::time::Instant;

const PROJECT_ID: i64 = 991;
const REVIEW_COUNT: usize = 100;
const ANALYSIS_BYTES: usize = 64 * 1024;
const WARMUPS: usize = 2;
const SAMPLES: usize = 15;

#[derive(Debug, Clone, Deserialize, FromQueryResult, Serialize, PartialEq)]
struct ImportReviewIndex {
    id: i64,
    project_id: i64,
    filename: String,
    review_status: String,
    created_at: chrono::NaiveDateTime,
    updated_at: chrono::NaiveDateTime,
}

#[derive(Debug, Serialize)]
struct FullReviewPayload<'a> {
    id: i64,
    project_id: i64,
    filename: &'a str,
    analysis_result: &'a serde_json::Value,
    review_status: &'a str,
    created_at: chrono::NaiveDateTime,
    updated_at: chrono::NaiveDateTime,
}

async fn fixture(db: &sea_orm::DatabaseConnection) {
    let now = Utc::now().naive_utc();
    let content = "c".repeat(ANALYSIS_BYTES);
    let rows = (0..REVIEW_COUNT)
        .map(|index| import_review_queue::ActiveModel {
            project_id: Set(PROJECT_ID),
            filename: Set(format!("chapter-{index:03}.txt")),
            analysis_result: Set(json!({
                "chapter_number": index + 1,
                "chapter_title": format!("第 {} 章", index + 1),
                "summary": content,
                "content": "正文",
            })),
            review_status: Set(if index % 3 == 0 {
                "accepted"
            } else {
                "pending"
            }
            .to_string()),
            created_at: Set(now + chrono::Duration::seconds(index as i64)),
            updated_at: Set(now + chrono::Duration::seconds(index as i64)),
            ..Default::default()
        })
        .collect::<Vec<_>>();
    import_review_queue::Entity::insert_many(rows)
        .exec(db)
        .await
        .expect("import review fixture must be inserted");
    db.execute_unprepared("ANALYZE import_review_queue")
        .await
        .expect("import review statistics must be refreshed");
}

async fn index_reviews(db: &sea_orm::DatabaseConnection) -> Vec<ImportReviewIndex> {
    import_review_queue::Entity::find()
        .select_only()
        .column(import_review_queue::Column::Id)
        .column(import_review_queue::Column::ProjectId)
        .column(import_review_queue::Column::Filename)
        .column(import_review_queue::Column::ReviewStatus)
        .column(import_review_queue::Column::CreatedAt)
        .column(import_review_queue::Column::UpdatedAt)
        .filter(import_review_queue::Column::ProjectId.eq(PROJECT_ID))
        .order_by_desc(import_review_queue::Column::CreatedAt)
        .into_model::<ImportReviewIndex>()
        .all(db)
        .await
        .expect("import review index must load")
}

fn percentile(samples: &[f64], percentile: f64) -> f64 {
    let mut sorted = samples.to_vec();
    sorted.sort_by(f64::total_cmp);
    sorted[((sorted.len() - 1) as f64 * percentile).round() as usize]
}

fn serialized_bytes<T: Serialize>(value: &T) -> usize {
    serde_json::to_vec(value)
        .expect("benchmark projection must serialize")
        .len()
}

#[tokio::test]
#[ignore = "manual PostgreSQL performance benchmark; requires isolated A4N_TEST_DATABASE_URL"]
async fn benchmark_import_review_list_projection() {
    let db = a4n_migration::test_support::connect_test_database().await;
    fixture(&db).await;
    let full = import_reviews_by_project(&db, PROJECT_ID)
        .await
        .expect("full import review list must load");
    let index = index_reviews(&db).await;
    assert_eq!(full.len(), REVIEW_COUNT);
    assert_eq!(index.len(), REVIEW_COUNT);
    for (review, item) in full.iter().zip(&index) {
        assert_eq!(review.id, item.id);
        assert_eq!(review.project_id, item.project_id);
        assert_eq!(review.filename, item.filename);
        assert_eq!(review.review_status, item.review_status);
        assert_eq!(review.created_at, item.created_at);
        assert_eq!(review.updated_at, item.updated_at);
    }
    let full_payload = full
        .iter()
        .map(|review| FullReviewPayload {
            id: review.id,
            project_id: review.project_id,
            filename: &review.filename,
            analysis_result: &review.analysis_result,
            review_status: &review.review_status,
            created_at: review.created_at,
            updated_at: review.updated_at,
        })
        .collect::<Vec<_>>();
    let full_bytes = serialized_bytes(&full_payload);
    let index_bytes = serialized_bytes(&index);

    for _ in 0..WARMUPS {
        let _ = import_reviews_by_project(&db, PROJECT_ID)
            .await
            .expect("full warmup must load");
        let _ = index_reviews(&db).await;
    }
    let mut full_samples = Vec::with_capacity(SAMPLES);
    let mut index_samples = Vec::with_capacity(SAMPLES);
    for sample in 0..SAMPLES {
        if sample % 2 == 0 {
            let started = Instant::now();
            let full = import_reviews_by_project(&db, PROJECT_ID)
                .await
                .expect("full sample must load");
            full_samples.push(started.elapsed().as_secs_f64() * 1_000.0);
            assert_eq!(full.len(), REVIEW_COUNT);

            let started = Instant::now();
            let index = index_reviews(&db).await;
            index_samples.push(started.elapsed().as_secs_f64() * 1_000.0);
            assert_eq!(index.len(), REVIEW_COUNT);
        } else {
            let started = Instant::now();
            let index = index_reviews(&db).await;
            index_samples.push(started.elapsed().as_secs_f64() * 1_000.0);
            assert_eq!(index.len(), REVIEW_COUNT);

            let started = Instant::now();
            let full = import_reviews_by_project(&db, PROJECT_ID)
                .await
                .expect("full sample must load");
            full_samples.push(started.elapsed().as_secs_f64() * 1_000.0);
            assert_eq!(full.len(), REVIEW_COUNT);
        }
    }
    let full_p50 = percentile(&full_samples, 0.50);
    let full_p95 = percentile(&full_samples, 0.95);
    let index_p50 = percentile(&index_samples, 0.50);
    let index_p95 = percentile(&index_samples, 0.95);
    println!(
        "{}",
        json!({
            "reviews": REVIEW_COUNT,
            "analysis_bytes_per_review": ANALYSIS_BYTES,
            "full_response_bytes": full_bytes,
            "index_response_bytes": index_bytes,
            "full_p50_p95_ms": [full_p50, full_p95],
            "index_p50_p95_ms": [index_p50, index_p95],
            "improvement_percent": [(1.0 - index_p50 / full_p50) * 100.0, (1.0 - index_p95 / full_p95) * 100.0],
        })
    );
}
