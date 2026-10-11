use a4n_db::character::{
    create_profile, profile_current_state_fields, replay_profile_state_through_chapter,
};
use a4n_db::fact_feedback::{
    FactChangeRecordInput, profile_enrichment_before_chapter, record_fact_change,
};
use a4n_db::generation::record_completed_generation_run;
use a4n_db::project::{
    CreateProjectRecordInput, chapter_generation_reference_context_rows, create_project_record,
};
use a4n_db::text_anchor::{
    EVIDENCE_ROLE_FIRST_SEEN, earliest_current_profile_evidence_chapter,
    merge_profile_evidence_records_batch, rebuild_chapter_content_anchor_records,
};
use a4n_db_models::models::{
    chapter::chapter,
    character::{attribute_event, protagonist_deletion_mark, protagonist_profile},
    project::{blueprint_character, blueprint_relationship, novel_blueprint},
};
use a4n_domain::fact_feedback::FactSourceRevision;
use a4n_domain::generation::{GenerationRunTraceTarget, RecordCompletedGenerationRunInput};
use a4n_domain::text_anchor::{TextAnchorEvidenceInput, TextAnchorProfileKind};
use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder, Set,
};
use serde_json::json;
use std::time::Instant;

const PROFILE_COUNTS: &[usize] = &[16, 32, 64];
const SAMPLES: usize = 15;

