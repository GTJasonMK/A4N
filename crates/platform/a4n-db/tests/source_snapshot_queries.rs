use a4n_db::{
    project::source_snapshot_chapter_content_by_snapshot_and_source,
    project::{CreateProjectRecordInput, create_project_record},
};
use a4n_db_models::models::{
    chapter::chapter,
    project::{project_source_snapshot, source_snapshot_chapter},
};
use chrono::Utc;
use sea_orm::{ActiveModelTrait, Set};
use serde_json::json;

#[tokio::test]
async fn source_snapshot_chapter_content_projection_preserves_content() {
    let db = a4n_migration::test_support::connect_test_database().await;
    let project = create_project_record(
        &db,
        CreateProjectRecordInput {
            user_id: 1,
            title: format!("源快照章节查询 {}", uuid::Uuid::new_v4()),
            description: String::new(),
            project_type: "novel_long".to_string(),
        },
    )
    .await
    .expect("test project must be created");
    let now = Utc::now().naive_utc();
    let source_chapter = chapter::ActiveModel {
        project_id: Set(project.id),
        chapter_number: Set(1),
        title: Set(Some("第一章".to_string())),
        content: Set("源章节正文".to_string()),
        word_count: Set(5),
        status: Set(chapter::STATUS_COMPLETED.to_string()),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(&db)
    .await
    .expect("source chapter must be created");
    let snapshot = project_source_snapshot::ActiveModel {
        root_project_id: Set(project.id),
        source_version_hash: Set(format!("snapshot-{}", uuid::Uuid::new_v4())),
        chapter_scope_json: Set(json!({"kind": "all"})),
        chapter_hashes_json: Set(json!([])),
        profile_hashes_json: Set(json!({})),
        created_at: Set(now),
        ..Default::default()
    }
    .insert(&db)
    .await
    .expect("source snapshot must be created");
    source_snapshot_chapter::ActiveModel {
        snapshot_id: Set(snapshot.id),
        root_project_id: Set(project.id),
        source_chapter_id: Set(source_chapter.id),
        chapter_number: Set(1),
        title: Set(Some("第一章".to_string())),
        content: Set("源章节正文".to_string()),
        summary: Set(Some("摘要".to_string())),
        analysis_data_json: Set(Some(json!({"large": "omitted"}))),
        content_hash: Set("content-hash".to_string()),
        analysis_hash: Set(Some("analysis-hash".to_string())),
        ..Default::default()
    }
    .insert(&db)
    .await
    .expect("source snapshot chapter must be created");

    let content = source_snapshot_chapter_content_by_snapshot_and_source(
        &db,
        project.id,
        snapshot.id,
        source_chapter.id,
    )
    .await
    .expect("content projection must load");
    assert_eq!(content.as_deref(), Some("源章节正文"));
    assert!(
        source_snapshot_chapter_content_by_snapshot_and_source(
            &db,
            project.id + 1,
            snapshot.id,
            source_chapter.id
        )
        .await
        .expect("scope query must execute")
        .is_none()
    );
}
