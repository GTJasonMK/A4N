use a4n_db::project::dependency_graph::asset_dependency_graph;
use a4n_db::project::{CreateProjectRecordInput, create_project_record};
use a4n_db_models::models::ai::generated_image;
use a4n_domain::image::{
    IMAGE_TYPE_OBJECT_MATERIAL, IMAGE_TYPE_SCENE_MATERIAL, IMAGE_TYPE_VIDEO_KEY_FRAME,
    IMAGE_TYPE_VIDEO_START_FRAME, IMAGE_TYPE_VIDEO_STORYBOARD_FRAME,
};
use chrono::Utc;
use sea_orm::{
    ColumnTrait, EntityTrait, FromQueryResult, QueryFilter, QueryOrder, QuerySelect, Set,
};
use serde_json::{Value, json};
use std::time::Instant;

const IMAGE_COUNT: usize = 12_000;
const RELEVANT_IMAGE_COUNT: usize = IMAGE_COUNT / 20;
const SAMPLES: usize = 9;
const GRAPH_IMAGE_TYPES: [&str; 6] = [
    "manga_page",
    IMAGE_TYPE_OBJECT_MATERIAL,
    IMAGE_TYPE_SCENE_MATERIAL,
    IMAGE_TYPE_VIDEO_KEY_FRAME,
    IMAGE_TYPE_VIDEO_START_FRAME,
    IMAGE_TYPE_VIDEO_STORYBOARD_FRAME,
];

#[derive(Debug, Clone, PartialEq, Eq, FromQueryResult)]
struct GraphImageRow {
    image_type: String,
    metadata_json: Option<Value>,
}

fn percentile(samples: &[f64], percentile: f64) -> f64 {
    let mut sorted = samples.to_vec();
    sorted.sort_by(f64::total_cmp);
    sorted[((sorted.len() - 1) as f64 * percentile).round() as usize]
}

async fn fixture(db: &sea_orm::DatabaseConnection) -> i64 {
    let suffix = uuid::Uuid::new_v4().simple().to_string();
    let project = create_project_record(
        db,
        CreateProjectRecordInput {
            user_id: 1,
            title: format!("dependency graph {suffix}"),
            description: String::new(),
            project_type: "novel_long".into(),
        },
    )
    .await
    .expect("benchmark project must be created");

    let now = Utc::now().naive_utc();
    let unrelated_payload = "unrelated graph metadata ".repeat(150);
    let mut rows = Vec::with_capacity(500);
    for index in 0..IMAGE_COUNT {
        let relevant = index < RELEVANT_IMAGE_COUNT;
        let image_type = if relevant {
            match index % GRAPH_IMAGE_TYPES.len() {
                0 => "manga_page",
                1 => IMAGE_TYPE_OBJECT_MATERIAL,
                2 => IMAGE_TYPE_SCENE_MATERIAL,
                3 => IMAGE_TYPE_VIDEO_KEY_FRAME,
                4 => IMAGE_TYPE_VIDEO_START_FRAME,
                _ => IMAGE_TYPE_VIDEO_STORYBOARD_FRAME,
            }
        } else {
            "canvas_image"
        };
        let metadata_json = if relevant {
            match image_type {
                IMAGE_TYPE_OBJECT_MATERIAL => Some(json!({
                    "profile_kind": "object",
                    "profile_id": (index % 20 + 1) as i64,
                })),
                IMAGE_TYPE_SCENE_MATERIAL => Some(json!({
                    "profile_kind": "scene",
                    "profile_id": (index % 20 + 1) as i64,
                })),
                IMAGE_TYPE_VIDEO_KEY_FRAME
                | IMAGE_TYPE_VIDEO_START_FRAME
                | IMAGE_TYPE_VIDEO_STORYBOARD_FRAME => Some(json!({
                    "generation_context": {
                        "source": "video_first_frame_prompt",
                        "prompt_id": (index + 1) as i64,
                    },
                })),
                _ => Some(json!({"page_number": index + 1})),
            }
        } else {
            Some(json!({"payload": unrelated_payload}))
        };
        rows.push(generated_image::ActiveModel {
            project_id: Set(project.id),
            image_type: Set(image_type.to_string()),
            file_path: Set(format!("benchmark/{index}.png")),
            prompt: Set("benchmark".into()),
            model: Set("test".into()),
            width: Set(1),
            height: Set(1),
            file_size: Set(Some(1)),
            ai_score: Set(None),
            is_favorite: Set(false),
            chapter_id: Set(None),
            panel_id: Set(None),
            metadata_json: Set(metadata_json),
            created_at: Set(now),
            ..Default::default()
        });
        if rows.len() == 500 {
            generated_image::Entity::insert_many(std::mem::take(&mut rows))
                .exec(db)
                .await
                .expect("benchmark images must be inserted");
        }
    }
    if !rows.is_empty() {
        generated_image::Entity::insert_many(rows)
            .exec(db)
            .await
            .expect("benchmark images must be inserted");
    }
    project.id
}

