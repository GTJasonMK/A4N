use super::{
    FACT_CONSOLIDATION_KIND, FACT_REBUILD_KIND, ProjectFactFeedbackSummary, business_key,
    feedback_error,
};
use crate::OrchestratorContext;
use crate::test_support::{TestOrchestratorBuilder, create_test_user};
use a4n_api_contract::fact_feedback::MaterialFactChangeSummary;
use a4n_db::project::{CreateProjectRecordInput, create_project_record};
use a4n_db_models::models::{chapter::chapter, project::material_fact_change};
use a4n_domain::{
    chapter::ChapterRecord,
    collaboration::ProjectAccessRole,
    fact_feedback::{FactSourceRevision, pending_fact_sources},
};
use chrono::Utc;
use sea_orm::{
    ColumnTrait, ConnectionTrait, DbBackend, EntityTrait, QueryFilter, QueryOrder, Set, Statement,
};
use serde_json::{Value, json};
use std::time::Instant;

const CHAPTER_COUNT: i32 = 1_000;
const CHAPTER_CONTENT_BYTES: usize = 9 * 1024;
const ANALYSIS_PAYLOAD_BYTES: usize = 8 * 1024;
const MATERIAL_CHANGE_HISTORY_RECORDS: i32 = 10_000;
const WARMUPS: usize = 2;
const SAMPLES: usize = 15;

struct CoverageCase {
    label: &'static str,
    applied_sources: i32,
    project_id: i64,
}

fn chapter_analysis(chapter_number: i32) -> Value {
    json!({
        "metadata": {
            "characters": ["林舟", "沈青"],
            "locations": ["河岸"],
            "tone": "紧张",
            "timeline_marker": format!("第{chapter_number}天")
        },
        "summaries": {
            "compressed": format!("第{chapter_number}章调查账本的来历"),
            "keywords": ["账本", "调查"]
        },
        "key_events": [],
        "world_facts": [],
        "unused_analysis_payload": "a".repeat(ANALYSIS_PAYLOAD_BYTES)
    })
}

fn chapter_record(row: chapter::Model) -> ChapterRecord {
    ChapterRecord {
        id: row.id,
        project_id: row.project_id,
        outline_id: row.outline_id,
        chapter_number: row.chapter_number,
        title: row.title,
        content: row.content,
        word_count: row.word_count,
        status: row.status,
        summary: row.summary,
        style_tags: row.style_tags,
        analysis_data: row.analysis_data,
        created_at: row.created_at,
        updated_at: row.updated_at,
    }
}

