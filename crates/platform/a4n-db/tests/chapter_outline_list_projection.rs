use a4n_db::project::{
    CreateProjectRecordInput, chapter_outlines_for_project, create_project_record,
};
use a4n_db_models::models::chapter::{chapter_outline, chapter_outline_provenance};
use a4n_domain::chapter::ChapterOutlineRecord;
use chrono::Utc;
use sea_orm::{ColumnTrait, ConnectionTrait, DatabaseConnection, EntityTrait, QueryFilter, Set};
use serde::Serialize;
use serde_json::{Value, json};
use std::collections::HashMap;
use std::time::Instant;

#[derive(Clone, Debug, Serialize, PartialEq)]
struct OutlineListEntry {
    id: i64,
    project_id: i64,
    chapter_number: i32,
    title: Option<String>,
    summary: Option<String>,
    word_count_target: Option<i32>,
    source_kind: Option<String>,
    updated_at: Option<String>,
}

#[derive(Serialize)]
struct FullOutlineSummary {
    id: i64,
    project_id: i64,
    part_id: Option<i64>,
    chapter_number: i32,
    title: Option<String>,
    summary: Option<String>,
    beats: Option<Value>,
    key_scenes: Option<Value>,
    pacing_score: Option<f32>,
    foreshadowing_setup: Option<Value>,
    foreshadowing_payoff: Option<Value>,
    characters_involved: Option<Value>,
    word_count_target: Option<i32>,
    status: String,
    source_kind: Option<String>,
    created_at: Option<String>,
    updated_at: Option<String>,
}

fn full_summary(
    outline: &ChapterOutlineRecord,
    source_kind: Option<&String>,
) -> FullOutlineSummary {
    FullOutlineSummary {
        id: outline.id,
        project_id: outline.project_id,
        part_id: outline.part_id,
        chapter_number: outline.chapter_number,
        title: outline.title.clone(),
        summary: outline.summary.clone(),
        beats: outline.beats.clone(),
        key_scenes: outline.key_scenes.clone(),
        pacing_score: outline.pacing_score,
        foreshadowing_setup: outline.foreshadowing_setup.clone(),
        foreshadowing_payoff: outline.foreshadowing_payoff.clone(),
        characters_involved: outline.characters_involved.clone(),
        word_count_target: outline.word_count_target,
        status: outline.status.clone(),
        source_kind: source_kind.cloned(),
        created_at: Some(outline.created_at.to_string()),
        updated_at: Some(outline.updated_at.to_string()),
    }
}

fn list_entry(outline: &ChapterOutlineRecord, source_kind: Option<&String>) -> OutlineListEntry {
    OutlineListEntry {
        id: outline.id,
        project_id: outline.project_id,
        chapter_number: outline.chapter_number,
        title: outline.title.clone(),
        summary: outline.summary.clone(),
        word_count_target: outline.word_count_target,
        source_kind: source_kind.cloned(),
        updated_at: Some(outline.updated_at.to_string()),
    }
}

async fn create_fixture(db: &DatabaseConnection) -> i64 {
    let project = create_project_record(
        db,
        CreateProjectRecordInput {
            user_id: 1,
            title: "章节大纲列表性能".into(),
            description: String::new(),
            project_type: "novel_long".into(),
        },
    )
    .await
    .expect("benchmark project must be created");
    let payload = "章节规划与人物线索。".repeat(128);
    let now = Utc::now().naive_utc();
    let outlines = (0..1_000_i32).map(|offset| {
        let number = offset + 1;
        chapter_outline::ActiveModel {
            project_id: Set(project.id),
            chapter_number: Set(number),
            title: Set(Some(format!("第 {number} 章"))),
            summary: Set(Some(format!("第 {number} 章的推进摘要"))),
            beats: Set(Some(json!({"items": [payload.clone(), payload.clone()]}))),
            key_scenes: Set(Some(json!({"items": [payload.clone(), payload.clone()]}))),
            foreshadowing_setup: Set(Some(json!({"items": [payload.clone()]}))),
            foreshadowing_payoff: Set(Some(json!({"items": [payload.clone()]}))),
            characters_involved: Set(Some(json!([payload.clone()]))),
            word_count_target: Set(Some(1_500)),
            status: Set(chapter_outline::STATUS_COMPLETED.into()),
            created_at: Set(now),
            updated_at: Set(now),
            ..Default::default()
        }
    });
    for chunk in outlines.collect::<Vec<_>>().chunks(250) {
        chapter_outline::Entity::insert_many(chunk.iter().cloned())
            .exec(db)
            .await
            .expect("benchmark outlines must be inserted");
    }
    let provenance = (1..=1_000_i64)
        .step_by(4)
        .map(|id| chapter_outline_provenance::ActiveModel {
            outline_id: Set(id),
            source_kind: Set("retrospective_analysis".into()),
            source_fingerprint_json: Set(json!({"source": payload.clone()})),
            created_at: Set(now),
            updated_at: Set(now),
        })
        .collect::<Vec<_>>();
    chapter_outline_provenance::Entity::insert_many(provenance)
        .exec(db)
        .await
        .expect("benchmark provenance must be inserted");
    db.execute_unprepared("ANALYZE chapter_outline")
        .await
        .expect("outline statistics must be refreshed");
    db.execute_unprepared("ANALYZE chapter_outline_provenance")
        .await
        .expect("provenance statistics must be refreshed");
    project.id
}