async fn fixture(db: &DatabaseConnection, profile_count: usize) -> (i64, Vec<String>) {
    let project = create_project_record(
        db,
        CreateProjectRecordInput {
            user_id: 1,
            title: format!("chapter reference context {}", uuid::Uuid::new_v4()),
            description: String::new(),
            project_type: "novel_long".to_string(),
        },
    )
    .await
    .expect("benchmark project must be created");

    let now = Utc::now().naive_utc();
    let chapter = chapter::ActiveModel {
        project_id: Set(project.id),
        chapter_number: Set(1),
        title: Set(Some("第一章".to_string())),
        content: Set("人物在旧城出现并离开。".to_string()),
        word_count: Set(12),
        status: Set(chapter::STATUS_DRAFT.to_string()),
        summary: Set(None),
        style_tags: Set(None),
        analysis_data: Set(None),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(db)
    .await
    .expect("benchmark chapter must be created");
    let anchors = rebuild_chapter_content_anchor_records(db, project.id, chapter.id)
        .await
        .expect("benchmark anchors must be created");
    let anchor = anchors.first().expect("chapter must produce an anchor");

    let mut profiles = Vec::with_capacity(profile_count);
    let mut names = Vec::with_capacity(profile_count);
    for index in 0..profile_count {
        let name = format!("character-{index:04}");
        names.push(name.clone());
        let profile = create_profile(
            db,
            project.id,
            &name,
            json!({
                "explicit_attributes": {
                    "first_seen_chapter": 1,
                    "appearance": format!("appearance-{index}"),
                    "background": format!("background-{index}")
                },
                "implicit_attributes": {},
                "social_attributes": {}
            }),
        )
        .await
        .expect("benchmark profile must be created");
        profiles.push(profile);
    }

    record_completed_generation_run(
        db,
        RecordCompletedGenerationRunInput {
            project_id: project.id,
            workflow: "character_discovery".to_string(),
            step_key: "character_discovery.llm".to_string(),
            title: "角色发现基准".to_string(),
            model_kind: "llm".to_string(),
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
            targets: profiles
                .iter()
                .map(|profile| GenerationRunTraceTarget {
                    target_kind: "character_profile_record".to_string(),
                    target_table: "protagonist_profile".to_string(),
                    target_id: profile.id,
                })
                .collect(),
        },
    )
    .await
    .expect("benchmark discovery run must be recorded");

    merge_profile_evidence_records_batch(
        db,
        project.id,
        None,
        TextAnchorProfileKind::Character,
        profiles
            .iter()
            .map(|profile| {
                (
                    profile.id,
                    vec![TextAnchorEvidenceInput {
                        text_anchor_id: anchor.id,
                        evidence_role: EVIDENCE_ROLE_FIRST_SEEN.to_string(),
                    }],
                )
            })
            .collect(),
    )
    .await
    .expect("benchmark profile evidence must be recorded");

    (project.id, names)
}

fn percentile(samples: &[f64], percentile: f64) -> f64 {
    let mut sorted = samples.to_vec();
    sorted.sort_by(f64::total_cmp);
    sorted[((sorted.len() - 1) as f64 * percentile).round() as usize]
}

#[tokio::test]
#[ignore = "manual PostgreSQL performance benchmark; requires isolated A4N_TEST_DATABASE_URL"]
async fn benchmark_chapter_generation_reference_context_profiles() {
    let db = a4n_migration::test_support::connect_test_database().await;
    for &profile_count in PROFILE_COUNTS {
        let (project_id, names) = fixture(&db, profile_count).await;
        let mut legacy_samples = Vec::with_capacity(SAMPLES);
        let mut batched_samples = Vec::with_capacity(SAMPLES);
        let mut legacy_bytes = Vec::new();
        let mut batched_bytes = Vec::new();
        for sample in 0..SAMPLES + 2 {
            let (legacy_elapsed, legacy_response, batched_elapsed, batched_response, batched_names) =
                if sample % 2 == 0 {
                    let (legacy_elapsed, legacy_response) =
                        measure_legacy_profile_projection(&db, project_id, 20).await;
                    let (batched_elapsed, batched_response, batched_names) =
                        measure_batched_profile_projection(&db, project_id, 20).await;
                    (
                        legacy_elapsed,
                        legacy_response,
                        batched_elapsed,
                        batched_response,
                        batched_names,
                    )
                } else {
                    let (batched_elapsed, batched_response, batched_names) =
                        measure_batched_profile_projection(&db, project_id, 20).await;
                    let (legacy_elapsed, legacy_response) =
                        measure_legacy_profile_projection(&db, project_id, 20).await;
                    (
                        legacy_elapsed,
                        legacy_response,
                        batched_elapsed,
                        batched_response,
                        batched_names,
                    )
                };
            assert_eq!(batched_names.len(), profile_count);
            assert_eq!(batched_names, names);
            assert_eq!(legacy_response, batched_response);
            legacy_bytes = legacy_response;
            batched_bytes = batched_response;
            if sample >= 2 {
                legacy_samples.push(legacy_elapsed);
                batched_samples.push(batched_elapsed);
            }
        }
        let legacy_p50 = percentile(&legacy_samples, 0.50);
        let legacy_p95 = percentile(&legacy_samples, 0.95);
        let batched_p50 = percentile(&batched_samples, 0.50);
        let batched_p95 = percentile(&batched_samples, 0.95);
        eprintln!(
            "chapter_generation_reference_context profiles={profile_count} response_bytes={} legacy_p50_p95_ms={legacy_p50:.3}/{legacy_p95:.3} batched_p50_p95_ms={batched_p50:.3}/{batched_p95:.3} p50_improvement={:.1}%",
            batched_bytes.len(),
            (1.0 - batched_p50 / legacy_p50) * 100.0,
        );
        assert_eq!(legacy_bytes.len(), batched_bytes.len());
    }
}

async fn measure_legacy_profile_projection(
    db: &DatabaseConnection,
    project_id: i64,
    chapter_number: i32,
) -> (f64, Vec<u8>) {
    let started = Instant::now();
    let profiles = legacy_profile_projection(db, project_id, chapter_number).await;
    let response = serde_json::to_vec(&profiles).expect("legacy profiles must serialize");
    (started.elapsed().as_secs_f64() * 1000.0, response)
}

async fn measure_batched_profile_projection(
    db: &DatabaseConnection,
    project_id: i64,
    chapter_number: i32,
) -> (f64, Vec<u8>, Vec<String>) {
    let started = Instant::now();
    let rows = chapter_generation_reference_context_rows(db, project_id, chapter_number)
        .await
        .expect("reference context must load");
    let response =
        serde_json::to_vec(&rows.protagonist_profiles).expect("reference profiles must serialize");
    let names = rows
        .protagonist_profiles
        .into_iter()
        .map(|profile| profile.name)
        .collect();
    (started.elapsed().as_secs_f64() * 1000.0, response, names)
}

async fn legacy_profile_projection(
    db: &DatabaseConnection,
    project_id: i64,
    chapter_number: i32,
) -> Vec<protagonist_profile::Model> {
    let (blueprint, characters, relationships, profiles) = futures::join!(
        novel_blueprint::Entity::find()
            .filter(novel_blueprint::Column::ProjectId.eq(project_id))
            .one(db),
        blueprint_character::Entity::find()
            .filter(blueprint_character::Column::ProjectId.eq(project_id))
            .order_by_asc(blueprint_character::Column::Id)
            .all(db),
        blueprint_relationship::Entity::find()
            .filter(blueprint_relationship::Column::ProjectId.eq(project_id))
            .order_by_desc(blueprint_relationship::Column::Strength)
            .order_by_asc(blueprint_relationship::Column::Id)
            .all(db),
        protagonist_profile::Entity::find()
            .filter(protagonist_profile::Column::ProjectId.eq(project_id))
            .order_by_asc(protagonist_profile::Column::Id)
            .all(db),
    );
    assert!(blueprint.expect("blueprint query must succeed").is_none());
    characters.expect("character query must succeed");
    relationships.expect("relationship query must succeed");
    let profiles = profiles.expect("profiles must load");
    let mut projected = Vec::new();
    for mut profile in profiles {
        if profile
            .first_seen_chapter()
            .is_some_and(|first| first >= chapter_number)
        {
            continue;
        }
        if profile.is_text_discovered()
            && earliest_current_profile_evidence_chapter(
                db,
                project_id,
                TextAnchorProfileKind::Character,
                profile.id,
            )
            .await
            .expect("profile evidence must load")
            .is_none_or(|first| first >= chapter_number)
        {
            continue;
        }
        let state = replay_profile_state_through_chapter(db, &profile, chapter_number - 1)
            .await
            .expect("profile state must replay");
        let fields = profile_current_state_fields(state);
        profile.gender = fields.gender;
        profile.age = fields.age;
        profile.appearance = fields.appearance;
        profile.background = fields.background;
        profile.personality = fields.personality;
        profile.current_state = fields.current_state;
        profile_enrichment_before_chapter(db, &mut profile, chapter_number)
            .await
            .expect("profile enrichment must project");
        profile.last_synced_chapter = profile.last_synced_chapter.min(chapter_number - 1);
        projected.push(profile);
    }
    projected
}

#[tokio::test]
async fn chapter_generation_reference_context_preserves_profile_projection() {
    let db = a4n_migration::test_support::connect_test_database().await;
    let (project_id, _) = fixture(&db, 4).await;
    let mut profiles = protagonist_profile::Entity::find()
        .filter(protagonist_profile::Column::ProjectId.eq(project_id))
        .order_by_asc(protagonist_profile::Column::Id)
        .all(&db)
        .await
        .expect("profiles must load");
    let first_profile = profiles.remove(0);
    let now = Utc::now().naive_utc();
    for (chapter_number, operation, value) in [
        (2, "add", "first location"),
        (4, "modify", "later location"),
    ] {
        attribute_event::ActiveModel {
            profile_id: Set(first_profile.id),
            chapter_number: Set(chapter_number),
            source_kind: Set(attribute_event::SOURCE_KIND_AI_SYNC.to_string()),
            timestamp: Set(now),
            category: Set("explicit".to_string()),
            key: Set("location".to_string()),
            operation: Set(operation.to_string()),
            change_description: Set("chapter profile state".to_string()),
            old_value: Set(serde_json::Value::Null),
            new_value: Set(json!(value)),
            cause: Set("benchmark fixture".to_string()),
            evidence: Set("chapter content".to_string()),
            ..Default::default()
        }
        .insert(&db)
        .await
        .expect("timeline event must be inserted");
    }
    protagonist_deletion_mark::ActiveModel {
        profile_id: Set(first_profile.id),
        source_kind: Set("user".to_string()),
        category: Set("explicit".to_string()),
        key: Set("location".to_string()),
        chapter_number: Set(3),
        reason: Set("manual history owns location".to_string()),
        evidence: Set("author edit".to_string()),
        consecutive_count: Set(1),
        is_executed: Set(false),
        last_marked_chapter: Set(3),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(&db)
    .await
    .expect("deletion mark must be inserted");

    let chapter = chapter::Entity::find()
        .filter(chapter::Column::ProjectId.eq(project_id))
        .one(&db)
        .await
        .expect("chapter lookup must succeed")
        .expect("fixture chapter must exist");
    let mut voice_profile: protagonist_profile::ActiveModel = first_profile.clone().into();
    voice_profile.voice_fingerprint = Set(Some(json!({"tone": "latest"})));
    voice_profile
        .update(&db)
        .await
        .expect("voice fingerprint must be updated");
    record_fact_change(
        &db,
        FactChangeRecordInput {
            project_id,
            workflow_run_id: None,
            material_kind: "character_voice".to_string(),
            target_id: Some(first_profile.id),
            field_path: String::new(),
            operation: "replace".to_string(),
            before: json!({"tone": "original"}),
            after: json!({"tone": "latest"}),
            sources: vec![FactSourceRevision {
                chapter_id: chapter.id,
                chapter_number: 30,
                content_revision: "chapter-30".to_string(),
                analysis_hash: "analysis-30".to_string(),
            }],
            evidence: json!([]),
            reason: "future chapter voice change".to_string(),
            status: "applied".to_string(),
        },
    )
    .await
    .expect("voice fact change must be recorded");
    create_profile(
        &db,
        project_id,
        "future-character",
        json!({
            "explicit_attributes": {"first_seen_chapter": 25},
            "implicit_attributes": {},
            "social_attributes": {}
        }),
    )
    .await
    .expect("future profile must be created");

    let expected = legacy_profile_projection(&db, project_id, 20).await;
    let rows = chapter_generation_reference_context_rows(&db, project_id, 20)
        .await
        .expect("chapter reference context must load");
    let expected_bytes = serde_json::to_vec(&expected).expect("legacy profiles must serialize");
    let actual_bytes =
        serde_json::to_vec(&rows.protagonist_profiles).expect("profile records must serialize");
    assert_eq!(actual_bytes, expected_bytes);
    assert_eq!(rows.protagonist_profiles.len(), 4);
    assert_eq!(
        rows.protagonist_profiles[0].current_state["explicit_attributes"]["location"],
        "first location"
    );
    assert_eq!(
        rows.protagonist_profiles[0].voice_fingerprint,
        Some(json!({"tone": "original"}))
    );
}
