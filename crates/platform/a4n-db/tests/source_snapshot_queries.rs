use a4n_db::{
    project::source_snapshot_chapter_content_by_snapshot_and_source,
    project::{
        CreateDerivedProjectParams, CreateProjectRecordInput, DerivedChapterScope,
        create_derived_project_record, create_project_record, source_snapshot_chapter_for_project,
        source_snapshot_chapter_index, source_snapshot_chapters,
    },
};
use a4n_db_models::models::chapter::chapter;
use chrono::Utc;
use sea_orm::{ActiveModelTrait, ConnectionTrait, EntityTrait, Set};
use serde_json::json;
use std::time::Instant;

#[tokio::test]
async fn source_snapshot_chapter_content_projection_preserves_content() {
    let db = a4n_migration::test_support::connect_test_database().await;
    let project = create_project_record(
        &db,
        CreateProjectRecordInput {
            user_id: 1,
            title: format!("源快照章节查询 {}", uuid::Uuid::new_v4()),
            description: String::new(),
            project_type: "novel_long".to_string(),
        },
    )
    .await
    .expect("test project must be created");
    let now = Utc::now().naive_utc();
    let source_chapter = chapter::ActiveModel {
        project_id: Set(project.id),
        chapter_number: Set(1),
        title: Set(Some("第一章".to_string())),
        content: Set("源章节正文".to_string()),
        word_count: Set(5),
        status: Set(chapter::STATUS_COMPLETED.to_string()),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(&db)
    .await
    .expect("source chapter must be created");
    let derived = create_derived_project_record(
        &db,
        1,
        CreateDerivedProjectParams {
            root_project_id: project.id,
            derived_kind: "comic".to_string(),
            title: "派生漫画项目".to_string(),
            description: String::new(),
            chapter_scope: DerivedChapterScope {
                start_chapter: 1,
                end_chapter: 1,
            },
        },
    )
    .await
    .expect("derived project and source snapshot must be created");
    let snapshot_id = derived.source_snapshot_id.expect("snapshot ID must exist");

    let content = source_snapshot_chapter_content_by_snapshot_and_source(
        &db,
        project.id,
        snapshot_id,
        source_chapter.id,
    )
    .await
    .expect("content projection must load");
    assert_eq!(content.as_deref(), Some("源章节正文"));
    assert!(
        source_snapshot_chapter_content_by_snapshot_and_source(
            &db,
            project.id + 1,
            snapshot_id,
            source_chapter.id
        )
        .await
        .expect("scope query must execute")
        .is_none()
    );

    let selected = source_snapshot_chapter_for_project(&db, derived.id, source_chapter.id)
        .await
        .expect("current source snapshot detail must load");
    assert_eq!(selected.snapshot_id, snapshot_id);
    let scoped = a4n_db::project::source_snapshot_chapter_for_snapshot(
        &db,
        derived.id,
        snapshot_id,
        source_chapter.id,
    )
    .await
    .expect("scoped source snapshot detail must load");
    assert_eq!(scoped, Some(selected));
    assert!(
        a4n_db::project::source_snapshot_chapter_for_snapshot(
            &db,
            derived.id,
            snapshot_id + 1,
            source_chapter.id,
        )
        .await
        .expect("stale snapshot scope must be rejected")
        .is_none()
    );
}

const SNAPSHOT_BENCHMARK_CHAPTERS: i32 = 1_000;
const SNAPSHOT_BENCHMARK_CONTENT_BYTES: usize = 9 * 1024;
const SNAPSHOT_BENCHMARK_ANALYSIS_BYTES: usize = 8 * 1024;

fn percentile(samples: &[f64], fraction: f64) -> f64 {
    let mut sorted = samples.to_vec();
    sorted.sort_by(f64::total_cmp);
    sorted[((sorted.len() - 1) as f64 * fraction).round() as usize]
}

async fn measure_full_snapshot_read(
    db: &sea_orm::DatabaseConnection,
    project_id: i64,
) -> (f64, usize) {
    let started = Instant::now();
    let chapters = source_snapshot_chapters(db, project_id)
        .await
        .expect("full source snapshot list must load");
    let response_bytes = serde_json::to_vec(&chapters)
        .expect("full source snapshot list must serialize")
        .len();
    (started.elapsed().as_secs_f64() * 1_000.0, response_bytes)
}

async fn measure_index_and_selected_chapter(
    db: &sea_orm::DatabaseConnection,
    project_id: i64,
) -> (f64, usize) {
    let started = Instant::now();
    let index = source_snapshot_chapter_index(db, project_id)
        .await
        .expect("source snapshot chapter index must load");
    let source_chapter_id = index
        .first()
        .expect("benchmark snapshot must contain chapters")
        .source_chapter_id;
    let selected = source_snapshot_chapter_for_project(db, project_id, source_chapter_id)
        .await
        .expect("selected source snapshot chapter must load");
    let response_bytes = serde_json::to_vec(&index)
        .expect("source snapshot chapter index must serialize")
        .len()
        + serde_json::to_vec(&selected)
            .expect("selected source snapshot chapter must serialize")
            .len();
    (started.elapsed().as_secs_f64() * 1_000.0, response_bytes)
}

#[tokio::test]
#[ignore = "manual PostgreSQL source snapshot performance benchmark; requires isolated A4N_TEST_DATABASE_URL"]
async fn benchmark_source_snapshot_index_and_selected_chapter() {
    let db = a4n_migration::test_support::connect_test_database().await;
    let root = create_project_record(
        &db,
        CreateProjectRecordInput {
            user_id: 1,
            title: format!("源快照索引性能基准 {}", uuid::Uuid::new_v4()),
            description: String::new(),
            project_type: "novel_long".into(),
        },
    )
    .await
    .expect("snapshot benchmark root must be created");
    let now = Utc::now().naive_utc();
    let content = "河岸调查账本线索".repeat(SNAPSHOT_BENCHMARK_CONTENT_BYTES / 24);
    assert_eq!(content.len(), SNAPSHOT_BENCHMARK_CONTENT_BYTES);
    let analysis_payload = "a".repeat(SNAPSHOT_BENCHMARK_ANALYSIS_BYTES);
    let analysis = json!({
        "metadata": {"characters": ["林舟", "沈青"], "locations": ["河岸"]},
        "summaries": {"compressed": "追寻账本的来历"},
        "key_events": [],
        "unused_analysis_payload": analysis_payload
    });
    let chapters = (1..=SNAPSHOT_BENCHMARK_CHAPTERS)
        .map(|chapter_number| chapter::ActiveModel {
            project_id: Set(root.id),
            chapter_number: Set(chapter_number),
            title: Set(Some(format!("第{chapter_number}章"))),
            summary: Set(Some(format!("第{chapter_number}章摘要"))),
            content: Set(content.clone()),
            word_count: Set(content.chars().count() as i32),
            status: Set(chapter::STATUS_COMPLETED.into()),
            analysis_data: Set(Some(analysis.clone())),
            created_at: Set(now),
            updated_at: Set(now),
            ..Default::default()
        })
        .collect::<Vec<_>>();
    for batch in chapters.chunks(250) {
        chapter::Entity::insert_many(batch.iter().cloned())
            .exec(&db)
            .await
            .expect("snapshot benchmark chapters must be inserted");
    }
    let derived = create_derived_project_record(
        &db,
        1,
        CreateDerivedProjectParams {
            root_project_id: root.id,
            derived_kind: "comic".into(),
            title: "源快照索引性能基准漫画".into(),
            description: String::new(),
            chapter_scope: DerivedChapterScope {
                start_chapter: 1,
                end_chapter: SNAPSHOT_BENCHMARK_CHAPTERS,
            },
        },
    )
    .await
    .expect("snapshot benchmark derived project must be created");
    db.execute_unprepared("ANALYZE source_snapshot_chapter")
        .await
        .expect("snapshot benchmark statistics must be refreshed");

    let full = source_snapshot_chapters(&db, derived.id)
        .await
        .expect("full snapshot response must load");
    let index = source_snapshot_chapter_index(&db, derived.id)
        .await
        .expect("snapshot index response must load");
    assert_eq!(full.len(), SNAPSHOT_BENCHMARK_CHAPTERS as usize);
    assert_eq!(index.len(), full.len());
    for (chapter, item) in full.iter().zip(&index) {
        assert_eq!(chapter.id, item.id);
        assert_eq!(chapter.snapshot_id, item.snapshot_id);
        assert_eq!(chapter.source_chapter_id, item.source_chapter_id);
        assert_eq!(chapter.chapter_number, item.chapter_number);
        assert_eq!(chapter.title, item.title);
    }
    let selected = source_snapshot_chapter_for_project(&db, derived.id, index[0].source_chapter_id)
        .await
        .expect("selected source snapshot chapter must load");
    assert_eq!(
        serde_json::to_value(&selected).unwrap(),
        serde_json::to_value(&full[0]).unwrap(),
        "the selected detail response must preserve the original chapter"
    );

    for _ in 0..2 {
        measure_full_snapshot_read(&db, derived.id).await;
        measure_index_and_selected_chapter(&db, derived.id).await;
    }
    let mut full_samples = Vec::with_capacity(15);
    let mut indexed_samples = Vec::with_capacity(15);
    let mut full_response_bytes = 0;
    let mut indexed_response_bytes = 0;
    for sample in 0..15 {
        let (full_result, indexed_result) = if sample % 2 == 0 {
            (
                measure_full_snapshot_read(&db, derived.id).await,
                measure_index_and_selected_chapter(&db, derived.id).await,
            )
        } else {
            let indexed = measure_index_and_selected_chapter(&db, derived.id).await;
            let full = measure_full_snapshot_read(&db, derived.id).await;
            (full, indexed)
        };
        if sample >= 2 {
            full_samples.push(full_result.0);
            indexed_samples.push(indexed_result.0);
        }
        full_response_bytes = full_result.1;
        indexed_response_bytes = indexed_result.1;
    }
    println!(
        "PERF source_snapshot_index_read={}",
        json!({
            "chapters": SNAPSHOT_BENCHMARK_CHAPTERS,
            "content_bytes_each": SNAPSHOT_BENCHMARK_CONTENT_BYTES,
            "analysis_payload_bytes_each": SNAPSHOT_BENCHMARK_ANALYSIS_BYTES,
            "full_response_bytes": full_response_bytes,
            "index_plus_selected_response_bytes": indexed_response_bytes,
            "full_p50_ms": percentile(&full_samples, 0.50),
            "full_p95_ms": percentile(&full_samples, 0.95),
            "index_plus_selected_p50_ms": percentile(&indexed_samples, 0.50),
            "index_plus_selected_p95_ms": percentile(&indexed_samples, 0.95),
            "p50_improvement_percent": (1.0 - percentile(&indexed_samples, 0.50) / percentile(&full_samples, 0.50)) * 100.0,
            "p95_improvement_percent": (1.0 - percentile(&indexed_samples, 0.95) / percentile(&full_samples, 0.95)) * 100.0,
            "full_samples_ms": full_samples,
            "index_plus_selected_samples_ms": indexed_samples,
        })
    );
}
