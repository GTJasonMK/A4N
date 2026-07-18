use super::*;
use crate::project::PROJECT_TYPE_NOVEL_LONG;
use a4n_core::state::status::ProjectStatus;
use sea_orm::{ColumnTrait, DbErr, EntityTrait, PaginatorTrait, QueryFilter};
use std::error::Error;

#[test]
fn validate_import_text_project_chapter_bounds_rejects_content_gap() {
    let err = validate_import_text_project_chapter_bounds(5, 3, None, None, 3, 3, 2)
        .expect_err("available imported content must be continuous from chapter 1");

    assert!(matches!(
        err,
        ImportTextProjectError::InvalidPackage(message)
            if message.contains("导入可用正文必须从第1章连续到当前章节")
                && message.contains("current_chapter=3")
                && message.contains("available_content_chapters=2")
    ));
}

#[test]
fn validate_import_text_project_chapter_bounds_accepts_contiguous_content() {
    validate_import_text_project_chapter_bounds(5, 3, None, None, 3, 3, 3)
        .expect("chapters 1..=current_chapter should be a valid import boundary");
}

#[test]
fn validate_import_text_project_chapter_bounds_accepts_deleted_content_beyond_current() {
    validate_import_text_project_chapter_bounds(5, 1, None, None, 2, 1, 1).expect(
        "deleted chapter records may exist beyond current_chapter without becoming available content",
    );
}

#[test]
fn import_text_project_error_preserves_database_operation_and_source() {
    let database = ImportTextProjectError::database(
        "导入章节失败",
        DbErr::Custom("insert failed".to_string()),
    );

    assert_eq!(
        database.to_string(),
        "导入章节失败: Custom Error: insert failed"
    );
    assert!(matches!(
        database,
        ImportTextProjectError::Database {
            operation: "导入章节失败",
            source: DbErr::Custom(ref message),
        } if message == "insert failed"
    ));
    assert!(database.source().is_some());
}

#[test]
fn import_text_project_chapter_bound_counts_and_preserved_tags_are_infallible() {
    let mut payload = ImportTextProjectPayload {
        chapters: vec![
            ImportTextProjectChapter {
                old_outline_id: None,
                chapter_number: 1,
                title: None,
                content: "正文".to_string(),
                word_count: 2,
                status: chapter::STATUS_COMPLETED.to_string(),
                summary: None,
                style_tags: None,
                analysis_data: None,
            },
            ImportTextProjectChapter {
                old_outline_id: None,
                chapter_number: 3,
                title: None,
                content: String::new(),
                word_count: 0,
                status: chapter::STATUS_DELETED.to_string(),
                summary: None,
                style_tags: None,
                analysis_data: None,
            },
        ],
        blueprint: None,
        blueprint_versions: Vec::new(),
        characters: Vec::new(),
        relationships: Vec::new(),
        part_outlines: Vec::new(),
        chapter_outlines: Vec::new(),
    };

    assert_eq!(payload.chapter_bound_counts(), (3, 1, 1));
    assert!(!payload.has_preserved_chapter_tags());

    payload.chapters[0].style_tags = Some(serde_json::json!(["紧张"]));
    assert!(payload.has_preserved_chapter_tags());
}

#[tokio::test]
async fn import_text_project_transaction_begin_failure_preserves_database_error() {
    let input = ImportTextProjectRecordInput {
        user_id: 1,
        title: "导入项目".to_string(),
        description: String::new(),
        project_type: PROJECT_TYPE_NOVEL_LONG.to_string(),
        status: ProjectStatus::Draft.as_str().to_string(),
        target_chapters: 1,
        current_chapter: 0,
        total_chapters: None,
        chapters_per_part: None,
        avatar_svg: None,
        avatar_animal: None,
        raw_blueprint: None,
        import_analysis_progress: None,
        export_config: None,
        payload: ImportTextProjectPayload {
            chapters: Vec::new(),
            blueprint: None,
            blueprint_versions: Vec::new(),
            characters: Vec::new(),
            relationships: Vec::new(),
            part_outlines: Vec::new(),
            chapter_outlines: Vec::new(),
        },
    };

    let error = import_text_project_record(&DatabaseConnection::Disconnected, input)
        .await
        .expect_err("disconnected database must fail before import writes");
    assert!(matches!(
        error,
        ImportTextProjectError::Database {
            operation: "启动文本项目导入事务失败",
            ..
        }
    ));
    assert!(error.source().is_some());
}

#[tokio::test]
async fn text_project_import_rolls_back_project_on_late_reference_error() {
    let db = a4n_migration::test_support::connect_test_database().await;
    let title = "文本项目导入晚期失败回滚";
    let count_before = novel_project::Entity::find()
        .filter(novel_project::Column::Title.eq(title))
        .count(&db)
        .await
        .unwrap();
    let input = ImportTextProjectRecordInput {
        user_id: 1,
        title: title.to_string(),
        description: String::new(),
        project_type: PROJECT_TYPE_NOVEL_LONG.to_string(),
        status: ProjectStatus::Writing.as_str().to_string(),
        target_chapters: 1,
        current_chapter: 1,
        total_chapters: Some(1),
        chapters_per_part: None,
        avatar_svg: None,
        avatar_animal: None,
        raw_blueprint: None,
        import_analysis_progress: None,
        export_config: None,
        payload: ImportTextProjectPayload {
            chapters: vec![ImportTextProjectChapter {
                old_outline_id: Some(i64::MAX),
                chapter_number: 1,
                title: Some("第一章".to_string()),
                content: "正文".to_string(),
                word_count: 2,
                status: chapter::STATUS_COMPLETED.to_string(),
                summary: None,
                style_tags: None,
                analysis_data: None,
            }],
            blueprint: None,
            blueprint_versions: Vec::new(),
            characters: Vec::new(),
            relationships: Vec::new(),
            part_outlines: Vec::new(),
            chapter_outlines: Vec::new(),
        },
    };

    let error = import_text_project_record(&db, input)
        .await
        .expect_err("missing remapped outline must abort the import transaction");
    assert!(matches!(
        error,
        ImportTextProjectError::InvalidPackage(message)
            if message.contains("引用的章节大纲不存在")
    ));

    let count_after = novel_project::Entity::find()
        .filter(novel_project::Column::Title.eq(title))
        .count(&db)
        .await
        .unwrap();
    assert_eq!(count_after, count_before);
}
