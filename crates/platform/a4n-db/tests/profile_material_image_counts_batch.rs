use a4n_db::profile_material::{
    profile_material_image_counts_by_profile, profile_material_image_identity_rows_by_type,
    profile_material_image_ids_by_profile,
};
use a4n_db::project::{CreateProjectRecordInput, create_project_record};
use a4n_db_models::models::ai::generated_image;
use chrono::Utc;
use sea_orm::{EntityTrait, Set};
use serde_json::{Value, json};
use std::collections::BTreeMap;
use std::time::Instant;

const IMAGE_COUNT: usize = 20_000;
const PROFILE_COUNT: i64 = 100;
const SAMPLES: usize = 9;

fn percentile(samples: &[f64], percentile: f64) -> f64 {
    let mut sorted = samples.to_vec();
    sorted.sort_by(f64::total_cmp);
    sorted[((sorted.len() - 1) as f64 * percentile).round() as usize]
}

async fn fixture(db: &sea_orm::DatabaseConnection) -> (i64, Vec<i64>) {
    let suffix = uuid::Uuid::new_v4().simple().to_string();
    let account = a4n_db::auth::create_user_summary(
        db,
        &format!("profile-counts-{suffix}"),
        &format!("profile-counts-{suffix}@example.invalid"),
        "profile-counts-password",
        "user",
    )
    .await
    .expect("benchmark user must be created");
    let project = create_project_record(
        db,
        CreateProjectRecordInput {
            user_id: account.id,
            title: format!("profile counts {suffix}"),
            description: String::new(),
            project_type: "novel_long".into(),
        },
    )
    .await
    .expect("benchmark project must be created");

    let mut rows = Vec::with_capacity(500);
    let payload = "metadata payload ".repeat(100);
    for index in 0..IMAGE_COUNT {
        let profile_id = (index as i64 % PROFILE_COUNT) + 1;
        rows.push(generated_image::ActiveModel {
            project_id: Set(project.id),
            image_type: Set("object_material".into()),
            file_path: Set(format!("benchmark/{index}.png")),
            prompt: Set("benchmark".into()),
            model: Set("test".into()),
            width: Set(1),
            height: Set(1),
            file_size: Set(Some(1)),
            ai_score: Set(None),
            is_favorite: Set(false),
            chapter_id: Set(None),
            panel_id: Set(None),
            metadata_json: Set(Some(json!({
                "profile_kind": "object",
                "profile_id": profile_id,
                "payload": payload,
            }))),
            created_at: Set(Utc::now().naive_utc()),
            ..Default::default()
        });
        if rows.len() == 500 {
            generated_image::Entity::insert_many(std::mem::take(&mut rows))
                .exec(db)
                .await
                .expect("benchmark images must be inserted");
        }
    }
    if !rows.is_empty() {
        generated_image::Entity::insert_many(rows)
            .exec(db)
            .await
            .expect("benchmark images must be inserted");
    }
    let edge_metadata = [
        json!({"profile_kind": "object", "profile_id": "1"}),
        json!({"profile_kind": "scene", "profile_id": 1}),
        json!({"profile_kind": "object", "profile_id": -2}),
        json!({"profile_kind": "object", "profile_id": 1.5}),
        json!({"profile_kind": "object", "profile_id": null}),
    ];
    generated_image::Entity::insert_many(edge_metadata.into_iter().enumerate().map(
        |(index, metadata_json)| generated_image::ActiveModel {
            project_id: Set(project.id),
            image_type: Set("object_material".into()),
            file_path: Set(format!("benchmark/edge-{index}.png")),
            prompt: Set("benchmark edge".into()),
            model: Set("test".into()),
            width: Set(1),
            height: Set(1),
            file_size: Set(Some(1)),
            ai_score: Set(None),
            is_favorite: Set(false),
            chapter_id: Set(None),
            panel_id: Set(None),
            metadata_json: Set(Some(metadata_json)),
            created_at: Set(Utc::now().naive_utc()),
            ..Default::default()
        },
    ))
    .exec(db)
    .await
    .expect("benchmark edge images must be inserted");
    (project.id, (1..=PROFILE_COUNT).collect())
}

async fn legacy_counts(
    db: &sea_orm::DatabaseConnection,
    project_id: i64,
    profile_ids: &[i64],
) -> BTreeMap<i64, i64> {
    let requested = profile_ids
        .iter()
        .copied()
        .collect::<std::collections::HashSet<_>>();
    let mut counts = BTreeMap::new();
    for image in profile_material_image_identity_rows_by_type(db, project_id, "object_material")
        .await
        .expect("full metadata count must succeed")
    {
        if let Some(metadata) = image.metadata_json.as_ref()
            && metadata.get("profile_kind").and_then(Value::as_str) == Some("object")
            && let Some(profile_id) = metadata.get("profile_id").and_then(Value::as_i64)
            && profile_id > 0
            && requested.contains(&profile_id)
        {
            *counts.entry(profile_id).or_insert(0) += 1;
        }
    }
    counts
}

