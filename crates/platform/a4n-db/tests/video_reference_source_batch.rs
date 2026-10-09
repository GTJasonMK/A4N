use a4n_db::project::{
    CreateDerivedProjectParams, CreateProjectRecordInput, DerivedChapterScope,
    create_derived_project_record, create_project_record, resolve_snapshot_sources,
};
use a4n_db_models::models::{chapter::chapter, project::source_snapshot_chapter};
use chrono::Utc;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, QueryOrder, QuerySelect, Set};
use std::time::Instant;

const CHAPTER_COUNT: i32 = 1_000;
const SAMPLES: usize = 15;

fn percentile(samples: &[f64], percentile: f64) -> f64 {
    let mut sorted = samples.to_vec();
    sorted.sort_by(f64::total_cmp);
    let index = ((sorted.len() - 1) as f64 * percentile).round() as usize;
    sorted[index]
}

async fn resolve_one_by_one(
    db: &impl a4n_db::DbExecutor,
    project_id: i64,
    root_project_id: i64,
    snapshot_id: i64,
    source_chapter_ids: &[i64],
) -> Vec<a4n_domain::chapter::ResolvedChapterSource> {
    let mut sources = Vec::with_capacity(source_chapter_ids.len());
    for source_chapter_id in source_chapter_ids {
        sources.extend(
            resolve_snapshot_sources(
                db,
                project_id,
                root_project_id,
                snapshot_id,
                &[*source_chapter_id],
            )
            .await
            .expect("single source query must succeed"),
        );
    }
    sources
}

#[tokio::test]
#[ignore = "manual PostgreSQL benchmark; needs isolated A4N_TEST_DATABASE_URL"]
async fn performance_video_reference_source_resolution_batch_1000_chapters() {
    let db = a4n_migration::test_support::connect_test_database().await;
    let root = create_project_record(
        &db,
        CreateProjectRecordInput {
            user_id: 1,
            title: format!("视频参考源解析性能 {}", uuid::Uuid::new_v4()),
            description: String::new(),
            project_type: "novel_long".to_string(),
        },
    )
    .await
    .expect("test project must be created");
    let now = Utc::now().naive_utc();
    let chapters = (1..=CHAPTER_COUNT)
        .map(|chapter_number| {
            let content = format!("源章节正文 {chapter_number} {}", "内容".repeat(256));
            chapter::ActiveModel {
                project_id: Set(root.id),
                chapter_number: Set(chapter_number),
                title: Set(Some(format!("源章节 {chapter_number}"))),
                word_count: Set(content.chars().count() as i32),
                content: Set(content),
                status: Set(chapter::STATUS_COMPLETED.to_string()),
                created_at: Set(now),
                updated_at: Set(now),
                ..Default::default()
            }
        })
        .collect::<Vec<_>>();
    chapter::Entity::insert_many(chapters)
        .exec(&db)
        .await
        .expect("synthetic chapters must be inserted");
    let video = create_derived_project_record(
        &db,
        1,
        CreateDerivedProjectParams {
            root_project_id: root.id,
            derived_kind: "video".to_string(),
            title: "性能测试视频项目".to_string(),
            description: String::new(),
            chapter_scope: DerivedChapterScope {
                start_chapter: 1,
                end_chapter: CHAPTER_COUNT,
            },
        },
    )
    .await
    .expect("video project must be created");
    let snapshot_id = video.source_snapshot_id.expect("snapshot must exist");
    let source_chapter_ids = source_snapshot_chapter::Entity::find()
        .select_only()
        .column(source_snapshot_chapter::Column::SourceChapterId)
        .filter(source_snapshot_chapter::Column::SnapshotId.eq(snapshot_id))
        .order_by_asc(source_snapshot_chapter::Column::ChapterNumber)
        .into_tuple::<i64>()
        .all(&db)
        .await
        .expect("snapshot chapters must be loaded");
    assert_eq!(source_chapter_ids.len(), CHAPTER_COUNT as usize);

    for _ in 0..2 {
        let _ = resolve_one_by_one(&db, video.id, root.id, snapshot_id, &source_chapter_ids).await;
        let batch =
            resolve_snapshot_sources(&db, video.id, root.id, snapshot_id, &source_chapter_ids)
                .await
                .expect("batch source query must succeed");
        assert_eq!(batch.len(), source_chapter_ids.len());
    }

    let mut single_samples = Vec::with_capacity(SAMPLES);
    let mut batch_samples = Vec::with_capacity(SAMPLES);
    for sample in 0..SAMPLES {
        if sample % 2 == 0 {
            let started = Instant::now();
            let single =
                resolve_one_by_one(&db, video.id, root.id, snapshot_id, &source_chapter_ids).await;
            single_samples.push(started.elapsed().as_secs_f64() * 1000.0);

            let started = Instant::now();
            let batch =
                resolve_snapshot_sources(&db, video.id, root.id, snapshot_id, &source_chapter_ids)
                    .await
                    .expect("batch source query must succeed");
            batch_samples.push(started.elapsed().as_secs_f64() * 1000.0);
            assert_eq!(single, batch);
        } else {
            let started = Instant::now();
            let batch =
                resolve_snapshot_sources(&db, video.id, root.id, snapshot_id, &source_chapter_ids)
                    .await
                    .expect("batch source query must succeed");
            batch_samples.push(started.elapsed().as_secs_f64() * 1000.0);

            let started = Instant::now();
            let single =
                resolve_one_by_one(&db, video.id, root.id, snapshot_id, &source_chapter_ids).await;
            single_samples.push(started.elapsed().as_secs_f64() * 1000.0);
            assert_eq!(single, batch);
        }
    }

    let single_p50 = percentile(&single_samples, 0.50);
    let batch_p50 = percentile(&batch_samples, 0.50);
    let single_p95 = percentile(&single_samples, 0.95);
    let batch_p95 = percentile(&batch_samples, 0.95);
    println!(
        "PERF-025 video source resolution: single p50={single_p50:.3}ms p95={single_p95:.3}ms; batch p50={batch_p50:.3}ms p95={batch_p95:.3}ms; p50_reduction={:.2}% p95_reduction={:.2}% records={}",
        (1.0 - batch_p50 / single_p50) * 100.0,
        (1.0 - batch_p95 / single_p95) * 100.0,
        source_chapter_ids.len()
    );
    assert!(
        (1.0 - batch_p50 / single_p50) >= 0.30,
        "batch p50 must be at least 30% faster"
    );
}
