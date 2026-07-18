use super::*;
use a4n_core::models::chapter::chapter_scene;
use chrono::{NaiveDate, NaiveDateTime};
use sea_orm::{DatabaseConnection, DbErr};
use std::error::Error;

#[test]
fn record_error_preserves_operation_and_database_source() {
    let database = ProjectChapterSceneRecordError::database(
        "读取章节场景",
        DbErr::Custom("query failed".to_string()),
    );
    assert_eq!(
        database.to_string(),
        "读取章节场景失败: Custom Error: query failed"
    );
    assert!(matches!(
        &database,
        ProjectChapterSceneRecordError::Database {
            operation: "读取章节场景",
            source: DbErr::Custom(message),
        } if message == "query failed"
    ));
    assert!(database.source().is_some());
}

#[tokio::test]
async fn record_operations_validate_identity_before_database_access() {
    let db = DatabaseConnection::Disconnected;

    let invalid_project = chapter_by_project_number(&db, 0, 1)
        .await
        .expect_err("invalid project ID must fail before chapter lookup");
    let invalid_chapter_number = chapter_by_project_number(&db, 1, 0)
        .await
        .expect_err("invalid chapter number must fail before chapter lookup");
    let invalid_chapter = chapter_scenes_by_chapter(&db, 0)
        .await
        .expect_err("invalid chapter ID must fail before scene list query");
    let invalid_scene = chapter_scene_by_id(&db, 0)
        .await
        .expect_err("invalid scene ID must fail before scene lookup");
    let invalid_delete = delete_chapter_scene_record(&db, 0)
        .await
        .expect_err("invalid scene ID must fail before scene deletion");

    assert_invalid_argument(invalid_project, "project_id 必须大于 0");
    assert_invalid_argument(invalid_chapter_number, "chapter_number 必须大于 0");
    assert_invalid_argument(invalid_chapter, "chapter_id 必须大于 0");
    assert_invalid_argument(invalid_scene, "scene_id 必须大于 0");
    assert_invalid_argument(invalid_delete, "scene_id 必须大于 0");

    for (scene, expected) in [
        (chapter_scene_record(0, 1, 1), "scene_id 必须大于 0"),
        (chapter_scene_record(1, 0, 1), "chapter_id 必须大于 0"),
        (chapter_scene_record(1, 1, 0), "scene_number 必须大于 0"),
    ] {
        let error = update_chapter_scene_record(
            &db,
            ProjectChapterSceneUpdateInput {
                scene,
                content: None,
                title: None,
                summary: None,
                scene_type: None,
                location: None,
                timeline_position: None,
                updated_at: test_time(),
            },
        )
        .await
        .expect_err("invalid scene identity must fail before scene update");
        assert_invalid_argument(error, expected);
    }
}

#[tokio::test]
async fn reorder_rejects_empty_and_invalid_numbers_before_database_access() {
    let db = DatabaseConnection::Disconnected;
    assert!(matches!(
        chapter_scene_reorder_scope(
            &db,
            ProjectChapterSceneReorderInput { order: Vec::new() },
        )
        .await,
        Err(ProjectChapterSceneReorderError::InvalidArgument(message))
            if message == "场景排序为空"
    ));
    assert!(matches!(
        chapter_scene_reorder_scope(
            &db,
            ProjectChapterSceneReorderInput {
                order: vec![(1, 0)],
            },
        )
        .await,
        Err(ProjectChapterSceneReorderError::InvalidArgument(message))
            if message == "场景序号必须为正数"
    ));
}

fn assert_invalid_argument(error: ProjectChapterSceneRecordError, expected: &str) {
    assert!(matches!(
        error,
        ProjectChapterSceneRecordError::InvalidArgument(message)
            if message == expected
    ));
}

fn chapter_scene_record(id: i64, chapter_id: i64, scene_number: i32) -> chapter_scene::Model {
    chapter_scene::Model {
        id,
        chapter_id,
        scene_number,
        title: None,
        content: "场景正文".to_string(),
        word_count: 4,
        summary: None,
        scene_type: "development".to_string(),
        characters_present: None,
        location: None,
        timeline_position: None,
        transition_from_previous: None,
        created_at: test_time(),
        updated_at: test_time(),
    }
}

fn test_time() -> NaiveDateTime {
    NaiveDate::from_ymd_opt(2026, 1, 1)
        .unwrap()
        .and_hms_opt(0, 0, 0)
        .unwrap()
}
