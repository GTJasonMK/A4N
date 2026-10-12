use a4n_db::asset::{object_profile_list_records_by_project, object_profile_records_by_project};
use a4n_db::collaboration::{ProjectAccessRole, verify_project_permission};
use a4n_db::profile_material::{PROFILE_KIND_OBJECT, profile_material_image_counts_by_profile};
use a4n_db::project::{CreateProjectRecordInput, create_project_record};
use a4n_db_models::models::{ai::generated_image, asset::object_profile};
use a4n_domain::image::IMAGE_TYPE_OBJECT_MATERIAL;
use chrono::Utc;
use sea_orm::{
    ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, QueryOrder, QuerySelect, Set,
};
use serde::Serialize;
use serde_json::{Value, json};
use std::time::Instant;

const PROFILE_COUNT: usize = 1_000;
const TEXT_BYTES_PER_FIELD: usize = 2 * 1024;
const EVIDENCE_BYTES: usize = 512;
const WARMUPS: usize = 2;
const SAMPLES: usize = 15;

#[derive(Debug, Serialize, PartialEq)]
struct ObjectProfileListIndex {
    id: i64,
    project_id: i64,
    ownership_kind: String,
    name: String,
    object_type: String,
    importance: String,
    visual_description: String,
    story_usage: String,
    location_context: Option<String>,
    first_seen_chapter: i32,
    last_seen_chapter: i32,
    evidence_present: bool,
    generated_image_count: i64,
    created_at: Option<String>,
    updated_at: Option<String>,
}

#[derive(Debug, Serialize, PartialEq)]
struct FullProfilePayload {
    id: i64,
    project_id: i64,
    ownership_kind: String,
    name: String,
    normalized_name: String,
    aliases_json: Value,
    object_type: String,
    importance: String,
    visual_description: String,
    material_and_state: String,
    story_usage: String,
    visual_usage: String,
    generation_usage: String,
    location_context: Option<String>,
    first_seen_chapter: i32,
    last_seen_chapter: i32,
    evidence_json: Value,
    generated_image_count: i64,
    image_prompt_seed: String,
    created_at: Option<String>,
    updated_at: Option<String>,
}

async fn fixture(db: &sea_orm::DatabaseConnection) -> (i64, i64) {
    let user_id = a4n_db::auth::create_user_summary(
        db,
        "object-profile-benchmark",
        "object-profile-benchmark@example.test",
        "benchmark-password",
        "user",
    )
    .await
    .expect("benchmark user must be created")
    .id;
    let project = create_project_record(
        db,
        CreateProjectRecordInput {
            user_id,
            title: "object profile list projection".into(),
            description: String::new(),
            project_type: "novel_long".into(),
        },
    )
    .await
    .expect("benchmark project must be created");
    let now = Utc::now().naive_utc();
    let text = "x".repeat(TEXT_BYTES_PER_FIELD);
    let evidence = "e".repeat(EVIDENCE_BYTES);
    let rows = (0..PROFILE_COUNT)
        .map(|index| object_profile::ActiveModel {
            project_id: Set(project.id),
            ownership_kind: Set("ai_sync".into()),
            name: Set(format!("Object {index:04}")),
            normalized_name: Set(format!("object-{index:04}")),
            aliases_json: Set(json!([format!("alias-{index}")])),
            object_type: Set("prop".into()),
            importance: Set(if index % 5 == 0 {
                "primary"
            } else {
                "secondary"
            }
            .into()),
            visual_description: Set(text.clone()),
            material_and_state: Set(text.clone()),
            story_usage: Set(text.clone()),
            visual_usage: Set(text.clone()),
            generation_usage: Set(text.clone()),
            location_context: Set(Some(text.clone())),
            first_seen_chapter: Set((index % 100 + 1) as i32),
            last_seen_chapter: Set((index % 100 + 1) as i32),
            evidence_json: Set(json!([evidence.clone()])),
            image_prompt_seed: Set(text.clone()),
            created_at: Set(now + chrono::Duration::seconds(index as i64)),
            updated_at: Set(now + chrono::Duration::seconds(index as i64)),
            ..Default::default()
        })
        .collect::<Vec<_>>();
    for chunk in rows.chunks(250) {
        object_profile::Entity::insert_many(chunk.iter().cloned())
            .exec(db)
            .await
            .expect("object profile fixture must be inserted");
    }
    let profile_ids = object_profile::Entity::find()
        .select_only()
        .column(object_profile::Column::Id)
        .filter(object_profile::Column::ProjectId.eq(project.id))
        .order_by_asc(object_profile::Column::Id)
        .into_tuple::<i64>()
        .all(db)
        .await
        .expect("object profile ids must be loaded");
    let images = profile_ids
        .into_iter()
        .map(|profile_id| generated_image::ActiveModel {
            project_id: Set(project.id),
            image_type: Set(IMAGE_TYPE_OBJECT_MATERIAL.into()),
            file_path: Set(format!("images/object-profile-{profile_id}.png")),
            prompt: Set("object profile benchmark image".into()),
            model: Set("benchmark".into()),
            width: Set(1024),
            height: Set(1024),
            file_size: Set(Some(1024)),
            ai_score: Set(None),
            is_favorite: Set(false),
            chapter_id: Set(None),
            panel_id: Set(None),
            metadata_json: Set(Some(json!({
                "profile_id": profile_id,
                "profile_kind": PROFILE_KIND_OBJECT,
            }))),
            created_at: Set(now),
            ..Default::default()
        })
        .collect::<Vec<_>>();
    for chunk in images.chunks(250) {
        generated_image::Entity::insert_many(chunk.iter().cloned())
            .exec(db)
            .await
            .expect("object profile image fixture must be inserted");
    }
    db.execute_unprepared("ANALYZE object_profile")
        .await
        .expect("object profile statistics must be refreshed");
    db.execute_unprepared("ANALYZE generated_image")
        .await
        .expect("generated image statistics must be refreshed");
    (project.id, user_id)
}

