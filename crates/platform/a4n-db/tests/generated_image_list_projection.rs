use a4n_db::video::generated_images_for_project;
use a4n_db_models::models::ai::generated_image;
use chrono::Utc;
use sea_orm::sea_query::Expr;
use sea_orm::{
    ColumnTrait, ConnectionTrait, EntityTrait, FromQueryResult, QueryFilter, QueryOrder,
    QuerySelect, Set,
};
use serde::Serialize;
use serde_json::{Value, json};
use std::time::Instant;

const PROJECT_ID: i64 = 992;
const IMAGE_COUNT: usize = 1_000;
const METADATA_BYTES: usize = 8 * 1024;
const WARMUPS: usize = 2;
const SAMPLES: usize = 15;

#[derive(Debug, FromQueryResult, Serialize, PartialEq)]
struct ImageListIndex {
    id: i64,
    project_id: i64,
    image_type: String,
    file_path: String,
    prompt: String,
    model: String,
    width: i32,
    height: i32,
    file_size: Option<i64>,
    ai_score: Option<f32>,
    is_favorite: bool,
    chapter_id: Option<i64>,
    panel_id: Option<String>,
    source_content_sha256: Option<String>,
    page_number: Option<i32>,
    created_at: chrono::NaiveDateTime,
}

#[derive(Debug, Serialize)]
struct FullImagePayload<'a> {
    id: i64,
    project_id: i64,
    image_type: &'a str,
    file_path: &'a str,
    prompt: &'a str,
    model: &'a str,
    width: i32,
    height: i32,
    file_size: Option<i64>,
    ai_score: Option<f32>,
    is_favorite: bool,
    chapter_id: Option<i64>,
    panel_id: Option<&'a str>,
    metadata_json: &'a Option<Value>,
    created_at: chrono::NaiveDateTime,
}

async fn fixture(db: &sea_orm::DatabaseConnection) {
    let now = Utc::now().naive_utc();
    let metadata_payload = "m".repeat(METADATA_BYTES);
    let rows = (0..IMAGE_COUNT)
        .map(|index| generated_image::ActiveModel {
            project_id: Set(PROJECT_ID),
            image_type: Set(if index % 2 == 0 {
                "scene"
            } else {
                "manga_page"
            }
            .into()),
            file_path: Set(format!("images/perf-{index:04}.png")),
            prompt: Set(format!("image prompt {index}")),
            model: Set("perf-model".into()),
            width: Set(1024),
            height: Set(1024),
            file_size: Set(Some(1024 * 1024)),
            ai_score: Set((index % 10 == 0).then_some(0.8)),
            is_favorite: Set(index % 7 == 0),
            chapter_id: Set(Some((index % 100 + 1) as i64)),
            panel_id: Set(Some(format!("page-{}", index % 100 + 1))),
            metadata_json: Set(Some(json!({
                "page_number": index % 100 + 1,
                "source": {"content_sha256": format!("hash-{index}")},
                "request_snapshot": metadata_payload,
            }))),
            created_at: Set(now + chrono::Duration::seconds(index as i64)),
            ..Default::default()
        })
        .collect::<Vec<_>>();
    for chunk in rows.chunks(250) {
        generated_image::Entity::insert_many(chunk.iter().cloned())
            .exec(db)
            .await
            .expect("generated image fixture must be inserted");
    }
    db.execute_unprepared("ANALYZE generated_image")
        .await
        .expect("generated image statistics must be refreshed");
}

async fn index_images(db: &sea_orm::DatabaseConnection) -> Vec<ImageListIndex> {
    generated_image::Entity::find()
        .select_only()
        .column(generated_image::Column::Id)
        .column(generated_image::Column::ProjectId)
        .column(generated_image::Column::ImageType)
        .column(generated_image::Column::FilePath)
        .column(generated_image::Column::Prompt)
        .column(generated_image::Column::Model)
        .column(generated_image::Column::Width)
        .column(generated_image::Column::Height)
        .column(generated_image::Column::FileSize)
        .column(generated_image::Column::AiScore)
        .column(generated_image::Column::IsFavorite)
        .column(generated_image::Column::ChapterId)
        .column(generated_image::Column::PanelId)
        .column_as(
            Expr::cust("metadata_json->'source'->>'content_sha256'"),
            "source_content_sha256",
        )
        .column_as(
            Expr::cust("(metadata_json->>'page_number')::integer"),
            "page_number",
        )
        .column(generated_image::Column::CreatedAt)
        .filter(generated_image::Column::ProjectId.eq(PROJECT_ID))
        .order_by_desc(generated_image::Column::CreatedAt)
        .into_model::<ImageListIndex>()
        .all(db)
        .await
        .expect("generated image index must load")
}

fn percentile(samples: &[f64], percentile: f64) -> f64 {
    let mut sorted = samples.to_vec();
    sorted.sort_by(f64::total_cmp);
    sorted[((sorted.len() - 1) as f64 * percentile).round() as usize]
}