async fn create_coverage_case(
    db: &a4n_db::DatabaseConnection,
    user_id: i64,
    label: &'static str,
    applied_sources: i32,
) -> CoverageCase {
    let project = create_project_record(
        db,
        CreateProjectRecordInput {
            user_id,
            title: format!("事实反馈性能夹具-{label}"),
            description: String::new(),
            project_type: "novel_long".into(),
        },
    )
    .await
    .expect("fact-feedback benchmark project must be created");
    let now = Utc::now().naive_utc();
    let content = "x".repeat(CHAPTER_CONTENT_BYTES);
    let rows = (1..=CHAPTER_COUNT)
        .map(|chapter_number| chapter::ActiveModel {
            project_id: Set(project.id),
            chapter_number: Set(chapter_number),
            title: Set(Some(format!("第{chapter_number}章"))),
            summary: Set(Some(format!("第{chapter_number}章摘要"))),
            content: Set(content.clone()),
            word_count: Set(CHAPTER_CONTENT_BYTES as i32),
            status: Set(chapter::STATUS_DRAFT.into()),
            analysis_data: Set(Some(chapter_analysis(chapter_number))),
            created_at: Set(now),
            updated_at: Set(now),
            ..Default::default()
        })
        .collect::<Vec<_>>();
    for batch in rows.chunks(250) {
        chapter::Entity::insert_many(batch.iter().cloned())
            .exec(db)
            .await
            .expect("fact-feedback benchmark chapters must be inserted");
    }

    let chapters = chapter::Entity::find()
        .filter(chapter::Column::ProjectId.eq(project.id))
        .order_by_asc(chapter::Column::ChapterNumber)
        .all(db)
        .await
        .expect("fact-feedback benchmark chapters must be read")
        .into_iter()
        .take(applied_sources as usize)
        .map(chapter_record)
        .collect::<Vec<_>>();
    let changes = chapters
        .into_iter()
        .map(|chapter| {
            let revision = a4n_db::fact_feedback::source_revision(&chapter)
                .expect("benchmark source must have a valid revision");
            let after =
                serde_json::to_value(&revision).expect("benchmark source revision must serialize");
            material_fact_change::ActiveModel {
                project_id: Set(project.id),
                workflow_run_id: Set(None),
                chapter_id: Set(Some(chapter.id)),
                change_key: Set(format!("fact-feedback-benchmark-{label}-{}", chapter.id)),
                material_kind: Set("chapter_feedback".into()),
                target_id: Set(Some(chapter.id)),
                field_path: Set(String::new()),
                operation: Set("replace".into()),
                before_json: Set(Value::Null),
                after_json: Set(after.clone()),
                source_refs_json: Set(json!([after])),
                evidence_json: Set(json!([])),
                reason: Set("性能基准夹具".into()),
                status: Set("applied".into()),
                created_at: Set(now),
                invalidated_at: Set(None),
                ..Default::default()
            }
        })
        .collect::<Vec<_>>();
    for batch in changes.chunks(250) {
        material_fact_change::Entity::insert_many(batch.iter().cloned())
            .exec(db)
            .await
            .expect("fact-feedback benchmark applied changes must be inserted");
    }

    CoverageCase {
        label,
        applied_sources,
        project_id: project.id,
    }
}

async fn create_summary_history(
    db: &a4n_db::DatabaseConnection,
    project_id: i64,
    label: &'static str,
) {
    let now = Utc::now().naive_utc();
    let payload = "h".repeat(256);
    let mut rows = (0..MATERIAL_CHANGE_HISTORY_RECORDS)
        .map(|sequence| {
            let before = json!({"revision": sequence, "payload": payload});
            let after = if sequence % 20 == 0 {
                before.clone()
            } else {
                json!({"revision": sequence + 1, "payload": payload})
            };
            material_fact_change::ActiveModel {
                project_id: Set(project_id),
                workflow_run_id: Set(None),
                chapter_id: Set(None),
                change_key: Set(format!("summary-history-{label}-{sequence}")),
                material_kind: Set("character_fact".into()),
                target_id: Set(Some(sequence as i64)),
                field_path: Set("name".into()),
                operation: Set("replace".into()),
                before_json: Set(before),
                after_json: Set(after),
                source_refs_json: Set(json!([])),
                evidence_json: Set(json!({"excerpt": payload})),
                reason: Set("性能基准夹具".into()),
                status: Set("applied".into()),
                created_at: Set(now),
                invalidated_at: Set(None),
                ..Default::default()
            }
        })
        .collect::<Vec<_>>();
    rows.push(material_fact_change::ActiveModel {
        project_id: Set(project_id),
        workflow_run_id: Set(None),
        chapter_id: Set(None),
        change_key: Set(format!("summary-history-{label}-number-kind")),
        material_kind: Set("character_fact".into()),
        target_id: Set(Some(i64::MAX)),
        field_path: Set("number".into()),
        operation: Set("replace".into()),
        before_json: Set(json!(1)),
        after_json: Set(json!(1.0)),
        source_refs_json: Set(json!([])),
        evidence_json: Set(json!([])),
        reason: Set("性能基准数字边界".into()),
        status: Set("applied".into()),
        created_at: Set(now),
        invalidated_at: Set(None),
        ..Default::default()
    });
    for sequence in 0..10 {
        rows.push(material_fact_change::ActiveModel {
            project_id: Set(project_id),
            workflow_run_id: Set(None),
            chapter_id: Set(None),
            change_key: Set(format!("summary-history-{label}-float-equal-{sequence}")),
            material_kind: Set("character_fact".into()),
            target_id: Set(Some(i64::MAX - sequence as i64 - 1)),
            field_path: Set("number".into()),
            operation: Set("replace".into()),
            before_json: Set(json!(-0.0)),
            after_json: Set(json!(0.0)),
            source_refs_json: Set(json!([])),
            evidence_json: Set(json!([])),
            reason: Set("性能基准等值边界".into()),
            status: Set("applied".into()),
            created_at: Set(now),
            invalidated_at: Set(None),
            ..Default::default()
        });
    }
    for sequence in 0..150 {
        let unchanged = json!({"unchanged": sequence, "payload": payload});
        rows.push(material_fact_change::ActiveModel {
            project_id: Set(project_id),
            workflow_run_id: Set(None),
            chapter_id: Set(None),
            change_key: Set(format!("summary-history-{label}-noop-{sequence}")),
            material_kind: Set("character_fact".into()),
            target_id: Set(Some(i64::MAX - sequence as i64 - 20)),
            field_path: Set("name".into()),
            operation: Set("replace".into()),
            before_json: Set(unchanged.clone()),
            after_json: Set(unchanged),
            source_refs_json: Set(json!([])),
            evidence_json: Set(json!({"excerpt": payload})),
            reason: Set("性能基准未变化夹具".into()),
            status: Set("applied".into()),
            created_at: Set(now),
            invalidated_at: Set(None),
            ..Default::default()
        });
    }
    for batch in rows.chunks(250) {
        material_fact_change::Entity::insert_many(batch.iter().cloned())
            .exec(db)
            .await
            .expect("fact-feedback benchmark history must be inserted");
    }
}

