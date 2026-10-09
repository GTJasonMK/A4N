use a4n_db::project::{
    CreateDerivedProjectParams, CreateProjectRecordInput, DerivedChapterScope,
    create_derived_project_record, create_project_record, resolve_snapshot_sources,
    resolve_snapshot_sources_for_scope, source_snapshot_chapter_content_by_snapshot_and_source,
    source_snapshot_chapter_for_project, source_snapshot_chapter_index, source_snapshot_chapters,
};
use a4n_db_models::models::chapter::chapter;
use a4n_domain::project::SourceSnapshotChapterIndexEntry;
use chrono::Utc;
use sea_orm::{ActiveModelTrait, EntityTrait, Set};

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
    let snapshot_id = derived.source_snapshot_id.expect("snapshot id must exist");

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

    let full = source_snapshot_chapters(&db, derived.id)
        .await
        .expect("full snapshot chapters must load");
    let index = source_snapshot_chapter_index(&db, derived.id)
        .await
        .expect("snapshot chapter index must load");
    assert_eq!(
        index,
        vec![SourceSnapshotChapterIndexEntry {
            id: full[0].id,
            snapshot_id,
            source_chapter_id: source_chapter.id,
            chapter_number: 1,
            title: Some("第一章".to_string()),
        }]
    );
    let detail =
        source_snapshot_chapter_for_project(&db, derived.id, snapshot_id, source_chapter.id)
            .await
            .expect("snapshot chapter detail must load");
    assert_eq!(detail, full.first().cloned());
    assert_eq!(
        detail.as_ref().map(|chapter| chapter.content.as_str()),
        Some("源章节正文")
    );
    assert!(
        source_snapshot_chapter_for_project(&db, derived.id, snapshot_id + 1, source_chapter.id)
            .await
            .expect("stale snapshot scope query must execute")
            .is_none()
    );
}

