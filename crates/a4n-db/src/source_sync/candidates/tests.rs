use sea_orm::{ActiveModelTrait, DatabaseConnection, Set};

use super::{
    generation_specs_for_profile_refs, generation_specs_for_source_chapter,
    generation_specs_for_source_chapters,
};
use crate::source_sync::SourceSyncDbError;
use a4n_core::models::source_sync::source_sync_run;
use a4n_core::models::{
    generation::{generation_spec, generation_spec_entity_ref},
    project::{project_source_snapshot, source_snapshot_profile},
};
use chrono::Utc;
use serde_json::json;

#[tokio::test]
async fn source_chapter_candidates_validate_project_id_before_database_access() {
    let error = generation_specs_for_source_chapter(&DatabaseConnection::Disconnected, 0, 1)
        .await
        .expect_err("invalid project id must be rejected");

    assert!(matches!(
        error,
        SourceSyncDbError::InvalidArgument(message)
            if message == "derived_project_id 必须为正数"
    ));
}

#[tokio::test]
async fn source_chapter_candidates_validate_source_chapter_before_database_access() {
    let error = generation_specs_for_source_chapter(&DatabaseConnection::Disconnected, 1, 0)
        .await
        .expect_err("invalid source chapter id must be rejected");

    assert!(matches!(
        error,
        SourceSyncDbError::InvalidArgument(message)
            if message == "source_chapter_id 必须为正数"
    ));
}

#[tokio::test]
async fn source_chapter_batch_candidates_validate_all_ids_before_database_access() {
    let error = generation_specs_for_source_chapters(&DatabaseConnection::Disconnected, 1, &[2, 0])
        .await
        .expect_err("invalid source chapter id must be rejected");

    assert!(matches!(
        error,
        SourceSyncDbError::InvalidArgument(message)
            if message == "source_chapter_id 必须为正数"
    ));
}

#[tokio::test]
async fn empty_source_chapter_batch_skips_database_access() {
    let specs = generation_specs_for_source_chapters(&DatabaseConnection::Disconnected, 1, &[])
        .await
        .expect("empty batch should not access the database");

    assert!(specs.is_empty());
}

#[tokio::test]
async fn empty_profile_batch_skips_database_access_after_run_validation() {
    let now = Utc::now().naive_utc();
    let run = source_sync_run::Model {
        id: 1,
        derived_project_id: 2,
        root_project_id: 3,
        from_snapshot_id: 4,
        to_snapshot_id: 5,
        mode: "dry_run".to_string(),
        status: "planned".to_string(),
        summary_json: json!({}),
        error_message: None,
        created_at: now,
        updated_at: now,
    };
    let refs = generation_specs_for_profile_refs(&DatabaseConnection::Disconnected, &run, &[])
        .await
        .expect("empty profile batch should not access the database");

    assert!(refs.is_empty());
}

#[tokio::test]
async fn profile_batch_candidates_reads_requested_refs_and_specs() {
    let db = a4n_migration::test_support::connect_test_database().await;
    let project = crate::project::create_project_record(
        &db,
        crate::project::CreateProjectRecordInput {
            user_id: 1,
            title: format!("源同步 profile 批量候选 {}", uuid::Uuid::new_v4()),
            description: String::new(),
            project_type: "novel_long".to_string(),
        },
    )
    .await
    .expect("test project must be created");
    let now = Utc::now().naive_utc();
    let snapshot = project_source_snapshot::ActiveModel {
        root_project_id: Set(project.id),
        source_version_hash: Set("s".repeat(64)),
        blueprint_version_id: Set(None),
        chapter_scope_json: Set(json!({"kind": "all"})),
        chapter_hashes_json: Set(json!([])),
        profile_hashes_json: Set(json!({})),
        created_at: Set(now),
        ..Default::default()
    }
    .insert(&db)
    .await
    .expect("source snapshot must be inserted");
    let character_id = project.id * 10 + 1;
    let object_id = project.id * 10 + 2;
    for (profile_kind, source_profile_id) in [("character", character_id), ("object", object_id)] {
        source_snapshot_profile::ActiveModel {
            snapshot_id: Set(snapshot.id),
            root_project_id: Set(project.id),
            profile_kind: Set(profile_kind.to_string()),
            source_profile_id: Set(source_profile_id),
            payload_json: Set(json!({"id": source_profile_id})),
            payload_hash: Set("p".repeat(64)),
            ..Default::default()
        }
        .insert(&db)
        .await
        .expect("source snapshot profile must be inserted");
    }
    let insert_spec = |target_entity_id| generation_spec::ActiveModel {
        project_id: Set(project.id),
        root_project_id: Set(project.id),
        source_context_kind: Set(Some("snapshot_backed".to_string())),
        source_snapshot_id: Set(Some(snapshot.id)),
        source_chapter_id: Set(None),
        source_snapshot_chapter_id: Set(None),
        canvas_text_asset_id: Set(None),
        target_kind: Set("final_assembly".to_string()),
        target_entity_kind: Set("assembly".to_string()),
        target_entity_id: Set(None),
        status: Set("draft".to_string()),
        source_version_hash: Set("a".repeat(64)),
        params_json: Set(json!({"profile_id": target_entity_id})),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    };
    let character_spec = insert_spec(character_id)
        .insert(&db)
        .await
        .expect("character generation spec must be inserted");
    let object_spec = insert_spec(object_id)
        .insert(&db)
        .await
        .expect("object generation spec must be inserted");
    generation_spec_entity_ref::ActiveModel {
        generation_spec_id: Set(character_spec.id),
        entity_kind: Set("character".to_string()),
        entity_id: Set(character_id),
        role: Set("subject".to_string()),
    }
    .insert(&db)
    .await
    .expect("character generation spec ref must be inserted");
    generation_spec_entity_ref::ActiveModel {
        generation_spec_id: Set(object_spec.id),
        entity_kind: Set("object".to_string()),
        entity_id: Set(object_id),
        role: Set("subject".to_string()),
    }
    .insert(&db)
    .await
    .expect("object generation spec ref must be inserted");

    let run = source_sync_run::Model {
        id: 1,
        derived_project_id: project.id,
        root_project_id: project.id,
        from_snapshot_id: snapshot.id,
        to_snapshot_id: snapshot.id,
        mode: "dry_run".to_string(),
        status: "planned".to_string(),
        summary_json: json!({}),
        error_message: None,
        created_at: now,
        updated_at: now,
    };
    let records = generation_specs_for_profile_refs(
        &db,
        &run,
        &[
            ("character".to_string(), character_id),
            ("object".to_string(), object_id),
        ],
    )
    .await
    .expect("profile batch candidates must load from PostgreSQL");

    assert_eq!(
        records[&(String::from("character"), character_id)][0]
            .spec
            .id,
        character_spec.id
    );
    assert_eq!(
        records[&(String::from("object"), object_id)][0].spec.id,
        object_spec.id
    );
}
