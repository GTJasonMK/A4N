use a4n_db::manga::{
    manga_pipeline_artifact, manga_pipeline_artifacts_by_chapter,
    manga_pipeline_artifacts_by_chapters,
};
use a4n_db::project::{CreateProjectRecordInput, create_project_record};
use a4n_db_models::models::chapter::{chapter, manga_pipeline_artifact as artifact};
use a4n_domain::manga::MangaPipelineArtifactStage;
use chrono::Utc;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, QueryOrder, Set};
use serde_json::json;
use std::time::Instant;

const CHAPTER_COUNT: i32 = 1_000;
const SAMPLES: usize = 15;

fn percentile(samples: &[f64], percentile: f64) -> f64 {
    let mut sorted = samples.to_vec();
    sorted.sort_by(f64::total_cmp);
    let index = ((sorted.len() - 1) as f64 * percentile).round() as usize;
    sorted[index]
}

async fn read_one(
    db: &impl a4n_db::DbExecutor,
    project_id: i64,
    chapter_ids: &[i64],
) -> Vec<Option<a4n_domain::manga::MangaPipelineArtifactRecord>> {
    let mut records = Vec::with_capacity(chapter_ids.len());
    for chapter_id in chapter_ids {
        records.push(
            manga_pipeline_artifact(
                db,
                project_id,
                *chapter_id,
                MangaPipelineArtifactStage::Preprocessing,
                None,
            )
            .await
            .expect("single artifact query must succeed"),
        );
    }
    records
}

