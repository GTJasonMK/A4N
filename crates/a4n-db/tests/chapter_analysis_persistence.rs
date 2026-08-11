use a4n_core::models::chapter::chapter;
use a4n_db::{
    chapter_content::{
        ChapterContentEditError, StoreChapterAnalysisRecordInput, lock_chapter_analysis_source,
        store_chapter_analysis_record,
    },
    project::{
        ChapterSourceSnapshot, CreateProjectRecordInput, PROJECT_TYPE_NOVEL_LONG,
        create_project_record,
    },
};
use chrono::{Duration, NaiveDate, NaiveDateTime};
use sea_orm::{ActiveModelTrait, DatabaseConnection, Set};
use serde_json::json;

async fn setup_db() -> DatabaseConnection {
    a4n_migration::test_support::connect_test_database().await
}

fn test_time() -> NaiveDateTime {
    NaiveDate::from_ymd_opt(2026, 8, 10)
        .expect("valid date")
        .and_hms_opt(12, 0, 0)
        .expect("valid time")
}

async fn insert_chapter(db: &DatabaseConnection) -> chapter::Model {
    let project_id = create_project_record(
        db,
        CreateProjectRecordInput {
            user_id: 1,
            title: "章节分析持久化测试".to_string(),
            description: String::new(),
            project_type: PROJECT_TYPE_NOVEL_LONG.to_string(),
        },
    )
    .await
    .expect("project must be created")
    .id;
    let now = test_time();
    chapter::ActiveModel {
        project_id: Set(project_id),
        outline_id: Set(None),
        chapter_number: Set(1),
        title: Set(Some("原始标题".to_string())),
        content: Set("原始正文".to_string()),
        word_count: Set(4),
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
    .expect("chapter must be inserted")
}

#[tokio::test]
async fn analysis_write_does_not_change_source_updated_at() {
    let db = setup_db().await;
    let chapter = insert_chapter(&db).await;

    let updated = store_chapter_analysis_record(
        &db,
        chapter.clone(),
        StoreChapterAnalysisRecordInput {
            analysis_data: json!({"summaries": {"compressed": "摘要"}}),
            summary: Some("摘要".to_string()),
            style_tags: Some(json!(["悬疑"])),
        },
    )
    .await
    .expect("analysis write must succeed");

    assert_eq!(updated.updated_at, chapter.updated_at);
    assert_eq!(updated.content, chapter.content);
    assert_eq!(updated.title, chapter.title);
}

#[tokio::test]
async fn stale_source_snapshot_rejects_analysis_write() {
    let db = setup_db().await;
    let chapter = insert_chapter(&db).await;
    let source = ChapterSourceSnapshot::from(&chapter);
    let mut active: chapter::ActiveModel = chapter.into();
    active.content = Set("用户刚刚修改的正文".to_string());
    active.updated_at = Set(test_time() + Duration::seconds(1));
    active
        .update(&db)
        .await
        .expect("concurrent content edit must succeed");

    let error = lock_chapter_analysis_source(&db, &source)
        .await
        .expect_err("stale analysis source must be rejected");

    assert!(matches!(
        error,
        ChapterContentEditError::InvalidState { current, .. }
            if current == "chapter_source_changed"
    ));
}