#[tokio::test]
#[ignore = "manual performance comparison for source snapshot chapter reads"]
async fn performance_source_snapshot_reads_1000_chapters() {
    use std::time::Instant;

    use serde_json::json;

    const CHAPTER_COUNT: i32 = 1000;
    const SAMPLES: usize = 15;

    let db = a4n_migration::test_support::connect_test_database().await;
    let root = create_project_record(
        &db,
        CreateProjectRecordInput {
            user_id: 1,
            title: format!("源快照性能测试 {}", uuid::Uuid::new_v4()),
            description: String::new(),
            project_type: "novel_long".to_string(),
        },
    )
    .await
    .expect("test project must be created");
    let now = Utc::now().naive_utc();
    let content = format!("{}。", "章节性能内容".repeat(512));
    let summary = "性能基准摘要。".repeat(32);
    let analysis = json!({
        "key_events": (0..5).map(|event| json!({
            "description": format!("事件{event}：{}", "灯火照亮河岸，林舟追寻来客留下的足迹。".repeat(6)),
            "type": "discovery",
            "importance": "medium",
            "involved_characters": ["林舟"]
        })).collect::<Vec<_>>()
    });
    let chapters: Vec<chapter::ActiveModel> = (1..=CHAPTER_COUNT)
        .map(|chapter_number| chapter::ActiveModel {
            project_id: Set(root.id),
            chapter_number: Set(chapter_number),
            title: Set(Some(format!("性能章节 {chapter_number}"))),
            content: Set(content.clone()),
            summary: Set(Some(summary.clone())),
            analysis_data: Set(Some(analysis.clone())),
            word_count: Set(content.chars().count() as i32),
            status: Set(chapter::STATUS_COMPLETED.to_string()),
            created_at: Set(now),
            updated_at: Set(now),
            ..Default::default()
        })
        .collect();
    chapter::Entity::insert_many(chapters)
        .exec(&db)
        .await
        .expect("synthetic chapters must be inserted");
    let derived = create_derived_project_record(
        &db,
        1,
        CreateDerivedProjectParams {
            root_project_id: root.id,
            derived_kind: "comic".to_string(),
            title: "性能测试漫画项目".to_string(),
            description: String::new(),
            chapter_scope: DerivedChapterScope {
                start_chapter: 1,
                end_chapter: CHAPTER_COUNT,
            },
        },
    )
    .await
    .expect("synthetic source snapshot must be created");
    let full = source_snapshot_chapters(&db, derived.id)
        .await
        .expect("full source snapshot must load");
    let index = source_snapshot_chapter_index(&db, derived.id)
        .await
        .expect("source snapshot index must load");
    assert_eq!(full.len(), CHAPTER_COUNT as usize);
    assert_eq!(index.len(), full.len());
    assert!(index.iter().zip(&full).all(|(index, chapter)| {
        index.snapshot_id == chapter.snapshot_id
            && index.id == chapter.id
            && index.source_chapter_id == chapter.source_chapter_id
            && index.chapter_number == chapter.chapter_number
            && index.title == chapter.title
    }));
    let first = full.first().expect("fixture must contain chapters");
    let detail = source_snapshot_chapter_for_project(
        &db,
        derived.id,
        first.snapshot_id,
        first.source_chapter_id,
    )
    .await
    .expect("first source snapshot detail must load");
    assert_eq!(detail.as_ref(), Some(first));

    async fn full_response(db: &impl a4n_db::DbExecutor, project_id: i64) -> (f64, usize) {
        let started = Instant::now();
        let chapters = source_snapshot_chapters(db, project_id)
            .await
            .expect("full source snapshot must load");
        let bytes = serde_json::to_vec(&chapters)
            .expect("full response must serialize")
            .len();
        (started.elapsed().as_secs_f64() * 1000.0, bytes)
    }

    async fn index_and_selected_detail_response(
        db: &impl a4n_db::DbExecutor,
        project_id: i64,
    ) -> (f64, usize) {
        let started = Instant::now();
        let chapters = source_snapshot_chapter_index(db, project_id)
            .await
            .expect("source snapshot index must load");
        let index_bytes = serde_json::to_vec(&chapters)
            .expect("source snapshot index must serialize")
            .len();
        let selected = chapters.first().expect("fixture must contain chapters");
        let detail = source_snapshot_chapter_for_project(
            db,
            project_id,
            selected.snapshot_id,
            selected.source_chapter_id,
        )
        .await
        .expect("selected source snapshot detail must load")
        .expect("selected source snapshot detail must exist");
        let detail_bytes = serde_json::to_vec(&detail)
            .expect("source snapshot detail must serialize")
            .len();
        (
            started.elapsed().as_secs_f64() * 1000.0,
            index_bytes + detail_bytes,
        )
    }

    async fn full_authoring_sources(
        db: &impl a4n_db::DbExecutor,
        project_id: i64,
        root_project_id: i64,
        snapshot_id: i64,
    ) -> (f64, usize, usize) {
        let started = Instant::now();
        let ids = source_snapshot_chapters(db, project_id)
            .await
            .expect("full source snapshot must load")
            .into_iter()
            .map(|chapter| chapter.source_chapter_id)
            .collect::<Vec<_>>();
        let sources = resolve_snapshot_sources(db, project_id, root_project_id, snapshot_id, &ids)
            .await
            .expect("all authoring sources must resolve");
        let content_bytes = sources.iter().map(|source| source.content.len()).sum();
        (
            started.elapsed().as_secs_f64() * 1000.0,
            sources.len(),
            content_bytes,
        )
    }

    async fn direct_authoring_sources(
        db: &impl a4n_db::DbExecutor,
        project_id: i64,
        root_project_id: i64,
        snapshot_id: i64,
    ) -> (f64, usize, usize) {
        let started = Instant::now();
        let sources =
            resolve_snapshot_sources_for_scope(db, project_id, root_project_id, snapshot_id)
                .await
                .expect("all authoring sources must resolve");
        let content_bytes = sources.iter().map(|source| source.content.len()).sum();
        (
            started.elapsed().as_secs_f64() * 1000.0,
            sources.len(),
            content_bytes,
        )
    }

    for _ in 0..2 {
        let _ = full_response(&db, derived.id).await;
        let _ = index_and_selected_detail_response(&db, derived.id).await;
        let _ = full_authoring_sources(&db, derived.id, root.id, first.snapshot_id).await;
        let _ = direct_authoring_sources(&db, derived.id, root.id, first.snapshot_id).await;
    }
    let mut full_samples = Vec::with_capacity(SAMPLES);
    let mut split_samples = Vec::with_capacity(SAMPLES);
    let mut full_bytes = 0;
    let mut split_bytes = 0;
    let mut full_authoring_samples = Vec::with_capacity(SAMPLES);
    let mut direct_authoring_samples = Vec::with_capacity(SAMPLES);
    for sample in 0..SAMPLES {
        let (full_ms, current_full_bytes);
        let (split_ms, current_split_bytes);
        if sample % 2 == 0 {
            (full_ms, current_full_bytes) = full_response(&db, derived.id).await;
            (split_ms, current_split_bytes) =
                index_and_selected_detail_response(&db, derived.id).await;
        } else {
            (split_ms, current_split_bytes) =
                index_and_selected_detail_response(&db, derived.id).await;
            (full_ms, current_full_bytes) = full_response(&db, derived.id).await;
        }
        assert!(full_bytes == 0 || current_full_bytes == full_bytes);
        assert!(split_bytes == 0 || current_split_bytes == split_bytes);
        full_bytes = current_full_bytes;
        split_bytes = current_split_bytes;
        full_samples.push(full_ms);
        split_samples.push(split_ms);
        let (full_authoring_ms, full_sources, direct_authoring_ms, direct_sources) =
            if sample % 2 == 0 {
                let (full_ms, full_count, full_bytes) =
                    full_authoring_sources(&db, derived.id, root.id, first.snapshot_id).await;
                let (direct_ms, direct_count, direct_bytes) =
                    direct_authoring_sources(&db, derived.id, root.id, first.snapshot_id).await;
                (
                    full_ms,
                    (full_count, full_bytes),
                    direct_ms,
                    (direct_count, direct_bytes),
                )
            } else {
                let (direct_ms, direct_count, direct_bytes) =
                    direct_authoring_sources(&db, derived.id, root.id, first.snapshot_id).await;
                let (full_ms, full_count, full_bytes) =
                    full_authoring_sources(&db, derived.id, root.id, first.snapshot_id).await;
                (
                    full_ms,
                    (full_count, full_bytes),
                    direct_ms,
                    (direct_count, direct_bytes),
                )
            };
        assert_eq!(full_sources, direct_sources);
        full_authoring_samples.push(full_authoring_ms);
        direct_authoring_samples.push(direct_authoring_ms);
    }
    full_samples.sort_by(f64::total_cmp);
    split_samples.sort_by(f64::total_cmp);
    let full_median = full_samples[SAMPLES / 2];
    let split_median = split_samples[SAMPLES / 2];
    let full_p95 = full_samples[SAMPLES - 1];
    let split_p95 = split_samples[SAMPLES - 1];
    let latency_reduction = (1.0 - split_median / full_median) * 100.0;
    let payload_reduction = (1.0 - split_bytes as f64 / full_bytes as f64) * 100.0;
    full_authoring_samples.sort_by(f64::total_cmp);
    direct_authoring_samples.sort_by(f64::total_cmp);
    let full_authoring_median = full_authoring_samples[SAMPLES / 2];
    let direct_authoring_median = direct_authoring_samples[SAMPLES / 2];
    let full_authoring_p95 = full_authoring_samples[SAMPLES - 1];
    let direct_authoring_p95 = direct_authoring_samples[SAMPLES - 1];
    let authoring_latency_reduction =
        (1.0 - direct_authoring_median / full_authoring_median) * 100.0;
    eprintln!(
        "PERF-020 chapters={CHAPTER_COUNT} full median/p95={full_median:.3}/{full_p95:.3} ms, {full_bytes} bytes; index+selected detail median/p95={split_median:.3}/{split_p95:.3} ms, {split_bytes} bytes; median reduction={latency_reduction:.2}%, payload reduction={payload_reduction:.2}%; all-source authoring full median/p95={full_authoring_median:.3}/{full_authoring_p95:.3} ms, direct scoped median/p95={direct_authoring_median:.3}/{direct_authoring_p95:.3} ms, reduction={authoring_latency_reduction:.2}%"
    );
}