#[tokio::test]
#[ignore = "manual PostgreSQL benchmark; needs isolated A4N_TEST_DATABASE_URL"]
async fn performance_manga_pipeline_artifacts_batch_1000_chapters() {
    let db = a4n_migration::test_support::connect_test_database().await;
    let project = create_project_record(
        &db,
        CreateProjectRecordInput {
            user_id: 1,
            title: format!("漫画阶段产物批量查询性能 {}", uuid::Uuid::new_v4()),
            description: String::new(),
            project_type: "novel_long".to_string(),
        },
    )
    .await
    .expect("test project must be created");
    let now = Utc::now().naive_utc();
    let chapters = (1..=CHAPTER_COUNT)
        .map(|chapter_number| chapter::ActiveModel {
            project_id: Set(project.id),
            chapter_number: Set(chapter_number),
            title: Set(Some(format!("性能章节 {chapter_number}"))),
            content: Set(format!("章节正文 {chapter_number}")),
            word_count: Set(6),
            status: Set(chapter::STATUS_COMPLETED.to_string()),
            created_at: Set(now),
            updated_at: Set(now),
            ..Default::default()
        })
        .collect::<Vec<_>>();
    chapter::Entity::insert_many(chapters)
        .exec(&db)
        .await
        .expect("synthetic chapters must be inserted");
    let chapter_ids = chapter::Entity::find()
        .filter(chapter::Column::ProjectId.eq(project.id))
        .order_by_asc(chapter::Column::ChapterNumber)
        .all(&db)
        .await
        .expect("synthetic chapters must be loaded")
        .into_iter()
        .map(|chapter| chapter.id)
        .collect::<Vec<_>>();
    assert_eq!(chapter_ids.len(), CHAPTER_COUNT as usize);

    let artifacts = chapter_ids
        .iter()
        .map(|chapter_id| artifact::ActiveModel {
            project_id: Set(project.id),
            chapter_id: Set(*chapter_id),
            stage: Set(MangaPipelineArtifactStage::Preprocessing.key().to_string()),
            page_number: Set(0),
            input_fingerprint: Set(format!("input-{chapter_id}")),
            output_fingerprint: Set(format!("output-{chapter_id}")),
            payload: Set(json!({"chapter_id": chapter_id, "content": "preprocessing"})),
            stale: Set(false),
            created_at: Set(now),
            updated_at: Set(now),
            ..Default::default()
        })
        .collect::<Vec<_>>();
    artifact::Entity::insert_many(artifacts)
        .exec(&db)
        .await
        .expect("synthetic artifacts must be inserted");

    let expected = manga_pipeline_artifacts_by_chapter(&db, project.id, chapter_ids[0])
        .await
        .expect("single chapter query must succeed");
    let batched = manga_pipeline_artifacts_by_chapters(
        &db,
        project.id,
        &chapter_ids,
        MangaPipelineArtifactStage::Preprocessing,
    )
    .await
    .expect("batch artifact query must succeed");
    assert_eq!(batched.len(), chapter_ids.len());
    assert_eq!(expected, vec![batched[&chapter_ids[0]].clone()]);

    for _ in 0..2 {
        let _ = read_one(&db, project.id, &chapter_ids).await;
        let _ = manga_pipeline_artifacts_by_chapters(
            &db,
            project.id,
            &chapter_ids,
            MangaPipelineArtifactStage::Preprocessing,
        )
        .await
        .expect("batch warmup must succeed");
    }

    let mut single_samples = Vec::with_capacity(SAMPLES);
    let mut batch_samples = Vec::with_capacity(SAMPLES);
    for sample in 0..SAMPLES {
        if sample % 2 == 0 {
            let started = Instant::now();
            let single = read_one(&db, project.id, &chapter_ids).await;
            single_samples.push(started.elapsed().as_secs_f64() * 1000.0);

            let started = Instant::now();
            let batch = manga_pipeline_artifacts_by_chapters(
                &db,
                project.id,
                &chapter_ids,
                MangaPipelineArtifactStage::Preprocessing,
            )
            .await
            .expect("batch artifact query must succeed");
            batch_samples.push(started.elapsed().as_secs_f64() * 1000.0);
            assert_eq!(
                single
                    .into_iter()
                    .zip(&chapter_ids)
                    .map(|(record, chapter_id)| (chapter_id, record))
                    .collect::<Vec<_>>(),
                chapter_ids
                    .iter()
                    .map(|chapter_id| (chapter_id, batch.get(chapter_id).cloned()))
                    .collect::<Vec<_>>()
            );
        } else {
            let started = Instant::now();
            let batch = manga_pipeline_artifacts_by_chapters(
                &db,
                project.id,
                &chapter_ids,
                MangaPipelineArtifactStage::Preprocessing,
            )
            .await
            .expect("batch artifact query must succeed");
            batch_samples.push(started.elapsed().as_secs_f64() * 1000.0);

            let started = Instant::now();
            let single = read_one(&db, project.id, &chapter_ids).await;
            single_samples.push(started.elapsed().as_secs_f64() * 1000.0);
            assert_eq!(
                single
                    .into_iter()
                    .zip(&chapter_ids)
                    .map(|(record, chapter_id)| (chapter_id, record))
                    .collect::<Vec<_>>(),
                chapter_ids
                    .iter()
                    .map(|chapter_id| (chapter_id, batch.get(chapter_id).cloned()))
                    .collect::<Vec<_>>()
            );
        }
    }

    let single_p50 = percentile(&single_samples, 0.50);
    let batch_p50 = percentile(&batch_samples, 0.50);
    let single_p95 = percentile(&single_samples, 0.95);
    let batch_p95 = percentile(&batch_samples, 0.95);
    println!(
        "PERF-023 manga artifacts: single p50={single_p50:.3}ms p95={single_p95:.3}ms; batch p50={batch_p50:.3}ms p95={batch_p95:.3}ms; p50_reduction={:.2}% p95_reduction={:.2}% records={}",
        (1.0 - batch_p50 / single_p50) * 100.0,
        (1.0 - batch_p95 / single_p95) * 100.0,
        chapter_ids.len()
    );
    assert!(
        (1.0 - batch_p50 / single_p50) >= 0.30,
        "batch p50 must be at least 30% faster"
    );
}