fn percentile(samples: &[f64], fraction: f64) -> f64 {
    let mut sorted = samples.to_vec();
    sorted.sort_by(f64::total_cmp);
    sorted[((sorted.len() - 1) as f64 * fraction).round() as usize]
}

async fn summary_history_index_scans(db: &a4n_db::DatabaseConnection) -> i64 {
    db.query_one(Statement::from_string(
        DbBackend::Postgres,
        "SELECT idx_scan FROM pg_stat_user_indexes
         WHERE schemaname = current_schema()
           AND indexrelname = 'idx_material_fact_change_summary_history'",
    ))
    .await
    .expect("fact-feedback summary index statistics must load")
    .expect("fact-feedback summary history index must exist")
    .try_get("", "idx_scan")
    .expect("fact-feedback summary index scan count must decode")
}

async fn measure_summary(
    context: &crate::OrchestratorContext,
    user_id: i64,
    project_id: i64,
    baseline: bool,
) -> (f64, ProjectFactFeedbackSummary, usize) {
    let started = Instant::now();
    let summary = if baseline {
        fact_feedback_summary_baseline(context, user_id, project_id)
            .await
            .expect("baseline fact-feedback summary must load")
    } else {
        context
            .fact_feedback_summary_for_user(user_id, project_id)
            .await
            .expect("production fact-feedback summary must load")
    };
    let response_bytes = serde_json::to_vec(&summary)
        .expect("fact-feedback summary must serialize")
        .len();
    (
        started.elapsed().as_secs_f64() * 1_000.0,
        summary,
        response_bytes,
    )
}

