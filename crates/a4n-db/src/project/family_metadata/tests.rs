use super::contracts::{ProjectFamilyMetadataQuery, ProjectFamilyMetadataReadError};
use super::projection::build_project_family_metadata;
use super::queries::{project_family_metadata, project_family_metadata_batch};
use crate::project::{SOURCE_MODE_DERIVED, SOURCE_MODE_SELF};
use a4n_core::models::project::{novel_project, project_derivation, project_source_snapshot};
use chrono::{NaiveDate, NaiveDateTime};
use sea_orm::{DatabaseConnection, DbErr};
use serde_json::json;
use std::{collections::HashMap, error::Error};

#[test]
fn preserves_family_fields() {
    let root = project();
    let snapshot = project_source_snapshot::Model {
        id: 20,
        root_project_id: 7,
        source_version_hash: "source-hash".to_string(),
        blueprint_version_id: None,
        chapter_scope_json: json!({"start": 1, "end": 2}),
        chapter_hashes_json: json!({}),
        profile_hashes_json: json!({}),
        created_at: naive_datetime(),
    };
    let derivation = project_derivation::Model {
        id: 30,
        root_project_id: 7,
        derived_project_id: 10,
        derived_kind: "comic".to_string(),
        source_snapshot_id: 20,
        chapter_scope_json: json!({"start_chapter": 1, "end_chapter": 3}),
        creation_options_json: json!({}),
        created_at: naive_datetime(),
    };
    let query = ProjectFamilyMetadataQuery {
        project_id: 10,
        root_project_id: Some(7),
        source_snapshot_id: Some(20),
        source_mode: SOURCE_MODE_DERIVED.to_string(),
    };

    let metadata = build_project_family_metadata(
        &query,
        &HashMap::from([(root.id, root)]),
        &HashMap::from([(snapshot.id, snapshot)]),
        &HashMap::from([(derivation.derived_project_id, derivation)]),
    )
    .expect("family metadata should be complete");

    assert_eq!(metadata.project_id, 10);
    assert_eq!(metadata.root_project_title.as_deref(), Some("项目"));
    assert_eq!(
        metadata.source_snapshot_created_at.as_deref(),
        Some("2026-01-01 00:00:00")
    );
    assert_eq!(
        metadata.derived_chapter_scope,
        Some(json!({"start_chapter": 1, "end_chapter": 3}))
    );
}

#[test]
fn keeps_missing_reference_errors() {
    let missing_root = build_project_family_metadata(
        &ProjectFamilyMetadataQuery {
            project_id: 10,
            root_project_id: Some(7),
            source_snapshot_id: None,
            source_mode: SOURCE_MODE_SELF.to_string(),
        },
        &HashMap::new(),
        &HashMap::new(),
        &HashMap::new(),
    )
    .expect_err("missing root should be rejected");
    assert!(matches!(
        missing_root,
        ProjectFamilyMetadataReadError::RootProjectMissing { root_project_id: 7 }
    ));

    let missing_snapshot = build_project_family_metadata(
        &ProjectFamilyMetadataQuery {
            project_id: 10,
            root_project_id: None,
            source_snapshot_id: Some(20),
            source_mode: SOURCE_MODE_SELF.to_string(),
        },
        &HashMap::new(),
        &HashMap::new(),
        &HashMap::new(),
    )
    .expect_err("missing snapshot should be rejected");
    assert!(matches!(
        missing_snapshot,
        ProjectFamilyMetadataReadError::SourceSnapshotMissing {
            source_snapshot_id: 20
        }
    ));

    let missing_derivation = build_project_family_metadata(
        &ProjectFamilyMetadataQuery {
            project_id: 10,
            root_project_id: None,
            source_snapshot_id: None,
            source_mode: SOURCE_MODE_DERIVED.to_string(),
        },
        &HashMap::new(),
        &HashMap::new(),
        &HashMap::new(),
    )
    .expect_err("missing derivation should be rejected");
    assert!(matches!(
        missing_derivation,
        ProjectFamilyMetadataReadError::DerivationMissing { project_id: 10 }
    ));
}

#[test]
fn read_error_preserves_invariants_and_database_source() {
    let missing = ProjectFamilyMetadataReadError::MetadataMissing { project_id: 17 };
    assert_eq!(missing.to_string(), "项目缺少族元数据: 17");
    assert!(missing.source().is_none());

    let database = ProjectFamilyMetadataReadError::database(
        "查询项目族根项目失败",
        DbErr::Custom("query failed".to_string()),
    );
    assert!(matches!(
        &database,
        ProjectFamilyMetadataReadError::Database {
            operation: "查询项目族根项目失败",
            source: DbErr::Custom(message),
        } if message == "query failed"
    ));
    assert!(database.source().is_some());
}

#[tokio::test]
async fn validates_ids_and_empty_batch_before_database_access() {
    let db = DatabaseConnection::Disconnected;
    let invalid_queries = [
        (
            ProjectFamilyMetadataQuery {
                project_id: 0,
                root_project_id: None,
                source_snapshot_id: None,
                source_mode: SOURCE_MODE_SELF.to_string(),
            },
            "project_id 必须大于 0",
        ),
        (
            ProjectFamilyMetadataQuery {
                project_id: 1,
                root_project_id: Some(0),
                source_snapshot_id: None,
                source_mode: SOURCE_MODE_DERIVED.to_string(),
            },
            "root_project_id 必须大于 0",
        ),
        (
            ProjectFamilyMetadataQuery {
                project_id: 1,
                root_project_id: None,
                source_snapshot_id: Some(0),
                source_mode: SOURCE_MODE_DERIVED.to_string(),
            },
            "source_snapshot_id 必须大于 0",
        ),
    ];

    for (query, expected) in invalid_queries {
        assert!(matches!(
            project_family_metadata(&db, query).await,
            Err(ProjectFamilyMetadataReadError::InvalidArgument(message))
                if message == expected
        ));
    }

    assert!(
        project_family_metadata_batch(&db, &[])
            .await
            .expect("empty metadata batch must not access the database")
            .is_empty()
    );
}

fn project() -> novel_project::Model {
    novel_project::Model {
        id: 7,
        user_id: 1,
        title: "项目".to_string(),
        description: String::new(),
        project_type: "novel_long".to_string(),
        project_kind: "text".to_string(),
        source_mode: SOURCE_MODE_SELF.to_string(),
        root_project_id: None,
        parent_project_id: None,
        source_snapshot_id: None,
        source_sync_status: "current".to_string(),
        derived_project_status: None,
        status: "draft".to_string(),
        target_chapters: 10,
        current_chapter: 1,
        total_chapters: Some(10),
        chapters_per_part: None,
        avatar_svg: None,
        avatar_animal: None,
        raw_blueprint: None,
        import_analysis_progress: None,
        export_config: None,
        created_at: naive_datetime(),
        updated_at: naive_datetime(),
    }
}

fn naive_datetime() -> NaiveDateTime {
    NaiveDate::from_ymd_opt(2026, 1, 1)
        .unwrap()
        .and_hms_opt(0, 0, 0)
        .unwrap()
}