async fn full_rows(
    db: &DatabaseConnection,
    project_id: i64,
) -> (Vec<ChapterOutlineRecord>, HashMap<i64, String>) {
    let outlines = chapter_outlines_for_project(db, project_id)
        .await
        .expect("full chapter outlines must load");
    let ids = outlines
        .iter()
        .map(|outline| outline.id)
        .collect::<Vec<_>>();
    let source_kinds = chapter_outline_provenance::Entity::find()
        .filter(chapter_outline_provenance::Column::OutlineId.is_in(ids))
        .all(db)
        .await
        .expect("full provenance rows must load")
        .into_iter()
        .map(|row| (row.outline_id, row.source_kind))
        .collect();
    (outlines, source_kinds)
}

async fn index_entries(db: &DatabaseConnection, project_id: i64) -> Vec<OutlineListEntry> {
    a4n_db::project::chapter_outline_index_for_project(db, project_id)
        .await
        .expect("narrow chapter outline index must load")
        .into_iter()
        .map(|outline| OutlineListEntry {
            id: outline.id,
            project_id: outline.project_id,
            chapter_number: outline.chapter_number,
            title: outline.title,
            summary: outline.summary,
            word_count_target: outline.word_count_target,
            source_kind: outline.source_kind,
            updated_at: Some(outline.updated_at.to_string()),
        })
        .collect()
}

fn percentile(samples: &[f64], percentile: f64) -> f64 {
    let mut sorted = samples.to_vec();
    sorted.sort_by(f64::total_cmp);
    let index = ((sorted.len() - 1) as f64 * percentile).round() as usize;
    sorted[index]
}

#[tokio::test]
#[ignore = "manual PostgreSQL performance benchmark; requires isolated A4N_TEST_DATABASE_URL"]
async fn benchmark_chapter_outline_list_projection() {
    let db = a4n_migration::test_support::connect_test_database().await;
    let project_id = create_fixture(&db).await;
    let (full_outlines, full_source_kinds) = full_rows(&db, project_id).await;
    let expected_index = full_outlines
        .iter()
        .map(|outline| list_entry(outline, full_source_kinds.get(&outline.id)))
        .collect::<Vec<_>>();
    let actual_index = index_entries(&db, project_id).await;
    assert_eq!(actual_index, expected_index);
    assert_eq!(full_outlines.len(), 1_000);

    let full_payload_bytes = serde_json::to_vec(
        &full_outlines
            .iter()
            .map(|outline| full_summary(outline, full_source_kinds.get(&outline.id)))
            .collect::<Vec<_>>(),
    )
    .expect("full outline response must serialize")
    .len();
    let index_payload_bytes = serde_json::to_vec(&actual_index)
        .expect("outline index response must serialize")
        .len();

    for _ in 0..2 {
        let (rows, sources) = full_rows(&db, project_id).await;
        let _ = serde_json::to_vec(
            &rows
                .iter()
                .map(|outline| full_summary(outline, sources.get(&outline.id)))
                .collect::<Vec<_>>(),
        )
        .expect("full warmup must serialize");
        let _ = index_entries(&db, project_id).await;
    }
    let mut full_samples = Vec::with_capacity(15);
    let mut index_samples = Vec::with_capacity(15);
    for _ in 0..15 {
        let started = Instant::now();
        let (rows, sources) = full_rows(&db, project_id).await;
        let bytes = serde_json::to_vec(
            &rows
                .iter()
                .map(|outline| full_summary(outline, sources.get(&outline.id)))
                .collect::<Vec<_>>(),
        )
        .expect("full sample must serialize");
        full_samples.push(started.elapsed().as_secs_f64() * 1_000.0);
        assert_eq!(bytes.len(), full_payload_bytes);

        let started = Instant::now();
        let entries = index_entries(&db, project_id).await;
        let bytes = serde_json::to_vec(&entries).expect("index sample must serialize");
        index_samples.push(started.elapsed().as_secs_f64() * 1_000.0);
        assert_eq!(entries, expected_index);
        assert_eq!(bytes.len(), index_payload_bytes);
    }
    println!(
        "{{\"outlines\":1000,\"full_bytes\":{full_payload_bytes},\"index_bytes\":{index_payload_bytes},\"full_p50_ms\":{:.3},\"full_p95_ms\":{:.3},\"index_p50_ms\":{:.3},\"index_p95_ms\":{:.3}}}",
        percentile(&full_samples, 0.50),
        percentile(&full_samples, 0.95),
        percentile(&index_samples, 0.50),
        percentile(&index_samples, 0.95),
    );
}