fn serialized_bytes<T: Serialize>(value: &T) -> usize {
    serde_json::to_vec(value)
        .expect("image projection must serialize")
        .len()
}

#[tokio::test]
#[ignore = "manual PostgreSQL performance benchmark; requires isolated A4N_TEST_DATABASE_URL"]
async fn benchmark_generated_image_list_projection() {
    let db = a4n_migration::test_support::connect_test_database().await;
    fixture(&db).await;
    let full = generated_images_for_project(&db, PROJECT_ID)
        .await
        .expect("full image list must load");
    let index = index_images(&db).await;
    assert_eq!(full.len(), IMAGE_COUNT);
    assert_eq!(index.len(), IMAGE_COUNT);
    for (image, item) in full.iter().zip(&index) {
        assert_eq!(image.id, item.id);
        assert_eq!(image.project_id, item.project_id);
        assert_eq!(image.image_type, item.image_type);
        assert_eq!(image.file_path, item.file_path);
        assert_eq!(image.prompt, item.prompt);
        assert_eq!(image.model, item.model);
        assert_eq!(image.width, item.width);
        assert_eq!(image.height, item.height);
        assert_eq!(image.file_size, item.file_size);
        assert_eq!(image.ai_score, item.ai_score);
        assert_eq!(image.is_favorite, item.is_favorite);
        assert_eq!(image.chapter_id, item.chapter_id);
        assert_eq!(image.panel_id, item.panel_id);
        assert_eq!(
            image
                .metadata_json
                .as_ref()
                .and_then(|value| value.get("source"))
                .and_then(|value| value.get("content_sha256"))
                .and_then(Value::as_str),
            item.source_content_sha256.as_deref(),
        );
        assert_eq!(
            image
                .metadata_json
                .as_ref()
                .and_then(|value| value.get("page_number"))
                .and_then(Value::as_i64)
                .map(|value| value as i32),
            item.page_number,
        );
        assert_eq!(image.created_at, item.created_at);
    }
    let full_payload = full
        .iter()
        .map(|image| FullImagePayload {
            id: image.id,
            project_id: image.project_id,
            image_type: &image.image_type,
            file_path: &image.file_path,
            prompt: &image.prompt,
            model: &image.model,
            width: image.width,
            height: image.height,
            file_size: image.file_size,
            ai_score: image.ai_score,
            is_favorite: image.is_favorite,
            chapter_id: image.chapter_id,
            panel_id: image.panel_id.as_deref(),
            metadata_json: &image.metadata_json,
            created_at: image.created_at,
        })
        .collect::<Vec<_>>();
    let full_bytes = serialized_bytes(&full_payload);
    let index_bytes = serialized_bytes(&index);

    for _ in 0..WARMUPS {
        let _ = generated_images_for_project(&db, PROJECT_ID)
            .await
            .expect("full warmup must load");
        let _ = index_images(&db).await;
    }
    let mut full_samples = Vec::with_capacity(SAMPLES);
    let mut index_samples = Vec::with_capacity(SAMPLES);
    for sample in 0..SAMPLES {
        if sample % 2 == 0 {
            let started = Instant::now();
            let full = generated_images_for_project(&db, PROJECT_ID)
                .await
                .expect("full sample must load");
            full_samples.push(started.elapsed().as_secs_f64() * 1_000.0);
            assert_eq!(full.len(), IMAGE_COUNT);

            let started = Instant::now();
            let index = index_images(&db).await;
            index_samples.push(started.elapsed().as_secs_f64() * 1_000.0);
            assert_eq!(index.len(), IMAGE_COUNT);
        } else {
            let started = Instant::now();
            let index = index_images(&db).await;
            index_samples.push(started.elapsed().as_secs_f64() * 1_000.0);
            assert_eq!(index.len(), IMAGE_COUNT);

            let started = Instant::now();
            let full = generated_images_for_project(&db, PROJECT_ID)
                .await
                .expect("full sample must load");
            full_samples.push(started.elapsed().as_secs_f64() * 1_000.0);
            assert_eq!(full.len(), IMAGE_COUNT);
        }
    }
    let full_p50 = percentile(&full_samples, 0.50);
    let full_p95 = percentile(&full_samples, 0.95);
    let index_p50 = percentile(&index_samples, 0.50);
    let index_p95 = percentile(&index_samples, 0.95);
    println!(
        "{}",
        json!({
            "images": IMAGE_COUNT,
            "metadata_bytes_per_image": METADATA_BYTES,
            "full_response_bytes": full_bytes,
            "index_response_bytes": index_bytes,
            "full_p50_p95_ms": [full_p50, full_p95],
            "index_p50_p95_ms": [index_p50, index_p95],
            "improvement_percent": [(1.0 - index_p50 / full_p50) * 100.0, (1.0 - index_p95 / full_p95) * 100.0],
        })
    );
}
