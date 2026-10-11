use a4n_db::{
    fact_feedback::feedback_changes,
    project::{CreateProjectRecordInput, create_project_record},
};
use a4n_db_models::models::project::material_fact_change;
use chrono::Utc;
use sea_orm::{ConnectionTrait, DatabaseConnection, EntityTrait, Set};
use serde_json::json;
use std::time::Instant;

async fn create_fixture(db: &DatabaseConnection) -> i64 {
    let project = create_project_record(
        db,
        CreateProjectRecordInput {
            user_id: 1,
            title: "事实反馈读取性能".into(),
            description: String::new(),
            project_type: "novel_long".into(),
        },
    )
    .await
    .expect("benchmark project must be created");
    let payload = "事实变更证据。".repeat(512);
    let now = Utc::now().naive_utc();
    let mut rows = Vec::with_capacity(5_000);
    for index in 0..5_000_i64 {
        let unchanged = index % 7 == 0;
        let before = if index == 4_999 {
            json!(1)
        } else {
            json!({"value": format!("before-{index}"), "evidence": payload})
        };
        let after = if index == 4_999 {
            json!(1.0)
        } else if unchanged {
            before.clone()
        } else {
            json!({"value": format!("after-{index}"), "evidence": payload})
        };
        rows.push(material_fact_change::ActiveModel {
            project_id: Set(project.id),
            change_key: Set(format!("benchmark-{index}")),
            material_kind: Set(if index % 11 == 0 {
                "chapter_feedback".into()
            } else {
                "world_setting".into()
            }),
            field_path: Set(format!("/field/{index}")),
            operation: Set("replace".into()),
            before_json: Set(before),
            after_json: Set(after),
            source_refs_json: Set(json!([])),
            evidence_json: Set(json!({"quote": payload})),
            reason: Set("benchmark".into()),
            status: Set("applied".into()),
            created_at: Set(now),
            ..Default::default()
        });
    }
    for chunk in rows.chunks(500) {
        material_fact_change::Entity::insert_many(chunk.iter().cloned())
            .exec(db)
            .await
            .expect("benchmark changes must be inserted");
    }
    drop(rows);

    let noise_project = create_project_record(
        db,
        CreateProjectRecordInput {
            user_id: 1,
            title: "事实反馈索引噪声".into(),
            description: String::new(),
            project_type: "novel_long".into(),
        },
    )
    .await
    .expect("noise project must be created");
    let noise_rows = (0..50_000_i64).map(|index| material_fact_change::ActiveModel {
        project_id: Set(noise_project.id),
        change_key: Set(format!("noise-{index}")),
        material_kind: Set("world_setting".into()),
        field_path: Set("/noise".into()),
        operation: Set("replace".into()),
        before_json: Set(json!("before")),
        after_json: Set(json!("after")),
        source_refs_json: Set(json!([])),
        evidence_json: Set(json!("noise")),
        reason: Set("noise".into()),
        status: Set("applied".into()),
        created_at: Set(now),
        ..Default::default()
    });
    for chunk in noise_rows.collect::<Vec<_>>().chunks(1_000) {
        material_fact_change::Entity::insert_many(chunk.iter().cloned())
            .exec(db)
            .await
            .expect("noise changes must be inserted");
    }
    db.execute_unprepared("ANALYZE material_fact_change")
        .await
        .expect("benchmark table statistics must be refreshed");
    project.id
}

async fn full_feedback_changes(
    db: &DatabaseConnection,
    project_id: i64,
) -> Vec<material_fact_change::Model> {
    use sea_orm::{ColumnTrait, QueryFilter, QueryOrder};
    material_fact_change::Entity::find()
        .filter(material_fact_change::Column::ProjectId.eq(project_id))
        .filter(material_fact_change::Column::MaterialKind.ne("chapter_feedback"))
        .order_by_desc(material_fact_change::Column::Id)
        .all(db)
        .await
        .expect("baseline changes must load")
}

fn summary_bytes(mut rows: Vec<material_fact_change::Model>) -> Vec<u8> {
    rows.retain(|row| row.before_json != row.after_json);
    rows.truncate(100);
    serde_json::to_vec(&rows).expect("summary must serialize")
}

fn percentile(samples: &[f64], percentile: f64) -> f64 {
    let mut sorted = samples.to_vec();
    sorted.sort_by(f64::total_cmp);
    let index = ((sorted.len() - 1) as f64 * percentile).round() as usize;
    sorted[index]
}

#[tokio::test]
#[ignore = "manual PostgreSQL performance benchmark; requires isolated A4N_TEST_DATABASE_URL"]
async fn benchmark_fact_feedback_summary_projection() {
    let db = a4n_migration::test_support::connect_test_database().await;
    let project_id = create_fixture(&db).await;
    let expected = summary_bytes(full_feedback_changes(&db, project_id).await);
    let actual = serde_json::to_vec(
        &feedback_changes(&db, project_id)
            .await
            .expect("optimized changes must load"),
    )
    .expect("optimized summary must serialize");
    assert_eq!(
        actual, expected,
        "optimized latest changes must preserve summary bytes"
    );

    for _ in 0..2 {
        let _ = summary_bytes(full_feedback_changes(&db, project_id).await);
        let _ = feedback_changes(&db, project_id)
            .await
            .expect("optimized warmup must load");
    }
    let mut baseline_samples = Vec::with_capacity(15);
    let mut optimized_samples = Vec::with_capacity(15);
    for _ in 0..15 {
        let started = Instant::now();
        let baseline = summary_bytes(full_feedback_changes(&db, project_id).await);
        baseline_samples.push(started.elapsed().as_secs_f64() * 1_000.0);
        let started = Instant::now();
        let optimized = serde_json::to_vec(
            &feedback_changes(&db, project_id)
                .await
                .expect("optimized sample must load"),
        )
        .expect("optimized sample must serialize");
        optimized_samples.push(started.elapsed().as_secs_f64() * 1_000.0);
        assert_eq!(optimized, baseline);
    }
    let optimized_rows = feedback_changes(&db, project_id)
        .await
        .expect("optimized rows must load");
    let bytes = serde_json::to_vec(&optimized_rows)
        .expect("optimized rows must serialize")
        .len();
    println!(
        "{{\"rows\":{},\"summary_bytes\":{},\"baseline_p50_ms\":{:.3},\"baseline_p95_ms\":{:.3},\"optimized_p50_ms\":{:.3},\"optimized_p95_ms\":{:.3}}}",
        optimized_rows.len(),
        bytes,
        percentile(&baseline_samples, 0.50),
        percentile(&baseline_samples, 0.95),
        percentile(&optimized_samples, 0.50),
        percentile(&optimized_samples, 0.95),
    );
}