async fn image_counts(
    db: &sea_orm::DatabaseConnection,
    project_id: i64,
    profile_ids: &[i64],
) -> std::collections::BTreeMap<i64, i64> {
    profile_material_image_counts_by_profile(
        db,
        project_id,
        IMAGE_TYPE_OBJECT_MATERIAL,
        profile_ids,
    )
    .await
    .expect("object profile image counts must load")
}

async fn full_response(
    db: &sea_orm::DatabaseConnection,
    project_id: i64,
    user_id: i64,
) -> (Vec<FullProfilePayload>, usize) {
    verify_project_permission(db, project_id, user_id, ProjectAccessRole::Viewer)
        .await
        .expect("full list permission must be granted");
    let profiles = object_profile_records_by_project(db, project_id)
        .await
        .expect("full object profile list must load");
    let profile_ids = profiles
        .iter()
        .map(|profile| profile.id)
        .collect::<Vec<_>>();
    let image_counts = image_counts(db, project_id, &profile_ids).await;
    let payload = profiles
        .iter()
        .map(|profile| FullProfilePayload {
            id: profile.id,
            project_id: profile.project_id,
            ownership_kind: profile.ownership_kind.clone(),
            name: profile.name.clone(),
            normalized_name: profile.normalized_name.clone(),
            aliases_json: profile.aliases_json.clone(),
            object_type: profile.object_type.clone(),
            importance: profile.importance.clone(),
            visual_description: profile.visual_description.clone(),
            material_and_state: profile.material_and_state.clone(),
            story_usage: profile.story_usage.clone(),
            visual_usage: profile.visual_usage.clone(),
            generation_usage: profile.generation_usage.clone(),
            location_context: profile.location_context.clone(),
            first_seen_chapter: profile.first_seen_chapter,
            last_seen_chapter: profile.last_seen_chapter,
            evidence_json: profile.evidence_json.clone(),
            generated_image_count: image_counts.get(&profile.id).copied().unwrap_or(0),
            image_prompt_seed: profile.image_prompt_seed.clone(),
            created_at: Some(profile.created_at.to_string()),
            updated_at: Some(profile.updated_at.to_string()),
        })
        .collect::<Vec<_>>();
    let bytes = serialized_bytes(&payload);
    (payload, bytes)
}

async fn index_response(
    db: &sea_orm::DatabaseConnection,
    project_id: i64,
    user_id: i64,
) -> (Vec<ObjectProfileListIndex>, usize) {
    verify_project_permission(db, project_id, user_id, ProjectAccessRole::Viewer)
        .await
        .expect("index list permission must be granted");
    let profiles = object_profile_list_records_by_project(db, project_id)
        .await
        .expect("object profile index must load");
    let profile_ids = profiles
        .iter()
        .map(|profile| profile.id)
        .collect::<Vec<_>>();
    let image_counts = image_counts(db, project_id, &profile_ids).await;
    let payload = profiles
        .into_iter()
        .map(|profile| ObjectProfileListIndex {
            id: profile.id,
            project_id: profile.project_id,
            ownership_kind: profile.ownership_kind,
            name: profile.name,
            object_type: profile.object_type,
            importance: profile.importance,
            visual_description: profile.visual_description,
            story_usage: profile.story_usage,
            location_context: profile.location_context,
            first_seen_chapter: profile.first_seen_chapter,
            last_seen_chapter: profile.last_seen_chapter,
            evidence_present: profile.evidence_present,
            generated_image_count: image_counts.get(&profile.id).copied().unwrap_or(0),
            created_at: Some(profile.created_at.to_string()),
            updated_at: Some(profile.updated_at.to_string()),
        })
        .collect::<Vec<_>>();
    let bytes = serialized_bytes(&payload);
    (payload, bytes)
}

