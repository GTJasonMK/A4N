use a4n_db::workflow::{
    GetWorkflowRunInput, UpsertWorkflowRunInput, upsert_workflow_run, workflow_run_detail,
};
use a4n_db_models::models::generation::generation_run;
use a4n_domain::workflow::WORKFLOW_STATUS_COMPLETED;
use chrono::Utc;
use sea_orm::{
    ColumnTrait, EntityTrait, FromQueryResult, QueryFilter, QueryOrder, QuerySelect, Set,
    sea_query::Expr,
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::time::Instant;

const GENERATION_RUN_COUNT: usize = 300;
const SAMPLES: usize = 9;

#[derive(Debug, Deserialize, FromQueryResult, Serialize)]
struct FullGenerationRunProjection {
    id: i64,
    workflow_run_id: Option<i64>,
    workflow: String,
    step_key: String,
    call_key: Option<String>,
    title: String,
    model_kind: String,
    provider: Option<String>,
    model: Option<String>,
    status: String,
    input_fingerprint_sha256: Option<String>,
    input_fingerprint_json: Option<Value>,
    raw_response_json: Option<Value>,
    parsed_output_json: Option<Value>,
    validation_json: Option<Value>,
    artifact_refs_json: Option<Value>,
    attempt: i32,
    response_summary_json: Option<Value>,
    error_message: Option<String>,
    created_at: chrono::NaiveDateTime,
    completed_at: Option<chrono::NaiveDateTime>,
}

#[derive(Debug, Deserialize, FromQueryResult, Serialize)]
struct SummaryGenerationRunProjection {
    id: i64,
    workflow_run_id: Option<i64>,
    workflow: String,
    step_key: String,
    call_key: Option<String>,
    title: String,
    model_kind: String,
    provider: Option<String>,
    model: Option<String>,
    status: String,
    input_fingerprint_sha256: Option<String>,
    attempt: i32,
    response_summary_json: Option<Value>,
    error_message: Option<String>,
    created_at: chrono::NaiveDateTime,
    completed_at: Option<chrono::NaiveDateTime>,
    has_raw_response: bool,
    has_parsed_output: bool,
    has_validation: bool,
}

fn percentile(samples: &[f64], percentile: f64) -> f64 {
    let mut sorted = samples.to_vec();
    sorted.sort_by(f64::total_cmp);
    sorted[((sorted.len() - 1) as f64 * percentile).round() as usize]
}

async fn fixture(db: &sea_orm::DatabaseConnection) -> i64 {
    let workflow = upsert_workflow_run(
        db,
        UpsertWorkflowRunInput {
            business_key: format!("workflow-detail-payload:{}", uuid::Uuid::new_v4()),
            workflow_kind: "video_provider_job".into(),
            project_id: None,
            root_project_id: None,
            source_snapshot_id: None,
            scope_table: None,
            scope_id: None,
            chapter_id: None,
            mode: None,
            input_fingerprint_json: None,
            status: WORKFLOW_STATUS_COMPLETED.into(),
            current_step_key: None,
            active_task_id: None,
            can_resume: false,
            resume_block_reason: None,
            error_message: None,
        },
    )
    .await
    .expect("workflow fixture must be created");

    let now = Utc::now().naive_utc();
    let payload = "large model output ".repeat(1_000);
    let mut rows = Vec::with_capacity(GENERATION_RUN_COUNT);
    for index in 0..GENERATION_RUN_COUNT {
        rows.push(generation_run::ActiveModel {
            project_id: Set(1),
            root_project_id: Set(1),
            workflow: Set("canvas_graph_execution".into()),
            step_key: Set(format!("node_{index}")),
            call_key: Set(Some(format!("call_{index}"))),
            title: Set(format!("Generation node {index}")),
            model_kind: Set("text".into()),
            provider: Set(Some("benchmark".into())),
            model: Set(Some("large-output".into())),
            status: Set(WORKFLOW_STATUS_COMPLETED.into()),
            workflow_run_id: Set(Some(workflow.id)),
            input_fingerprint_sha256: Set(Some(format!("fingerprint-{index}"))),
            input_fingerprint_json: Set(Some(json!({"input": payload, "index": index}))),
            raw_response_json: Set(Some(json!({"raw": payload, "index": index}))),
            parsed_output_json: Set(Some(json!({"parsed": payload, "index": index}))),
            validation_json: Set(Some(json!({"valid": true, "index": index}))),
            artifact_refs_json: Set(Some(json!({"artifact_id": index + 1}))),
            attempt: Set(1),
            response_summary_json: Set(Some(json!({"summary": "completed"}))),
            created_at: Set(now + chrono::Duration::milliseconds(index as i64)),
            completed_at: Set(Some(now)),
            ..Default::default()
        });
    }
    generation_run::Entity::insert_many(rows)
        .exec(db)
        .await
        .expect("generation run fixtures must be inserted");
    workflow.id
}

async fn full_projection(
    db: &sea_orm::DatabaseConnection,
    workflow_run_id: i64,
) -> Vec<FullGenerationRunProjection> {
    generation_run::Entity::find()
        .select_only()
        .column(generation_run::Column::Id)
        .column(generation_run::Column::WorkflowRunId)
        .column(generation_run::Column::Workflow)
        .column(generation_run::Column::StepKey)
        .column(generation_run::Column::CallKey)
        .column(generation_run::Column::Title)
        .column(generation_run::Column::ModelKind)
        .column(generation_run::Column::Provider)
        .column(generation_run::Column::Model)
        .column(generation_run::Column::Status)
        .column(generation_run::Column::InputFingerprintSha256)
        .column(generation_run::Column::InputFingerprintJson)
        .column(generation_run::Column::RawResponseJson)
        .column(generation_run::Column::ParsedOutputJson)
        .column(generation_run::Column::ValidationJson)
        .column(generation_run::Column::ArtifactRefsJson)
        .column(generation_run::Column::Attempt)
        .column(generation_run::Column::ResponseSummaryJson)
        .column(generation_run::Column::ErrorMessage)
        .column(generation_run::Column::CreatedAt)
        .column(generation_run::Column::CompletedAt)
        .filter(generation_run::Column::WorkflowRunId.eq(workflow_run_id))
        .order_by_asc(generation_run::Column::CreatedAt)
        .limit(300)
        .into_model::<FullGenerationRunProjection>()
        .all(db)
        .await
        .expect("full generation run projection must succeed")
}

async fn summary_projection(
    db: &sea_orm::DatabaseConnection,
    workflow_run_id: i64,
) -> Vec<SummaryGenerationRunProjection> {
    generation_run::Entity::find()
        .select_only()
        .column(generation_run::Column::Id)
        .column(generation_run::Column::WorkflowRunId)
        .column(generation_run::Column::Workflow)
        .column(generation_run::Column::StepKey)
        .column(generation_run::Column::CallKey)
        .column(generation_run::Column::Title)
        .column(generation_run::Column::ModelKind)
        .column(generation_run::Column::Provider)
        .column(generation_run::Column::Model)
        .column(generation_run::Column::Status)
        .column(generation_run::Column::InputFingerprintSha256)
        .column(generation_run::Column::Attempt)
        .column(generation_run::Column::ResponseSummaryJson)
        .column(generation_run::Column::ErrorMessage)
        .column(generation_run::Column::CreatedAt)
        .column(generation_run::Column::CompletedAt)
        .column_as(
            Expr::cust("raw_response_json IS NOT NULL"),
            "has_raw_response",
        )
        .column_as(
            Expr::cust("parsed_output_json IS NOT NULL"),
            "has_parsed_output",
        )
        .column_as(Expr::cust("validation_json IS NOT NULL"), "has_validation")
        .filter(generation_run::Column::WorkflowRunId.eq(workflow_run_id))
        .order_by_asc(generation_run::Column::CreatedAt)
        .limit(300)
        .into_model::<SummaryGenerationRunProjection>()
        .all(db)
        .await
        .expect("summary generation run projection must succeed")
}

fn serialized_bytes<T: Serialize>(rows: &[T]) -> usize {
    rows.iter()
        .map(|row| {
            serde_json::to_vec(row)
                .expect("projection must serialize")
                .len()
        })
        .sum()
}

async fn workflow_detail(
    db: &sea_orm::DatabaseConnection,
    workflow_run_id: i64,
    include_payloads: Option<bool>,
) -> a4n_db::workflow::WorkflowRunDetailSummary {
    workflow_run_detail(
        db,
        GetWorkflowRunInput {
            workflow_run_id,
            include_generation_runs: true,
            include_generation_run_payloads: include_payloads,
        },
    )
    .await
    .expect("workflow detail query must succeed")
}

fn assert_light_detail_matches_full(
    full: &a4n_db::workflow::WorkflowRunDetailSummary,
    light: &a4n_db::workflow::WorkflowRunDetailSummary,
) {
    assert_eq!(full.run.id, light.run.id);
    assert_eq!(full.run.status, light.run.status);
    assert_eq!(full.checkpoints.len(), light.checkpoints.len());
    assert_eq!(full.generation_runs.len(), light.generation_runs.len());
    for (full, light) in full.generation_runs.iter().zip(&light.generation_runs) {
        assert_eq!(full.id, light.id);
        assert_eq!(full.workflow_run_id, light.workflow_run_id);
        assert_eq!(full.workflow, light.workflow);
        assert_eq!(full.step_key, light.step_key);
        assert_eq!(full.call_key, light.call_key);
        assert_eq!(full.title, light.title);
        assert_eq!(full.model_kind, light.model_kind);
        assert_eq!(full.provider, light.provider);
        assert_eq!(full.model, light.model);
        assert_eq!(full.status, light.status);
        assert_eq!(
            full.input_fingerprint_sha256,
            light.input_fingerprint_sha256
        );
        assert_eq!(full.attempt, light.attempt);
        assert_eq!(full.response_summary_json, light.response_summary_json);
        assert_eq!(full.error_message, light.error_message);
        assert_eq!(full.created_at, light.created_at);
        assert_eq!(full.completed_at, light.completed_at);
        assert!(full.input_fingerprint_json.is_some());
        assert!(full.raw_response_json.is_some());
        assert!(full.parsed_output_json.is_some());
        assert!(full.validation_json.is_some());
        assert!(full.artifact_refs_json.is_some());
        assert!(light.input_fingerprint_json.is_none());
        assert!(light.raw_response_json.is_none());
        assert!(light.parsed_output_json.is_none());
        assert!(light.validation_json.is_none());
        assert!(light.artifact_refs_json.is_none());
        assert_eq!(full.has_raw_response, light.has_raw_response);
        assert_eq!(full.has_parsed_output, light.has_parsed_output);
        assert_eq!(full.has_validation, light.has_validation);
    }
}

#[tokio::test]
#[ignore = "manual PostgreSQL performance benchmark; requires isolated A4N_TEST_DATABASE_URL"]
async fn benchmark_workflow_generation_detail_projection() {
    let db = a4n_migration::test_support::connect_test_database().await;
    let workflow_run_id = fixture(&db).await;
    let full = full_projection(&db, workflow_run_id).await;
    let summary = summary_projection(&db, workflow_run_id).await;
    assert_eq!(full.len(), GENERATION_RUN_COUNT);
    assert_eq!(summary.len(), GENERATION_RUN_COUNT);

    for (full, summary) in full.iter().zip(&summary) {
        assert_eq!(full.id, summary.id);
        assert_eq!(full.workflow_run_id, summary.workflow_run_id);
        assert_eq!(full.workflow, summary.workflow);
        assert_eq!(full.step_key, summary.step_key);
        assert_eq!(full.call_key, summary.call_key);
        assert_eq!(full.title, summary.title);
        assert_eq!(full.model_kind, summary.model_kind);
        assert_eq!(full.provider, summary.provider);
        assert_eq!(full.model, summary.model);
        assert_eq!(full.status, summary.status);
        assert_eq!(
            full.input_fingerprint_sha256,
            summary.input_fingerprint_sha256
        );
        assert_eq!(full.attempt, summary.attempt);
        assert_eq!(full.response_summary_json, summary.response_summary_json);
        assert_eq!(full.error_message, summary.error_message);
        assert_eq!(full.created_at, summary.created_at);
        assert_eq!(full.completed_at, summary.completed_at);
        assert_eq!(summary.has_raw_response, full.raw_response_json.is_some());
        assert_eq!(summary.has_parsed_output, full.parsed_output_json.is_some());
        assert_eq!(summary.has_validation, full.validation_json.is_some());
    }

    let mut full_samples = Vec::with_capacity(SAMPLES);
    let mut summary_samples = Vec::with_capacity(SAMPLES);
    for sample in 0..SAMPLES {
        if sample % 2 == 0 {
            let started = Instant::now();
            let full = full_projection(&db, workflow_run_id).await;
            full_samples.push(started.elapsed().as_secs_f64() * 1000.0);
            assert_eq!(full.len(), GENERATION_RUN_COUNT);

            let started = Instant::now();
            let summary = summary_projection(&db, workflow_run_id).await;
            summary_samples.push(started.elapsed().as_secs_f64() * 1000.0);
            assert_eq!(summary.len(), GENERATION_RUN_COUNT);
        } else {
            let started = Instant::now();
            let summary = summary_projection(&db, workflow_run_id).await;
            summary_samples.push(started.elapsed().as_secs_f64() * 1000.0);
            assert_eq!(summary.len(), GENERATION_RUN_COUNT);

            let started = Instant::now();
            let full = full_projection(&db, workflow_run_id).await;
            full_samples.push(started.elapsed().as_secs_f64() * 1000.0);
            assert_eq!(full.len(), GENERATION_RUN_COUNT);
        }
    }

    let full_p50 = percentile(&full_samples, 0.50);
    let full_p95 = percentile(&full_samples, 0.95);
    let summary_p50 = percentile(&summary_samples, 0.50);
    let summary_p95 = percentile(&summary_samples, 0.95);
    let full_detail = workflow_detail(&db, workflow_run_id, None).await;
    let light_detail = workflow_detail(&db, workflow_run_id, Some(false)).await;
    assert_light_detail_matches_full(&full_detail, &light_detail);
    let full_detail_bytes = serde_json::to_vec(&full_detail)
        .expect("full detail must serialize")
        .len();
    let light_detail_bytes = serde_json::to_vec(&light_detail)
        .expect("light detail must serialize")
        .len();
    let mut full_detail_samples = Vec::with_capacity(SAMPLES);
    let mut light_detail_samples = Vec::with_capacity(SAMPLES);
    for sample in 0..SAMPLES {
        if sample % 2 == 0 {
            let started = Instant::now();
            let full = workflow_detail(&db, workflow_run_id, None).await;
            full_detail_samples.push(started.elapsed().as_secs_f64() * 1000.0);
            assert_light_detail_matches_full(&full, &light_detail);

            let started = Instant::now();
            let light = workflow_detail(&db, workflow_run_id, Some(false)).await;
            light_detail_samples.push(started.elapsed().as_secs_f64() * 1000.0);
            assert_light_detail_matches_full(&full_detail, &light);
        } else {
            let started = Instant::now();
            let light = workflow_detail(&db, workflow_run_id, Some(false)).await;
            light_detail_samples.push(started.elapsed().as_secs_f64() * 1000.0);
            assert_light_detail_matches_full(&full_detail, &light);

            let started = Instant::now();
            let full = workflow_detail(&db, workflow_run_id, None).await;
            full_detail_samples.push(started.elapsed().as_secs_f64() * 1000.0);
            assert_light_detail_matches_full(&full, &light_detail);
        }
    }
    let full_detail_p50 = percentile(&full_detail_samples, 0.50);
    let full_detail_p95 = percentile(&full_detail_samples, 0.95);
    let light_detail_p50 = percentile(&light_detail_samples, 0.50);
    let light_detail_p95 = percentile(&light_detail_samples, 0.95);
    eprintln!(
        "workflow_generation_detail_projection runs={GENERATION_RUN_COUNT} full_bytes={} summary_bytes={} full_p50_p95_ms={full_p50:.3}/{full_p95:.3} summary_p50_p95_ms={summary_p50:.3}/{summary_p95:.3} improvement={:.1}%/{:.1}% workflow_detail_full_bytes={full_detail_bytes} workflow_detail_light_bytes={light_detail_bytes} workflow_detail_full_p50_p95_ms={full_detail_p50:.3}/{full_detail_p95:.3} workflow_detail_light_p50_p95_ms={light_detail_p50:.3}/{light_detail_p95:.3} workflow_detail_improvement={:.1}%/{:.1}%",
        serialized_bytes(&full),
        serialized_bytes(&summary),
        (1.0 - summary_p50 / full_p50) * 100.0,
        (1.0 - summary_p95 / full_p95) * 100.0,
        (1.0 - light_detail_p50 / full_detail_p50) * 100.0,
        (1.0 - light_detail_p95 / full_detail_p95) * 100.0,
    );
}
