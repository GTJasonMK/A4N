use super::source_revision;
use chrono::Utc;
use serde_json::json;

#[test]
fn legacy_analysis_is_not_reported_as_new_fact_feedback() {
    let now = Utc::now().naive_utc();
    let mut chapter = a4n_domain::chapter::ChapterRecord {
        id: 1,
        project_id: 2,
        chapter_number: 1,
        outline_id: None,
        title: None,
        content: "施法会损失记忆。".into(),
        word_count: 9,
        summary: None,
        style_tags: None,
        status: "draft".into(),
        analysis_data: Some(json!({"summaries": {}})),
        created_at: now,
        updated_at: now,
    };
    assert!(source_revision(&chapter).is_none());
    chapter.analysis_data = Some(json!({"world_facts": []}));
    let first = source_revision(&chapter).unwrap();
    chapter.analysis_data.as_mut().unwrap()["_meta"] = json!({"attempt": 2});
    assert_eq!(source_revision(&chapter), Some(first));
}

#[cfg(test)]
mod performance_tests {
    use std::{collections::BTreeSet, time::Instant};

    use a4n_db_models::models::{chapter::chapter, project::material_fact_change};
    use chrono::Utc;
    use sea_orm::{
        ColumnTrait, ConnectionTrait, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder, Set,
    };
    use serde_json::{Value, json};

    use super::super::{
        FactFeedbackDbError, FeedbackSource, current_feedback_sources, source_revision,
    };

    const CHAPTER_COUNT: i32 = 1_000;
    const CONTENT_BYTES: usize = 9 * 1024;
    const ANALYSIS_BYTES: usize = 8 * 1024;
    const COMPLETED_FEEDBACK_COUNTS: [usize; 3] = [0, 100, 1_000];
    const SAMPLES: usize = 15;

    async fn baseline_current_feedback_sources<C: ConnectionTrait>(
        db: &C,
        project_id: i64,
    ) -> Result<Vec<FeedbackSource>, FactFeedbackDbError> {
        let chapters = chapter::Entity::find_current()
            .filter(chapter::Column::ProjectId.eq(project_id))
            .order_by_asc(chapter::Column::ChapterNumber)
            .all(db)
            .await?;
        let completed_records = material_fact_change::Entity::find()
            .filter(material_fact_change::Column::ProjectId.eq(project_id))
            .filter(material_fact_change::Column::MaterialKind.eq("chapter_feedback"))
            .filter(material_fact_change::Column::Status.eq("applied"))
            .all(db)
            .await?;
        let completed: BTreeSet<_> = completed_records
            .into_iter()
            .filter_map(|record| {
                record
                    .chapter_id
                    .map(|id| (id, record.after_json.to_string()))
            })
            .collect();
        let mut result = Vec::new();
        for chapter in chapters {
            let chapter = crate::project::chapter_record(chapter);
            let Some(revision) = source_revision(&chapter) else {
                continue;
            };
            let revision_json = serde_json::to_value(&revision)?;
            if !completed.contains(&(chapter.id, revision_json.to_string())) {
                continue;
            }
            let world_facts: Vec<a4n_domain::fact_feedback::WorldFact> = serde_json::from_value(
                chapter.analysis_data.as_ref().unwrap()["world_facts"].clone(),
            )?;
            for fact in &world_facts {
                fact.validate().map_err(FactFeedbackDbError::Invalid)?;
            }
            result.push(FeedbackSource {
                chapter,
                revision,
                world_facts,
            });
        }
        Ok(result)
    }

    async fn create_fixture(db: &DatabaseConnection) -> (i64, Vec<chapter::Model>) {
        let project = crate::project::create_project_record(
            db,
            crate::project::CreateProjectRecordInput {
                user_id: 1,
                title: format!("事实反馈来源扫描性能 {}", uuid::Uuid::new_v4()),
                description: String::new(),
                project_type: "novel_long".into(),
            },
        )
        .await
        .expect("benchmark project must be created");
        let content = "x".repeat(CONTENT_BYTES);
        let analysis_payload = "y".repeat(ANALYSIS_BYTES);
        let now = Utc::now().naive_utc();
        let chapters = (1..=CHAPTER_COUNT)
            .map(|chapter_number| chapter::ActiveModel {
                project_id: Set(project.id),
                chapter_number: Set(chapter_number),
                title: Set(Some(format!("性能章节 {chapter_number}"))),
                content: Set(content.clone()),
                word_count: Set(CONTENT_BYTES as i32),
                status: Set(chapter::STATUS_COMPLETED.into()),
                analysis_data: Set(Some(json!({
                    "world_facts": [{
                        "field": "core_rules",
                        "subject": format!("事实 {chapter_number}"),
                        "statement": "正文中的规则。",
                        "assertion": "observed",
                        "source_span_ids": [format!("ch{chapter_number}:s1:f1")]
                    }],
                        "analysis_payload": analysis_payload.clone()
                }))),
                created_at: Set(now),
                updated_at: Set(now),
                ..Default::default()
            })
            .collect::<Vec<_>>();
        for batch in chapters.chunks(250) {
            chapter::Entity::insert_many(batch.iter().cloned())
                .exec(db)
                .await
                .expect("benchmark chapters must be inserted");
        }
        let chapters = chapter::Entity::find_current()
            .filter(chapter::Column::ProjectId.eq(project.id))
            .order_by_asc(chapter::Column::ChapterNumber)
            .all(db)
            .await
            .expect("benchmark chapters must be loaded");
        assert_eq!(chapters.len(), CHAPTER_COUNT as usize);
        (project.id, chapters)
    }