async fn full_image_projection(
    db: &sea_orm::DatabaseConnection,
    project_id: i64,
) -> Vec<GraphImageRow> {
    generated_image::Entity::find()
        .select_only()
        .column(generated_image::Column::ImageType)
        .column(generated_image::Column::MetadataJson)
        .filter(generated_image::Column::ProjectId.eq(project_id))
        .order_by_asc(generated_image::Column::Id)
        .into_model::<GraphImageRow>()
        .all(db)
        .await
        .expect("full graph image projection must succeed")
}

async fn filtered_image_projection(
    db: &sea_orm::DatabaseConnection,
    project_id: i64,
) -> Vec<GraphImageRow> {
    generated_image::Entity::find()
        .select_only()
        .column(generated_image::Column::ImageType)
        .column(generated_image::Column::MetadataJson)
        .filter(generated_image::Column::ProjectId.eq(project_id))
        .filter(generated_image::Column::ImageType.is_in(GRAPH_IMAGE_TYPES))
        .order_by_asc(generated_image::Column::Id)
        .into_model::<GraphImageRow>()
        .all(db)
        .await
        .expect("filtered graph image projection must succeed")
}

fn metadata_bytes(rows: &[GraphImageRow]) -> usize {
    rows.iter()
        .filter_map(|row| row.metadata_json.as_ref())
        .map(|metadata| {
            serde_json::to_vec(metadata)
                .expect("metadata must serialize")
                .len()
        })
        .sum()
}

#[tokio::test]
#[ignore = "manual PostgreSQL performance benchmark; requires isolated A4N_TEST_DATABASE_URL"]
async fn benchmark_dependency_graph_image_projection() {
    let db = a4n_migration::test_support::connect_test_database().await;
    let project_id = fixture(&db).await;

    let full_rows = full_image_projection(&db, project_id).await;
    let filtered_rows = filtered_image_projection(&db, project_id).await;
    let expected = full_rows
        .iter()
        .filter(|row| GRAPH_IMAGE_TYPES.contains(&row.image_type.as_str()))
        .cloned()
        .collect::<Vec<_>>();
    assert_eq!(filtered_rows, expected);

    let graph = asset_dependency_graph(&db, project_id)
        .await
        .expect("dependency graph must build with filtered image projection");
    assert_eq!(
        graph
            .nodes
            .iter()
            .find(|node| node.id == "manga_images")
            .map(|node| node.count),
        Some((RELEVANT_IMAGE_COUNT / GRAPH_IMAGE_TYPES.len()) as u64),
    );

    let mut full_samples = Vec::with_capacity(SAMPLES);
    let mut filtered_samples = Vec::with_capacity(SAMPLES);
    for sample in 0..SAMPLES {
        if sample % 2 == 0 {
            let started = Instant::now();
            let full = full_image_projection(&db, project_id).await;
            full_samples.push(started.elapsed().as_secs_f64() * 1000.0);
            assert_eq!(full.len(), IMAGE_COUNT);

            let started = Instant::now();
            let filtered = filtered_image_projection(&db, project_id).await;
            filtered_samples.push(started.elapsed().as_secs_f64() * 1000.0);
            assert_eq!(filtered, expected);
        } else {
            let started = Instant::now();
            let filtered = filtered_image_projection(&db, project_id).await;
            filtered_samples.push(started.elapsed().as_secs_f64() * 1000.0);
            assert_eq!(filtered, expected);

            let started = Instant::now();
            let full = full_image_projection(&db, project_id).await;
            full_samples.push(started.elapsed().as_secs_f64() * 1000.0);
            assert_eq!(full.len(), IMAGE_COUNT);
        }
    }

    let full_p50 = percentile(&full_samples, 0.50);
    let full_p95 = percentile(&full_samples, 0.95);
    let filtered_p50 = percentile(&filtered_samples, 0.50);
    let filtered_p95 = percentile(&filtered_samples, 0.95);
    eprintln!(
        "dependency_graph_image_projection images={IMAGE_COUNT} relevant={RELEVANT_IMAGE_COUNT} full_rows={} filtered_rows={} full_metadata_bytes={} filtered_metadata_bytes={} full_p50_p95_ms={full_p50:.3}/{full_p95:.3} filtered_p50_p95_ms={filtered_p50:.3}/{filtered_p95:.3} improvement={:.1}%/{:.1}%",
        full_rows.len(),
        filtered_rows.len(),
        metadata_bytes(&full_rows),
        metadata_bytes(&filtered_rows),
        (1.0 - filtered_p50 / full_p50) * 100.0,
        (1.0 - filtered_p95 / full_p95) * 100.0,
    );
}
