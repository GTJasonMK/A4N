use a4n_db::asset::upsert_object_profiles_with_generation_run_record;
use a4n_db::project::{
    CreateProjectRecordInput, chapter_source_snapshot_from_model, create_project_record,
};
use a4n_db::text_anchor::rebuild_chapter_content_anchor_records;
use a4n_db_models::models::{
    asset::object_profile,
    chapter::chapter,
    project::{material_fact_change, profile_evidence},
};
use a4n_domain::chapter::ChapterSourceSnapshot;
use a4n_domain::generation::RecordCompletedGenerationRunInput;
use a4n_domain::object_profile::{ObjectProfileEvidenceRecordInput, ObjectProfileRecordInput};
use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, PaginatorTrait, QueryFilter,
    Set,
};
use serde_json::json;
use std::time::Instant;

const PROFILE_COUNTS: &[usize] = &[16, 32, 64, 128];
const SAMPLES: usize = 15;

fn generation_run(project_id: i64) -> RecordCompletedGenerationRunInput {
    RecordCompletedGenerationRunInput {
        project_id,
        workflow: "object_discovery".into(),
        step_key: "object_discovery.llm".into(),
        title: "物体档案发现".into(),
        model_kind: "llm".into(),
        provider: None,
        model: None,
        prompt_template_names_json: None,
        prompt_template_hashes_json: None,
        rendered_messages_json: None,
        rendered_prompts_json: None,
        input_variables_json: None,
        source_refs_json: None,
        media_refs_json: None,
        request_json: None,
        response_summary_json: None,
        targets: Vec::new(),
    }
}

fn profile_input(index: usize, quote: &str) -> ObjectProfileRecordInput {
    let name = format!("object-{index:04}");
    ObjectProfileRecordInput {
        name: name.clone(),
        normalized_name: name,
        aliases_json: json!([]),
        object_type: "prop".into(),
        importance: "primary".into(),
        visual_description: format!("visual {index}"),
        material_and_state: format!("state {index}"),
        story_usage: format!("usage {index}"),
        visual_usage: format!("visual usage {index}"),
        generation_usage: format!("generation usage {index}"),
        location_context: Some(format!("location {index}")),
        first_seen_chapter: 1,
        last_seen_chapter: 1,
        evidence_json: json!([quote]),
        image_prompt_seed: format!("prompt {index}"),
    }
}

async fn fixture(
    db: &DatabaseConnection,
    profile_count: usize,
) -> (
    i64,
    ChapterSourceSnapshot,
    i64,
    Vec<ObjectProfileRecordInput>,
    Vec<ObjectProfileEvidenceRecordInput>,
) {
    let project = create_project_record(
        db,
        CreateProjectRecordInput {
            user_id: 1,
            title: format!("object generation benchmark {profile_count}"),
            description: String::new(),
            project_type: "novel_long".into(),
        },
    )
    .await
    .expect("benchmark project must be created");
    let quote = (0..profile_count)
        .map(|index| format!("object-{index:04}"))
        .collect::<Vec<_>>()
        .join(" ");
    let now = Utc::now().naive_utc();
    let chapter = chapter::ActiveModel {
        project_id: Set(project.id),
        chapter_number: Set(1),
        title: Set(Some("第一章".into())),
        content: Set(quote.clone()),
        word_count: Set(quote.chars().count() as i32),
        status: Set(chapter::STATUS_DRAFT.into()),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(db)
    .await
    .expect("benchmark chapter must be created");
    let anchor = rebuild_chapter_content_anchor_records(db, project.id, chapter.id)
        .await
        .expect("benchmark anchor must be created")
        .into_iter()
        .next()
        .expect("benchmark chapter must have an anchor");
    let inputs = (0..profile_count)
        .map(|index| profile_input(index, &format!("object-{index:04}")))
        .collect::<Vec<_>>();
    let evidence = inputs
        .iter()
        .map(|input| ObjectProfileEvidenceRecordInput {
            normalized_name: input.normalized_name.clone(),
            text_anchor_ids: vec![anchor.id],
        })
        .collect::<Vec<_>>();
    (
        project.id,
        chapter_source_snapshot_from_model(&chapter),
        anchor.id,
        inputs,
        evidence,
    )
}

fn percentile(samples: &[f64], percentile: f64) -> f64 {
    let mut sorted = samples.to_vec();
    sorted.sort_by(f64::total_cmp);
    sorted[((sorted.len() - 1) as f64 * percentile).round() as usize]
}

#[tokio::test]
#[ignore = "manual PostgreSQL performance benchmark; requires isolated A4N_TEST_DATABASE_URL"]
async fn benchmark_object_profile_generation() {
    let db = a4n_migration::test_support::connect_test_database().await;
    for &profile_count in PROFILE_COUNTS {
        let mut samples = Vec::with_capacity(SAMPLES);
        for _ in 0..SAMPLES {
            let (project_id, source, _anchor_id, inputs, evidence) =
                fixture(&db, profile_count).await;
            let started = Instant::now();
            let result = upsert_object_profiles_with_generation_run_record(
                &db,
                project_id,
                inputs,
                vec![source],
                evidence,
                generation_run(project_id),
            )
            .await
            .expect("object generation persistence must succeed");
            assert_eq!(result.created.len(), profile_count);
            samples.push(started.elapsed().as_secs_f64() * 1000.0);
        }
        eprintln!(
            "object_profile_generation profiles={profile_count} p50_ms={:.3} p95_ms={:.3}",
            percentile(&samples, 0.50),
            percentile(&samples, 0.95),
        );
    }
}

#[tokio::test]
async fn first_observation_keeps_profile_projection_and_audit() {
    let db = a4n_migration::test_support::connect_test_database().await;
    let (project_id, source, _anchor_id, inputs, evidence) = fixture(&db, 1).await;
    let result = upsert_object_profiles_with_generation_run_record(
        &db,
        project_id,
        inputs,
        vec![source],
        evidence,
        generation_run(project_id),
    )
    .await
    .expect("object generation persistence must succeed");
    let profile_id = result.created[0].id;
    let stored = object_profile::Entity::find_by_id(profile_id)
        .one(&db)
        .await
        .expect("profile lookup must succeed")
        .expect("profile must remain");
    assert_eq!(
        stored.visual_description,
        result.created[0].visual_description
    );
    assert_eq!(
        stored.material_and_state,
        result.created[0].material_and_state
    );
    assert_eq!(
        material_fact_change::Entity::find()
            .filter(material_fact_change::Column::ProjectId.eq(project_id))
            .count(&db)
            .await
            .expect("fact changes must be countable"),
        1
    );
    assert_eq!(
        profile_evidence::Entity::find()
            .filter(profile_evidence::Column::ProfileId.eq(profile_id))
            .count(&db)
            .await
            .expect("profile evidence must be countable"),
        1
    );
}
