use super::{
    ProjectResetDbError, reset_project_generated_data_record, rollback_project_status_record,
};
use a4n_core::models::{
    ai::{generated_image, generated_video},
    chapter::chapter,
    project::novel_project,
};
use a4n_core::state::status::ProjectStatus;
use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, DatabaseConnection, DbErr, EntityTrait,
    PaginatorTrait, QueryFilter, Set,
};
use std::error::Error;

async fn setup_db() -> DatabaseConnection {
    a4n_migration::test_support::connect_test_database().await
}

async fn text_project(db: &DatabaseConnection, title: &str) -> novel_project::Model {
    crate::project::create_project_record(
        db,
        crate::project::CreateProjectRecordInput {
            user_id: 1,
            title: title.to_string(),
            description: String::new(),
            project_type: crate::project::PROJECT_TYPE_NOVEL_LONG.to_string(),
        },
    )
    .await
    .unwrap()
}

async fn chapter_record(db: &DatabaseConnection, project_id: i64) -> chapter::Model {
    let now = Utc::now().naive_utc();
    chapter::ActiveModel {
        project_id: Set(project_id),
        outline_id: Set(None),
        chapter_number: Set(1),
        title: Set(Some("第一章".to_string())),
        content: Set("正文".to_string()),
        word_count: Set(2),
        status: Set(chapter::STATUS_COMPLETED.to_string()),
        summary: Set(None),
        style_tags: Set(None),
        analysis_data: Set(None),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(db)
    .await
    .unwrap()
}

async fn generated_image_record(
    db: &DatabaseConnection,
    project_id: i64,
    chapter_id: Option<i64>,
    file_path: &str,
) -> generated_image::Model {
    crate::video::create_generated_image_record(
        db,
        crate::video::CreateGeneratedImageRecordDraftInput {
            project_id,
            image_type: "chapter_image".to_string(),
            file_path: file_path.to_string(),
            prompt: "reset outcome test image".to_string(),
            model: "test".to_string(),
            width: 512,
            height: 512,
            file_size: Some(1024),
            chapter_id,
            panel_id: None,
            metadata_json: None,
            created_at: None,
        },
    )
    .await
    .unwrap()
}

async fn completed_generated_video_record(
    db: &DatabaseConnection,
    project_id: i64,
    chapter_id: i64,
    file_path: &str,
    thumbnail_path: &str,
) -> generated_video::Model {
    let video = crate::video::create_generated_video_record(
        db,
        crate::video::CreateGeneratedVideoRecordDraftInput {
            project_id,
            chapter_id: Some(chapter_id),
            video_type: "chapter_video".to_string(),
            workflow_mode: Some("storyboard".to_string()),
            prompt: "reset outcome test video".to_string(),
            negative_prompt: None,
            model: "test".to_string(),
            width: 1280,
            height: 720,
            duration_secs: 5.0,
            fps: 24,
            source_video_id: None,
            source_image_id: None,
            provider_job_id: None,
            metadata_json: None,
        },
    )
    .await
    .unwrap();
    crate::video::mark_generated_video_completed_and_link_prompt_record(
        db,
        video.id,
        file_path,
        Some(thumbnail_path),
        2048,
        None,
        None,
    )
    .await
    .unwrap()
}

#[test]
fn project_not_found_error_keeps_project_id() {
    let error = ProjectResetDbError::ProjectNotFound { project_id: 42 };

    assert_eq!(error.to_string(), "项目不存在: 42");
    assert!(error.source().is_none());
}

#[test]
fn database_error_keeps_sea_orm_source() {
    let error = ProjectResetDbError::Database {
        source: DbErr::Custom("query failed".to_string()),
    };

    assert_eq!(error.to_string(), "Custom Error: query failed");
    assert!(error.source().is_some());
}

#[test]
fn rollback_status_parse_error_is_invalid_argument() {
    let error = a4n_core::state::validator::StateValidator::parse_status("unknown")
        .map_err(|error| ProjectResetDbError::InvalidArgument(error.to_string()))
        .expect_err("unknown status should fail");

    assert!(matches!(error, ProjectResetDbError::InvalidArgument(_)));
}

#[tokio::test]
async fn reset_returns_only_paths_unreferenced_after_transaction_cleanup() {
    let db = setup_db().await;
    let project = text_project(&db, "生成态重置路径测试").await;
    let other_project = text_project(&db, "生成态重置共享路径项目").await;
    let shared_path = "projects/shared/reset-image.png";
    let unique_path = format!("projects/{}/images/reset-unique.png", project.id);

    generated_image_record(&db, project.id, None, shared_path).await;
    generated_image_record(&db, project.id, None, &unique_path).await;
    let other_image = generated_image_record(&db, other_project.id, None, shared_path).await;

    let mut gated_project: novel_project::ActiveModel = project.clone().into();
    gated_project.lifecycle_gate = Set(crate::generation::PROJECT_GATE_RESETTING.to_string());
    gated_project.update(&db).await.unwrap();
    let outcome = reset_project_generated_data_record(&db, project.id)
        .await
        .unwrap();

    assert_eq!(outcome.media_paths, vec![unique_path]);
    assert_eq!(
        generated_image::Entity::find()
            .filter(generated_image::Column::ProjectId.eq(project.id))
            .count(&db)
            .await
            .unwrap(),
        0
    );
    assert!(
        generated_image::Entity::find_by_id(other_image.id)
            .one(&db)
            .await
            .unwrap()
            .is_some()
    );
}

#[tokio::test]
async fn rollback_returns_only_paths_unreferenced_after_chapter_cleanup() {
    let db = setup_db().await;
    let project = text_project(&db, "状态回退路径测试").await;
    let other_project = text_project(&db, "状态回退共享路径项目").await;
    let mut active: novel_project::ActiveModel = project.into();
    active.status = Set(ProjectStatus::Writing.as_str().to_string());
    let project = active.update(&db).await.unwrap();
    let chapter = chapter_record(&db, project.id).await;
    let shared_path = "projects/shared/rollback-image.png";
    let video_path = format!("projects/{}/videos/rollback-unique.mp4", project.id);
    let thumbnail_path = format!("projects/{}/videos/rollback-unique.jpg", project.id);

    let target_image = generated_image_record(&db, project.id, Some(chapter.id), shared_path).await;
    let target_video =
        completed_generated_video_record(&db, project.id, chapter.id, &video_path, &thumbnail_path)
            .await;
    let other_image = generated_image_record(&db, other_project.id, None, shared_path).await;

    let outcome = rollback_project_status_record(
        &db,
        project.id,
        ProjectStatus::Writing.as_str(),
        ProjectStatus::ChapterOutlinesReady.as_str(),
        Utc::now().naive_utc(),
    )
    .await
    .unwrap();

    assert_eq!(
        outcome.project.status,
        ProjectStatus::ChapterOutlinesReady.as_str()
    );
    assert!(outcome.needs_rag_clear);
    let mut expected_media_paths = vec![video_path, thumbnail_path];
    expected_media_paths.sort();
    assert_eq!(outcome.media_paths, expected_media_paths);
    assert_eq!(
        chapter::Entity::find()
            .filter(chapter::Column::ProjectId.eq(project.id))
            .count(&db)
            .await
            .unwrap(),
        0
    );
    assert!(
        generated_video::Entity::find_by_id(target_video.id)
            .one(&db)
            .await
            .unwrap()
            .is_none()
    );
    assert!(
        generated_image::Entity::find_by_id(target_image.id)
            .one(&db)
            .await
            .unwrap()
            .is_none()
    );
    assert!(
        generated_image::Entity::find_by_id(other_image.id)
            .one(&db)
            .await
            .unwrap()
            .is_some()
    );
}

#[tokio::test]
async fn rollback_status_history_failure_rolls_back_cleanup_and_status() {
    let db = setup_db().await;
    let project = crate::project::create_project_record(
        &db,
        crate::project::CreateProjectRecordInput {
            user_id: 1,
            title: "状态回退事务测试".to_string(),
            description: String::new(),
            project_type: crate::project::PROJECT_TYPE_NOVEL_LONG.to_string(),
        },
    )
    .await
    .unwrap();
    let mut active: novel_project::ActiveModel = project.clone().into();
    active.status = Set("writing".to_string());
    let project = active.update(&db).await.unwrap();
    let now = Utc::now().naive_utc();
    let chapter = chapter::ActiveModel {
        project_id: Set(project.id),
        outline_id: Set(None),
        chapter_number: Set(1),
        title: Set(Some("第一章".to_string())),
        content: Set("正文".to_string()),
        word_count: Set(2),
        status: Set(chapter::STATUS_COMPLETED.to_string()),
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
    let function_name = format!("a4n_test_block_status_history_{}", project.id);
    let trigger_name = format!("a4n_test_block_status_history_trigger_{}", project.id);
    db.execute_unprepared(&format!(
        "DROP TRIGGER IF EXISTS {trigger_name} ON project_status_history; DROP FUNCTION IF EXISTS {function_name}()"
    ))
    .await
    .unwrap();
    db.execute_unprepared(&format!(
        "CREATE FUNCTION {function_name}() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN RAISE EXCEPTION 'blocked status history'; END; $$"
    ))
    .await
    .unwrap();
    db.execute_unprepared(&format!(
        "CREATE TRIGGER {trigger_name} BEFORE INSERT ON project_status_history FOR EACH ROW WHEN (NEW.project_id = {}) EXECUTE FUNCTION {function_name}()",
        project.id
    ))
    .await
    .unwrap();

    let error =
        rollback_project_status_record(&db, project.id, "writing", "chapter_outlines_ready", now)
            .await
            .expect_err("history failure must roll back cleanup and status update");
    assert!(error.to_string().contains("blocked status history"));

    db.execute_unprepared(&format!(
        "DROP TRIGGER {trigger_name} ON project_status_history; DROP FUNCTION {function_name}()"
    ))
    .await
    .unwrap();

    assert_eq!(
        novel_project::Entity::find_by_id(project.id)
            .one(&db)
            .await
            .unwrap()
            .unwrap()
            .status,
        "writing"
    );
    assert!(
        chapter::Entity::find_by_id(chapter.id)
            .one(&db)
            .await
            .unwrap()
            .is_some()
    );
    assert_eq!(
        a4n_core::models::project::project_status_history::Entity::find()
            .filter(
                a4n_core::models::project::project_status_history::Column::ProjectId.eq(project.id),
            )
            .count(&db)
            .await
            .unwrap(),
        0
    );
}