async fn legacy_ids(
    db: &sea_orm::DatabaseConnection,
    project_id: i64,
    profile_id: i64,
) -> Vec<i64> {
    profile_material_image_identity_rows_by_type(db, project_id, "object_material")
        .await
        .expect("full metadata ID read must succeed")
        .into_iter()
        .filter(|image| {
            image
                .metadata_json
                .as_ref()
                .and_then(|metadata| metadata.get("profile_kind"))
                .and_then(Value::as_str)
                == Some("object")
                && image
                    .metadata_json
                    .as_ref()
                    .and_then(|metadata| metadata.get("profile_id"))
                    .and_then(Value::as_i64)
                    == Some(profile_id)
        })
        .map(|image| image.id)
        .collect()
}

#[tokio::test]
#[ignore = "manual PostgreSQL performance benchmark; requires isolated A4N_TEST_DATABASE_URL"]
async fn benchmark_profile_material_image_counts() {
    let db = a4n_migration::test_support::connect_test_database().await;
    let (project_id, profile_ids) = fixture(&db).await;
    let mut previous = Vec::with_capacity(SAMPLES);
    let mut grouped = Vec::with_capacity(SAMPLES);
    let mut previous_ids = Vec::with_capacity(SAMPLES);
    let mut grouped_ids = Vec::with_capacity(SAMPLES);
    for sample in 0..SAMPLES {
        if sample % 2 == 0 {
            let started = Instant::now();
            let old = legacy_counts(&db, project_id, &profile_ids).await;
            previous.push(started.elapsed().as_secs_f64() * 1000.0);
            assert_eq!(old.values().sum::<i64>(), IMAGE_COUNT as i64);

            let started = Instant::now();
            let next = profile_material_image_counts_by_profile(
                &db,
                project_id,
                "object_material",
                &profile_ids,
            )
            .await
            .expect("grouped image count must succeed");
            grouped.push(started.elapsed().as_secs_f64() * 1000.0);
            assert_eq!(next.values().sum::<i64>(), IMAGE_COUNT as i64);
            assert_eq!(next, old);

            let started = Instant::now();
            let old_ids = legacy_ids(&db, project_id, 1).await;
            previous_ids.push(started.elapsed().as_secs_f64() * 1000.0);
            let started = Instant::now();
            let new_ids =
                profile_material_image_ids_by_profile(&db, project_id, "object_material", 1)
                    .await
                    .expect("grouped image ID read must succeed");
            grouped_ids.push(started.elapsed().as_secs_f64() * 1000.0);
            assert_eq!(new_ids, old_ids);
        } else {
            let started = Instant::now();
            let next = profile_material_image_counts_by_profile(
                &db,
                project_id,
                "object_material",
                &profile_ids,
            )
            .await
            .expect("grouped image count must succeed");
            grouped.push(started.elapsed().as_secs_f64() * 1000.0);
            assert_eq!(next.values().sum::<i64>(), IMAGE_COUNT as i64);

            let started = Instant::now();
            let old = legacy_counts(&db, project_id, &profile_ids).await;
            previous.push(started.elapsed().as_secs_f64() * 1000.0);
            assert_eq!(old, next);

            let started = Instant::now();
            let new_ids =
                profile_material_image_ids_by_profile(&db, project_id, "object_material", 1)
                    .await
                    .expect("grouped image ID read must succeed");
            grouped_ids.push(started.elapsed().as_secs_f64() * 1000.0);
            let started = Instant::now();
            let old_ids = legacy_ids(&db, project_id, 1).await;
            previous_ids.push(started.elapsed().as_secs_f64() * 1000.0);
            assert_eq!(old_ids, new_ids);
        }
    }
    let previous_p50 = percentile(&previous, 0.50);
    let previous_p95 = percentile(&previous, 0.95);
    let grouped_p50 = percentile(&grouped, 0.50);
    let grouped_p95 = percentile(&grouped, 0.95);
    let previous_ids_p50 = percentile(&previous_ids, 0.50);
    let previous_ids_p95 = percentile(&previous_ids, 0.95);
    let grouped_ids_p50 = percentile(&grouped_ids, 0.50);
    let grouped_ids_p95 = percentile(&grouped_ids, 0.95);
    eprintln!(
        "profile_material_image_counts images={IMAGE_COUNT} metadata_bytes={} previous_p50_p95_ms={previous_p50:.3}/{previous_p95:.3} grouped_p50_p95_ms={grouped_p50:.3}/{grouped_p95:.3} improvement={:.1}%/{:.1}% ids_previous_p50_p95_ms={previous_ids_p50:.3}/{previous_ids_p95:.3} ids_grouped_p50_p95_ms={grouped_ids_p50:.3}/{grouped_ids_p95:.3} ids_improvement={:.1}%/{:.1}%",
        "metadata payload ".repeat(100).len() + 50,
        (1.0 - grouped_p50 / previous_p50) * 100.0,
        (1.0 - grouped_p95 / previous_p95) * 100.0,
        (1.0 - grouped_ids_p50 / previous_ids_p50) * 100.0,
        (1.0 - grouped_ids_p95 / previous_ids_p95) * 100.0,
    );
}