    async fn insert_completed_feedback(
        db: &DatabaseConnection,
        project_id: i64,
        chapters: &[chapter::Model],
    ) {
        let now = Utc::now().naive_utc();
        let records = chapters
            .iter()
            .map(|model| {
                let chapter_record = crate::project::chapter_record(model.clone());
                let revision = source_revision(&chapter_record)
                    .expect("fixture chapters must have feedback sources");
                let after = serde_json::to_value(&revision).expect("revision must serialize");
                material_fact_change::ActiveModel {
                    project_id: Set(project_id),
                    workflow_run_id: Set(None),
                    chapter_id: Set(Some(model.id)),
                    change_key: Set(format!("perf-chapter-feedback-{}", model.id)),
                    material_kind: Set("chapter_feedback".into()),
                    target_id: Set(Some(model.id)),
                    field_path: Set(String::new()),
                    operation: Set("replace".into()),
                    before_json: Set(Value::Null),
                    after_json: Set(after.clone()),
                    source_refs_json: Set(json!([after])),
                    evidence_json: Set(json!([])),
                    reason: Set("performance fixture".into()),
                    status: Set("applied".into()),
                    created_at: Set(now),
                    invalidated_at: Set(None),
                    ..Default::default()
                }
            })
            .collect::<Vec<_>>();
        for batch in records.chunks(250) {
            material_fact_change::Entity::insert_many(batch.iter().cloned())
                .exec(db)
                .await
                .expect("feedback completion fixtures must be inserted");
        }
    }

    fn assert_sources_equal(left: &[FeedbackSource], right: &[FeedbackSource]) {
        assert_eq!(left.len(), right.len());
        for (left, right) in left.iter().zip(right) {
            assert_eq!(left.chapter, right.chapter);
            assert_eq!(left.revision, right.revision);
            assert_eq!(left.world_facts, right.world_facts);
        }
    }

    fn percentile(samples: &[f64], percentile: f64) -> f64 {
        let mut sorted = samples.to_vec();
        sorted.sort_by(f64::total_cmp);
        let index = ((sorted.len() - 1) as f64 * percentile).round() as usize;
        sorted[index]
    }

    async fn baseline_sample(
        db: &DatabaseConnection,
        project_id: i64,
    ) -> (f64, Vec<FeedbackSource>) {
        let started = Instant::now();
        let rows = baseline_current_feedback_sources(db, project_id)
            .await
            .expect("baseline feedback sources must load");
        (started.elapsed().as_secs_f64() * 1_000.0, rows)
    }

    async fn current_sample(
        db: &DatabaseConnection,
        project_id: i64,
    ) -> (f64, Vec<FeedbackSource>) {
        let started = Instant::now();
        let rows = current_feedback_sources(db, project_id)
            .await
            .expect("current feedback sources must load");
        (started.elapsed().as_secs_f64() * 1_000.0, rows)
    }

    async fn benchmark_coverage(db: &DatabaseConnection, project_id: i64, expected_sources: usize) {
        for _ in 0..2 {
            let (_, baseline) = baseline_sample(db, project_id).await;
            let (_, current) = current_sample(db, project_id).await;
            assert_eq!(baseline.len(), expected_sources);
            assert_sources_equal(&baseline, &current);
        }
        let mut baseline_samples = Vec::with_capacity(SAMPLES);
        let mut current_samples = Vec::with_capacity(SAMPLES);
        for sample in 0..SAMPLES {
            if sample % 2 == 0 {
                let (elapsed, baseline) = baseline_sample(db, project_id).await;
                baseline_samples.push(elapsed);
                let (elapsed, current) = current_sample(db, project_id).await;
                current_samples.push(elapsed);
                assert_sources_equal(&baseline, &current);
            } else {
                let (elapsed, current) = current_sample(db, project_id).await;
                current_samples.push(elapsed);
                let (elapsed, baseline) = baseline_sample(db, project_id).await;
                baseline_samples.push(elapsed);
                assert_sources_equal(&baseline, &current);
            }
        }
        println!(
            "PERF fact_feedback_sources chapters={CHAPTER_COUNT} content_bytes={CONTENT_BYTES} analysis_bytes={ANALYSIS_BYTES} completed_sources={expected_sources} poll_interval_ms=1500 samples={SAMPLES} baseline_p50_ms={:.3} baseline_p95_ms={:.3} current_p50_ms={:.3} current_p95_ms={:.3}",
            percentile(&baseline_samples, 0.50),
            percentile(&baseline_samples, 0.95),
            percentile(&current_samples, 0.50),
            percentile(&current_samples, 0.95),
        );
    }

    #[tokio::test]
    #[ignore = "manual PostgreSQL performance benchmark; requires isolated A4N_TEST_DATABASE_URL"]
    async fn benchmark_current_feedback_sources_skips_unapplied_chapters() {
        let db = a4n_migration::test_support::connect_test_database().await;
        let (project_id, chapters) = create_fixture(&db).await;
        let mut inserted_feedback = 0;
        for completed_sources in COMPLETED_FEEDBACK_COUNTS {
            if completed_sources > inserted_feedback {
                insert_completed_feedback(
                    &db,
                    project_id,
                    &chapters[inserted_feedback..completed_sources],
                )
                .await;
                inserted_feedback = completed_sources;
            }
            benchmark_coverage(&db, project_id, completed_sources).await;
        }
    }
}