async fn fact_feedback_summary_baseline(
    context: &OrchestratorContext,
    user_id: i64,
    project_id: i64,
) -> Result<ProjectFactFeedbackSummary, a4n_error::AppError> {
    context
        .verify_project_permission(project_id, user_id, ProjectAccessRole::Viewer)
        .await?;
    let db = context.database.executor();
    let (config, sources, changes, chapters) = tokio::join!(
        a4n_db::fact_feedback::feedback_config(db, project_id),
        a4n_db::fact_feedback::current_feedback_sources(db, project_id),
        a4n_db::fact_feedback::feedback_changes(db, project_id),
        context.chapters_for_project(project_id),
    );
    let config = config?;
    let sources = sources.map_err(feedback_error)?;
    let revisions = sources
        .iter()
        .map(|source| source.revision.clone())
        .collect::<Vec<_>>();
    let consumed: Vec<FactSourceRevision> =
        serde_json::from_value(config.consolidated_sources_json)?;
    let pending_chapters = pending_fact_sources(&revisions, &consumed)
        .iter()
        .map(|source| source.chapter_number)
        .collect();
    let consolidated_chapters = consumed
        .iter()
        .filter(|source| revisions.contains(source))
        .map(|source| source.chapter_number)
        .collect();
    let latest = [
        context
            .latest_workflow_run_by_business_key(&business_key(project_id, FACT_CONSOLIDATION_KIND))
            .await?,
        context
            .latest_workflow_run_by_business_key(&business_key(project_id, FACT_REBUILD_KIND))
            .await?,
    ]
    .into_iter()
    .flatten()
    .max_by_key(|run| (run.status == "running", run.updated_at.clone(), run.id));
    Ok(ProjectFactFeedbackSummary {
        project_id,
        interval_chapters: config.interval_chapters,
        pending_chapters,
        consolidated_chapters,
        legacy_chapters: chapters?
            .into_iter()
            .filter(|chapter| {
                chapter.has_available_content()
                    && !revisions
                        .iter()
                        .any(|source| source.chapter_id == chapter.id)
            })
            .map(|chapter| chapter.chapter_number)
            .collect(),
        last_consolidated_at: config
            .last_consolidated_at
            .map(|time| time.and_utc().to_rfc3339()),
        workflow_run_id: latest.as_ref().map(|run| run.id),
        workflow_status: latest.as_ref().map(|run| run.status.clone()),
        workflow_kind: latest.as_ref().map(|run| run.workflow_kind.clone()),
        can_resume: latest.as_ref().is_some_and(|run| run.can_resume),
        error: latest.and_then(|run| run.error_message),
        changes: changes?
            .into_iter()
            .filter(|change| change.before_json != change.after_json)
            .take(100)
            .map(|change| MaterialFactChangeSummary {
                id: change.id,
                material_kind: change.material_kind,
                target_id: change.target_id,
                field_path: change.field_path,
                before: change.before_json,
                after: change.after_json,
                evidence: change.evidence_json,
                reason: change.reason,
                status: change.status,
                created_at: change.created_at.and_utc().to_rfc3339(),
            })
            .collect(),
    })
}