fn percentile(samples: &[f64], percentile: f64) -> f64 {
    let mut sorted = samples.to_vec();
    sorted.sort_by(f64::total_cmp);
    sorted[((sorted.len() - 1) as f64 * percentile).round() as usize]
}

fn serialized_bytes<T: Serialize>(value: &T) -> usize {
    serde_json::to_vec(value)
        .expect("object profile projection must serialize")
        .len()
}

#[tokio::test]
#[ignore = "manual PostgreSQL performance benchmark; requires isolated A4N_TEST_DATABASE_URL"]
async fn benchmark_object_profile_list_projection() {
    let db = a4n_migration::test_support::connect_test_database().await;
    let (project_id, user_id) = fixture(&db).await;
    let (full, full_bytes) = full_response(&db, project_id, user_id).await;
    let (index, index_bytes) = index_response(&db, project_id, user_id).await;
    assert_eq!(full.len(), PROFILE_COUNT);
    assert_eq!(index.len(), PROFILE_COUNT);
    for (profile, item) in full.iter().zip(&index) {
        assert_eq!(profile.id, item.id);
        assert_eq!(profile.project_id, item.project_id);
        assert_eq!(profile.ownership_kind, item.ownership_kind);
        assert_eq!(profile.name, item.name);
        assert_eq!(profile.object_type, item.object_type);
        assert_eq!(profile.importance, item.importance);
        assert_eq!(profile.visual_description, item.visual_description);
        assert_eq!(profile.story_usage, item.story_usage);
        assert_eq!(profile.location_context, item.location_context);
        assert_eq!(profile.first_seen_chapter, item.first_seen_chapter);
        assert_eq!(profile.last_seen_chapter, item.last_seen_chapter);
        assert_eq!(profile.created_at, item.created_at);
        assert_eq!(profile.updated_at, item.updated_at);
        assert_eq!(profile.generated_image_count, 1);
        assert_eq!(profile.generated_image_count, item.generated_image_count);
        assert_eq!(
            profile
                .evidence_json
                .as_array()
                .is_some_and(|rows| !rows.is_empty()),
            item.evidence_present
        );
    }
    for _ in 0..WARMUPS {
        let _ = full_response(&db, project_id, user_id).await;
        let _ = index_response(&db, project_id, user_id).await;
    }
    let mut full_samples = Vec::with_capacity(SAMPLES);
    let mut index_samples = Vec::with_capacity(SAMPLES);
    for sample in 0..SAMPLES {
        if sample % 2 == 0 {
            let started = Instant::now();
            let (full, bytes) = full_response(&db, project_id, user_id).await;
            assert!(bytes > 0);
            full_samples.push(started.elapsed().as_secs_f64() * 1_000.0);
            assert_eq!(full.len(), PROFILE_COUNT);

            let started = Instant::now();
            let (index, bytes) = index_response(&db, project_id, user_id).await;
            assert!(bytes > 0);
            index_samples.push(started.elapsed().as_secs_f64() * 1_000.0);
            assert_eq!(index.len(), PROFILE_COUNT);
        } else {
            let started = Instant::now();
            let (index, bytes) = index_response(&db, project_id, user_id).await;
            assert!(bytes > 0);
            index_samples.push(started.elapsed().as_secs_f64() * 1_000.0);
            assert_eq!(index.len(), PROFILE_COUNT);

            let started = Instant::now();
            let (full, bytes) = full_response(&db, project_id, user_id).await;
            assert!(bytes > 0);
            full_samples.push(started.elapsed().as_secs_f64() * 1_000.0);
            assert_eq!(full.len(), PROFILE_COUNT);
        }
    }
    let full_p50 = percentile(&full_samples, 0.50);
    let full_p95 = percentile(&full_samples, 0.95);
    let index_p50 = percentile(&index_samples, 0.50);
    let index_p95 = percentile(&index_samples, 0.95);
    println!(
        "{}",
        json!({
            "profiles": PROFILE_COUNT,
            "associated_object_material_images": PROFILE_COUNT,
            "text_bytes_per_field": TEXT_BYTES_PER_FIELD,
            "full_response_bytes": full_bytes,
            "index_response_bytes": index_bytes,
            "full_p50_p95_ms": [full_p50, full_p95],
            "index_p50_p95_ms": [index_p50, index_p95],
            "improvement_percent": [(1.0 - index_p50 / full_p50) * 100.0, (1.0 - index_p95 / full_p95) * 100.0],
        })
    );
}
