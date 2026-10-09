use a4n_db::media_authoring::{video_reference_script_summaries, video_reference_scripts};
use a4n_db::project::{
    CreateDerivedProjectParams, CreateProjectRecordInput, DerivedChapterScope,
    create_derived_project_record, create_project_record,
};
use a4n_db_models::models::{chapter::chapter, chapter::chapter_shooting_script};
use chrono::Utc;
use sea_orm::{ActiveModelTrait, EntityTrait, Set};
use serde_json::json;
use std::time::Instant;

const SCRIPT_COUNT: usize = 1_000;
const SCRIPT_JSON_BYTES: usize = 64 * 1024;
const SAMPLES: usize = 15;

fn percentile(samples: &[f64], percentile: f64) -> f64 {
    let mut sorted = samples.to_vec();
    sorted.sort_by(f64::total_cmp);
    let index = ((sorted.len() - 1) as f64 * percentile).round() as usize;
    sorted[index]
}

fn assert_metadata_matches(
    full: &[(
        a4n_domain::project::ProjectRecord,
        a4n_domain::shooting_script::ChapterShootingScriptReferenceRecord,
    )],
    summaries: &[(
        a4n_domain::project::ProjectRecord,
        a4n_domain::shooting_script::ChapterShootingScriptReferenceSummary,
    )],
) {
    assert_eq!(full.len(), summaries.len());
    for ((full_project, full), (summary_project, summary)) in full.iter().zip(summaries) {
        assert_eq!(full_project.id, summary_project.id);
        assert_eq!(full.id, summary.id);
        assert_eq!(full.project_id, summary.project_id);
        assert_eq!(full.root_project_id, summary.root_project_id);
        assert_eq!(full.source_chapter_id, summary.source_chapter_id);
        assert_eq!(full.source_snapshot_id, summary.source_snapshot_id);
        assert_eq!(
            full.source_snapshot_chapter_id,
            summary.source_snapshot_chapter_id
        );
        assert_eq!(full.source_revision_key, summary.source_revision_key);
        assert_eq!(full.source_content_sha256, summary.source_content_sha256);
        assert_eq!(full.revision, summary.revision);
        assert_eq!(full.script_sha256, summary.script_sha256);
    }
}

#[tokio::test]
#[ignore = "manual PostgreSQL benchmark; needs isolated A4N_TEST_DATABASE_URL"]
async fn performance_video_reference_projection_1000_large_scripts() {
    let db = a4n_migration::test_support::connect_test_database().await;
    let root = create_project_record(
        &db,
        CreateProjectRecordInput {
            user_id: 1,
            title: format!("视频参考轻量投影性能 {}", uuid::Uuid::new_v4()),
            description: String::new(),
            project_type: "novel_long".to_string(),
        },
    )
    .await
    .expect("test project must be created");
    let now = Utc::now().naive_utc();
    let chapter = chapter::ActiveModel {
        project_id: Set(root.id),
        chapter_number: Set(1),
        title: Set(Some("第一章".to_string())),
        content: Set("视频参考轻量投影正文".to_string()),
        word_count: Set(10),
        status: Set(chapter::STATUS_COMPLETED.to_string()),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(&db)
    .await
    .expect("source chapter must be created");
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
                end_chapter: 1,
            },
        },
    )
    .await
    .expect("video project must be created");
    let large_payload = "x".repeat(SCRIPT_JSON_BYTES);
    let scripts = (0..SCRIPT_COUNT)
        .map(|index| chapter_shooting_script::ActiveModel {
            project_id: Set(video.id),
            root_project_id: Set(root.id),
            source_chapter_id: Set(chapter.id),
            source_snapshot_id: Set(video.source_snapshot_id),
            source_snapshot_chapter_id: Set(None),
            source_revision_key: Set(format!("snapshot:{index}")),
            source_content_sha256: Set("content-sha256".to_string()),
            visual_plan_id: Set(None),
            visual_plan_version: Set(None),
            visual_plan_sha256: Set(None),
            input_fingerprint_sha256: Set(format!("input-{index}")),
            contract_version: Set("chapter_shooting_script_v2".to_string()),
            revision: Set(1),
            script_sha256: Set(format!("script-{index}")),
            script_json: Set(
                json!({"schema_version": "chapter_shooting_script_v2", "payload": large_payload}),
            ),
            workflow_run_id: Set(None),
            created_at: Set(now),
            updated_at: Set(now),
            ..Default::default()
        })
        .collect::<Vec<_>>();
    chapter_shooting_script::Entity::insert_many(scripts)
        .exec(&db)
        .await
        .expect("synthetic scripts must be inserted");

    for _ in 0..2 {
        let _ = video_reference_scripts(&db, root.id, Some(chapter.id))
            .await
            .expect("full projection warmup must succeed");
        let _ = video_reference_script_summaries(&db, root.id, Some(chapter.id))
            .await
            .expect("summary projection warmup must succeed");
    }

    let mut full_samples = Vec::with_capacity(SAMPLES);
    let mut summary_samples = Vec::with_capacity(SAMPLES);
    let mut full_json_bytes = 0usize;
    for sample in 0..SAMPLES {
        if sample % 2 == 0 {
            let started = Instant::now();
            let full = video_reference_scripts(&db, root.id, Some(chapter.id))
                .await
                .expect("full projection query must succeed");
            full_samples.push(started.elapsed().as_secs_f64() * 1000.0);
            full_json_bytes = full
                .iter()
                .map(|(_, record)| serde_json::to_vec(&record.script_json).unwrap().len())
                .sum();

            let started = Instant::now();
            let summaries = video_reference_script_summaries(&db, root.id, Some(chapter.id))
                .await
                .expect("summary projection query must succeed");
            summary_samples.push(started.elapsed().as_secs_f64() * 1000.0);
            assert_metadata_matches(&full, &summaries);
        } else {
            let started = Instant::now();
            let summaries = video_reference_script_summaries(&db, root.id, Some(chapter.id))
                .await
                .expect("summary projection query must succeed");
            summary_samples.push(started.elapsed().as_secs_f64() * 1000.0);

            let started = Instant::now();
            let full = video_reference_scripts(&db, root.id, Some(chapter.id))
                .await
                .expect("full projection query must succeed");
            full_samples.push(started.elapsed().as_secs_f64() * 1000.0);
            full_json_bytes = full
                .iter()
                .map(|(_, record)| serde_json::to_vec(&record.script_json).unwrap().len())
                .sum();
            assert_metadata_matches(&full, &summaries);
        }
    }

    let full_p50 = percentile(&full_samples, 0.50);
    let summary_p50 = percentile(&summary_samples, 0.50);
    let full_p95 = percentile(&full_samples, 0.95);
    let summary_p95 = percentile(&summary_samples, 0.95);
    println!(
        "PERF-026 video reference projection: full p50={full_p50:.3}ms p95={full_p95:.3}ms; summary p50={summary_p50:.3}ms p95={summary_p95:.3}ms; p50_reduction={:.2}% p95_reduction={:.2}% json_bytes={full_json_bytes} summary_json_bytes=0 records={SCRIPT_COUNT}",
        (1.0 - summary_p50 / full_p50) * 100.0,
        (1.0 - summary_p95 / full_p95) * 100.0,
    );
    assert!(
        (1.0 - summary_p50 / full_p50) >= 0.30,
        "summary p50 must be at least 30% faster"
    );
}