#[tokio::test]
#[ignore = "manual PostgreSQL end-to-end performance benchmark; requires isolated A4N_TEST_DATABASE_URL"]
async fn benchmark_fact_feedback_summary_full_application_read() {
    let db = std::sync::Arc::new(a4n_migration::test_support::connect_test_database().await);
    let user_id = create_test_user(db.as_ref()).await;
    let context = TestOrchestratorBuilder::new(db.clone()).build().await;
    let cases = [
        create_coverage_case(db.as_ref(), user_id, "none", 0).await,
        create_coverage_case(db.as_ref(), user_id, "partial", 100).await,
        create_coverage_case(db.as_ref(), user_id, "full", CHAPTER_COUNT).await,
    ];
    for case in &cases {
        create_summary_history(db.as_ref(), case.project_id, case.label).await;
    }
    db.execute_unprepared("ANALYZE chapter")
        .await
        .expect("fact-feedback benchmark chapter statistics must be refreshed");
    db.execute_unprepared("ANALYZE material_fact_change")
        .await
        .expect("fact-feedback benchmark change statistics must be refreshed");
    let mut results = Vec::with_capacity(cases.len());
    for case in &cases {
        let expected_pending = case.applied_sources;
        let expected_legacy = CHAPTER_COUNT - case.applied_sources;
        let (_, baseline_warmup, baseline_bytes) =
            measure_summary(&context, user_id, case.project_id, true).await;
        let (_, candidate_warmup, candidate_bytes) =
            measure_summary(&context, user_id, case.project_id, false).await;
        assert_eq!(
            serde_json::to_value(&baseline_warmup).unwrap(),
            serde_json::to_value(&candidate_warmup).unwrap(),
            "baseline and candidate summary DTOs must match for {} coverage",
            case.label
        );
        assert_eq!(baseline_bytes, candidate_bytes);
        let response_bytes = candidate_bytes;
        let warmup = candidate_warmup;
        assert_eq!(warmup.project_id, case.project_id);
        assert_eq!(warmup.pending_chapters.len(), expected_pending as usize);
        assert_eq!(warmup.consolidated_chapters.len(), 0);
        assert_eq!(warmup.legacy_chapters.len(), expected_legacy as usize);
        assert_eq!(warmup.changes.len(), 100);

        for _ in 1..WARMUPS {
            let (_, baseline, baseline_response_bytes) =
                measure_summary(&context, user_id, case.project_id, true).await;
            let (_, summary, bytes) =
                measure_summary(&context, user_id, case.project_id, false).await;
            assert_eq!(
                serde_json::to_value(baseline).unwrap(),
                serde_json::to_value(&warmup).unwrap()
            );
            assert_eq!(
                serde_json::to_value(summary).unwrap(),
                serde_json::to_value(&warmup).unwrap()
            );
            assert_eq!(baseline_response_bytes, response_bytes);
            assert_eq!(bytes, response_bytes);
        }
        let mut baseline_samples = Vec::with_capacity(SAMPLES);
        let mut candidate_samples = Vec::with_capacity(SAMPLES);
        for sample in 0..SAMPLES {
            let (baseline_result, candidate_result) = if sample % 2 == 0 {
                (
                    measure_summary(&context, user_id, case.project_id, true).await,
                    measure_summary(&context, user_id, case.project_id, false).await,
                )
            } else {
                let candidate = measure_summary(&context, user_id, case.project_id, false).await;
                let baseline = measure_summary(&context, user_id, case.project_id, true).await;
                (baseline, candidate)
            };
            let (baseline_elapsed, baseline, baseline_response_bytes) = baseline_result;
            let (candidate_elapsed, summary, candidate_response_bytes) = candidate_result;
            assert_eq!(
                serde_json::to_value(baseline).unwrap(),
                serde_json::to_value(&warmup).unwrap()
            );
            assert_eq!(
                serde_json::to_value(summary).unwrap(),
                serde_json::to_value(&warmup).unwrap()
            );
            assert_eq!(baseline_response_bytes, response_bytes);
            assert_eq!(candidate_response_bytes, response_bytes);
            if sample >= 2 {
                baseline_samples.push(baseline_elapsed);
                candidate_samples.push(candidate_elapsed);
            }
        }
        results.push(json!({
            "coverage": case.label,
            "chapters": CHAPTER_COUNT,
            "chapter_content_bytes_each": CHAPTER_CONTENT_BYTES,
            "analysis_payload_bytes_each": ANALYSIS_PAYLOAD_BYTES,
            "material_change_history_records": MATERIAL_CHANGE_HISTORY_RECORDS + 161,
            "applied_sources": case.applied_sources,
            "pending_chapters": warmup.pending_chapters.len(),
            "legacy_chapters": warmup.legacy_chapters.len(),
            "summary_response_bytes": response_bytes,
            "baseline_p50_ms": percentile(&baseline_samples, 0.50),
            "baseline_p95_ms": percentile(&baseline_samples, 0.95),
            "candidate_p50_ms": percentile(&candidate_samples, 0.50),
            "candidate_p95_ms": percentile(&candidate_samples, 0.95),
            "p50_improvement_percent": (1.0 - percentile(&candidate_samples, 0.50) / percentile(&baseline_samples, 0.50)) * 100.0,
            "p95_improvement_percent": (1.0 - percentile(&candidate_samples, 0.95) / percentile(&baseline_samples, 0.95)) * 100.0,
            "baseline_samples": baseline_samples,
            "candidate_samples": candidate_samples,
        }));
    }
    let index_scans = summary_history_index_scans(db.as_ref()).await;
    assert!(
        index_scans > 0,
        "fact-feedback summary query must use its per-project history index"
    );
    println!("PERF fact_feedback_summary_index_scans={index_scans}");
    println!(
        "PERF fact_feedback_summary_full_application={}",
        json!(results)
    );
}
