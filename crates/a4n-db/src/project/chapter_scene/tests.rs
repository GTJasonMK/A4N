use super::*;
use a4n_db_models::models::chapter::{chapter, chapter_scene};
use chrono::{NaiveDate, NaiveDateTime};
use sea_orm::{ActiveModelTrait, DatabaseConnection, DbErr, Set};
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
        chapter_scene_reorder_project_id(&db, &[]).await,
        Err(ProjectChapterSceneWriteError::InvalidArgument(message))
            if message == "场景排序为空"
    ));
    assert!(matches!(
        chapter_scene_reorder_project_id(&db, &[(1, 0)]).await,
        Err(ProjectChapterSceneWriteError::InvalidArgument(message))
            if message == "场景序号必须为正数"
    ));
}

#[tokio::test]
async fn reorder_updates_all_scene_numbers_and_returns_sorted_rows() {
    let db = a4n_migration::test_support::connect_test_database().await;
    let project = crate::project::create_project_record(
        &db,
        crate::project::CreateProjectRecordInput {
            user_id: 1,
            title: "章节场景批量重排".to_string(),
            description: String::new(),
            project_type: "novel_long".to_string(),
        },
    )
    .await
    .unwrap();
    let now = chrono::Utc::now().naive_utc();
    let chapter = chapter::ActiveModel {
        project_id: Set(project.id),
        outline_id: Set(None),
        chapter_number: Set(1),
        title: Set(Some("第一章".to_string())),
        content: Set("正文".to_string()),
        word_count: Set(2),
        status: Set(chapter::STATUS_DRAFT.to_string()),
        summary: Set(None),
        style_tags: Set(None),
        analysis_data: Set(None),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(&db)
    .await
    .unwrap();
    let mut scenes = Vec::new();
    for scene_number in 1..=3 {
        scenes.push(
            save_generated_chapter_scene_record(
                &db,
                ProjectGeneratedChapterSceneSaveInput {
                    chapter_id: chapter.id,
                    scene_number,
                    content: format!("场景 {scene_number}"),
                },
            )
            .await
            .unwrap(),
        );
    }
    let updated = reorder_chapter_scene_records(
        &db,
        &[(scenes[2].id, 1), (scenes[0].id, 2), (scenes[1].id, 3)],
    )
    .await
    .unwrap();
    assert_eq!(
        updated
            .iter()
            .map(|scene| (scene.id, scene.scene_number))
            .collect::<Vec<_>>(),
        vec![(scenes[2].id, 1), (scenes[0].id, 2), (scenes[1].id, 3)]
    );
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
