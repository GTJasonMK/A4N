use a4n_db::auth::create_user_summary;
use a4n_db::collaboration::{project_ids_with_permission, verify_project_permission};
use a4n_db::media_authoring::{video_reference_script, video_reference_scripts_by_ids};
use a4n_db::project::{
    CreateDerivedProjectParams, CreateProjectRecordInput, DerivedChapterScope,
    create_derived_project_record, create_project_record,
};
use a4n_db_models::models::chapter::{chapter, chapter_shooting_script};
use a4n_domain::collaboration::ProjectAccessRole;
use chrono::Utc;
use sea_orm::{ActiveModelTrait, EntityTrait, Set};
use serde_json::json;
use std::time::Instant;

const SCRIPT_COUNT: usize = 1_000;
const SAMPLES: usize = 15;

fn percentile(samples: &[f64], percentile: f64) -> f64 {
    let mut sorted = samples.to_vec();
    sorted.sort_by(f64::total_cmp);
    let index = ((sorted.len() - 1) as f64 * percentile).round() as usize;
    sorted[index]
}

async fn read_legacy(
    db: &impl a4n_db::DbExecutor,
    user_id: i64,
    project_id: i64,
    script_ids: &[i64],
) -> Vec<Option<a4n_domain::shooting_script::ChapterShootingScriptReferenceRecord>> {
    let mut records = Vec::with_capacity(script_ids.len());
    for script_id in script_ids {
        verify_project_permission(db, project_id, user_id, ProjectAccessRole::Viewer)
            .await
            .expect("single permission query must succeed");
        records.push(
            video_reference_script(db, project_id, *script_id)
                .await
                .expect("single script query must succeed"),
        );
    }
    records
}

#[tokio::test]
#[ignore = "manual PostgreSQL benchmark; needs isolated A4N_TEST_DATABASE_URL"]
async fn performance_video_reference_status_batch_1000_scripts() {
    let db = a4n_migration::test_support::connect_test_database().await;
    let user = create_user_summary(
        &db,
        &format!("video-batch-{}", uuid::Uuid::new_v4()),
        &format!("video-batch-{}@example.com", uuid::Uuid::new_v4()),
        "password",
        "user",
    )
    .await
    .expect("test user must be created");
    let root = create_project_record(
        &db,
        CreateProjectRecordInput {
            user_id: user.id,
            title: format!("视频参考批量查询性能 {}", uuid::Uuid::new_v4()),
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
        content: Set("视频参考性能正文".to_string()),
        word_count: Set(9),
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
        user.id,
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
            script_json: Set(json!({"schema_version": "chapter_shooting_script_v2", "units": []})),
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
    let script_ids = chapter_shooting_script::Entity::find()
        .all(&db)
        .await
        .expect("synthetic scripts must be loaded")
        .into_iter()
        .filter(|script| script.project_id == video.id)
        .map(|script| script.id)
        .collect::<Vec<_>>();
    assert_eq!(script_ids.len(), SCRIPT_COUNT);

    for _ in 0..2 {
        let _ = read_legacy(&db, user.id, video.id, &script_ids).await;
        let visible =
            project_ids_with_permission(&db, &[video.id], user.id, ProjectAccessRole::Viewer)
                .await
                .expect("batch permission warmup must succeed");
        assert_eq!(visible, vec![video.id]);
        let _ = video_reference_scripts_by_ids(
            &db,
            &script_ids
                .iter()
                .map(|id| (video.id, *id))
                .collect::<Vec<_>>(),
        )
        .await
        .expect("batch script warmup must succeed");
    }

    let mut legacy_samples = Vec::with_capacity(SAMPLES);
    let mut batch_samples = Vec::with_capacity(SAMPLES);
    for sample in 0..SAMPLES {
        let references = script_ids
            .iter()
            .map(|script_id| (video.id, *script_id))
            .collect::<Vec<_>>();
        if sample % 2 == 0 {
            let started = Instant::now();
            let legacy = read_legacy(&db, user.id, video.id, &script_ids).await;
            legacy_samples.push(started.elapsed().as_secs_f64() * 1000.0);

            let started = Instant::now();
            let visible =
                project_ids_with_permission(&db, &[video.id], user.id, ProjectAccessRole::Viewer)
                    .await
                    .expect("batch permission query must succeed");
            let batch = video_reference_scripts_by_ids(&db, &references)
                .await
                .expect("batch script query must succeed");
            batch_samples.push(started.elapsed().as_secs_f64() * 1000.0);
            assert_eq!(visible, vec![video.id]);
            assert_eq!(batch.len(), SCRIPT_COUNT);
            assert_eq!(
                legacy
                    .into_iter()
                    .zip(&script_ids)
                    .map(|(record, script_id)| (script_id, record))
                    .collect::<Vec<_>>(),
                script_ids
                    .iter()
                    .map(|script_id| (script_id, batch.get(&(video.id, *script_id)).cloned()))
                    .collect::<Vec<_>>()
            );
        } else {
            let started = Instant::now();
            let visible =
                project_ids_with_permission(&db, &[video.id], user.id, ProjectAccessRole::Viewer)
                    .await
                    .expect("batch permission query must succeed");
            let batch = video_reference_scripts_by_ids(&db, &references)
                .await
                .expect("batch script query must succeed");
            batch_samples.push(started.elapsed().as_secs_f64() * 1000.0);

            let started = Instant::now();
            let legacy = read_legacy(&db, user.id, video.id, &script_ids).await;
            legacy_samples.push(started.elapsed().as_secs_f64() * 1000.0);
            assert_eq!(visible, vec![video.id]);
            assert_eq!(batch.len(), SCRIPT_COUNT);
            assert_eq!(
                legacy
                    .into_iter()
                    .zip(&script_ids)
                    .map(|(record, script_id)| (script_id, record))
                    .collect::<Vec<_>>(),
                script_ids
                    .iter()
                    .map(|script_id| (script_id, batch.get(&(video.id, *script_id)).cloned()))
                    .collect::<Vec<_>>()
            );
        }
    }

    let legacy_p50 = percentile(&legacy_samples, 0.50);
    let batch_p50 = percentile(&batch_samples, 0.50);
    let legacy_p95 = percentile(&legacy_samples, 0.95);
    let batch_p95 = percentile(&batch_samples, 0.95);
    println!(
        "PERF-024 video references: legacy p50={legacy_p50:.3}ms p95={legacy_p95:.3}ms; batch p50={batch_p50:.3}ms p95={batch_p95:.3}ms; p50_reduction={:.2}% p95_reduction={:.2}% records={SCRIPT_COUNT}",
        (1.0 - batch_p50 / legacy_p50) * 100.0,
        (1.0 - batch_p95 / legacy_p95) * 100.0,
    );
    assert!(
        (1.0 - batch_p50 / legacy_p50) >= 0.30,
        "batch p50 must be at least 30% faster"
    );
}
