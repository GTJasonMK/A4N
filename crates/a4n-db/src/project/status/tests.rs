use super::*;
use crate::project::{
    CreateProjectRecordInput, PROJECT_TYPE_NOVEL_LONG, create_project_record, required_text,
};
use a4n_core::models::project::{novel_project, project_status_history};
use a4n_domain::project::ProjectStatus;
use chrono::{Duration, NaiveDate, NaiveDateTime};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, DbErr, EntityTrait, QueryFilter, Set,
};
use std::error::Error;

async fn setup_db() -> DatabaseConnection {
    a4n_migration::test_support::connect_test_database().await
}

fn naive_datetime() -> NaiveDateTime {
    NaiveDate::from_ymd_opt(2026, 1, 1)
        .unwrap()
        .and_hms_opt(0, 0, 0)
        .unwrap()
}

fn test_time(offset_seconds: i64) -> NaiveDateTime {
    naive_datetime() + Duration::seconds(offset_seconds)
}

#[test]
fn project_status_history_summary_maps_fields_and_timestamp() {
    let changed_at = naive_datetime();
    let item = project_status_history::Model {
        id: 9,
        project_id: 7,
        from_status: "draft".to_string(),
        to_status: "blueprint_ready".to_string(),
        reason: Some("蓝图生成完成".to_string()),
        changed_by: Some("system".to_string()),
        changed_at,
    };

    let summary = project_status_history_summary(item);

    assert_eq!(summary.id, 9);
    assert_eq!(summary.project_id, 7);
    assert_eq!(summary.from_status, "draft");
    assert_eq!(summary.to_status, "blueprint_ready");
    assert_eq!(summary.reason.as_deref(), Some("蓝图生成完成"));
    assert_eq!(summary.changed_at, changed_at.to_string());
}

#[test]
fn project_status_history_text_helpers_trim_and_reject_empty_values() {
    assert_eq!(
        required_text(" draft ".to_string(), "from_status").unwrap(),
        "draft"
    );
    assert_eq!(
        required_text("   ".to_string(), "to_status").unwrap_err(),
        "to_status 不能为空"
    );
    assert_eq!(
        normalize_optional_text(Some(" system ".to_string())).as_deref(),
        Some("system")
    );
    assert_eq!(normalize_optional_text(Some(" ".to_string())), None);
    assert_eq!(normalize_optional_text(None), None);
}

#[test]
fn project_status_write_error_classifies_and_preserves_database_source() {
    let invalid = ProjectStatusWriteError::InvalidArgument("project_id 必须大于 0".to_string());
    assert_eq!(invalid.to_string(), "project_id 必须大于 0");
    assert!(invalid.source().is_none());

    let missing = ProjectStatusWriteError::ProjectNotFound { project_id: 17 };
    assert_eq!(missing.to_string(), "项目不存在: 17");
    assert!(missing.source().is_none());

    let changed = ProjectStatusWriteError::StatusChanged {
        expected: "writing".to_string(),
        actual: "draft".to_string(),
    };
    assert_eq!(
        changed.to_string(),
        "项目状态已变化，请刷新后重试: expected=writing, actual=draft"
    );
    assert!(changed.source().is_none());

    let database = ProjectStatusWriteError::from(DbErr::Custom("write failed".to_string()));
    assert!(matches!(
        database,
        ProjectStatusWriteError::Database {
            source: DbErr::Custom(ref message),
        } if message == "write failed"
    ));
    assert!(database.source().is_some());
}

#[test]
fn project_status_read_error_classifies_and_preserves_database_context() {
    let invalid = ProjectStatusReadError::InvalidArgument("project_id 必须大于 0".to_string());
    assert_eq!(invalid.to_string(), "project_id 必须大于 0");
    assert!(invalid.source().is_none());

    let database = ProjectStatusReadError::database(
        "读取项目章节正文",
        DbErr::Custom("query failed".to_string()),
    );
    assert_eq!(
        database.to_string(),
        "读取项目章节正文失败: Custom Error: query failed"
    );
    assert!(matches!(
        database,
        ProjectStatusReadError::Database {
            operation: "读取项目章节正文",
            source: DbErr::Custom(ref message),
        } if message == "query failed"
    ));
    assert!(database.source().is_some());
}

#[tokio::test]
async fn project_status_reads_validate_arguments_before_database_access() {
    let db = DatabaseConnection::Disconnected;

    assert!(matches!(
        project_status_artifacts(&db, 0).await,
        Err(ProjectStatusReadError::InvalidArgument(message))
            if message == "project_id 必须大于 0"
    ));
    assert!(matches!(
        chapter_generation_readiness(&db, 0, 1).await,
        Err(ProjectStatusReadError::InvalidArgument(message))
            if message == "project_id 必须大于 0"
    ));
    assert!(matches!(
        chapter_generation_readiness(&db, 1, 0).await,
        Err(ProjectStatusReadError::InvalidArgument(message))
            if message == "chapter_number 必须大于 0"
    ));
}

#[tokio::test]
async fn update_project_status_record_updates_project_and_history() {
    let db = setup_db().await;
    let project = create_project_record(
        &db,
        CreateProjectRecordInput {
            user_id: 1,
            title: "状态项目".to_string(),
            description: String::new(),
            project_type: PROJECT_TYPE_NOVEL_LONG.to_string(),
        },
    )
    .await
    .unwrap();

    let mut active: novel_project::ActiveModel = project.into();
    active.status = Set(ProjectStatus::Writing.as_str().to_string());
    active.raw_blueprint = Set(Some("{\"title\":\"状态项目\"}".to_string()));
    active.total_chapters = Set(Some(12));
    let project = active.update(&db).await.unwrap();
    let changed_at = test_time(30);

    let updated = update_project_status_record(
        &db,
        project,
        UpdateProjectStatusRecordInput {
            to_status: " draft ".to_string(),
            history_from_status: " writing ".to_string(),
            history_reason: " 项目状态回退 ".to_string(),
            changed_by: Some(" system ".to_string()),
            changed_at,
            clear_draft_fields: true,
        },
    )
    .await
    .unwrap();

    assert_eq!(updated.status, ProjectStatus::Draft.as_str());
    assert_eq!(updated.raw_blueprint, None);
    assert_eq!(updated.total_chapters, None);
    assert_eq!(updated.updated_at, changed_at);

    let history = project_status_history::Entity::find()
        .filter(project_status_history::Column::ProjectId.eq(updated.id))
        .all(&db)
        .await
        .unwrap();
    assert_eq!(history.len(), 1);
    assert_eq!(history[0].from_status, "writing");
    assert_eq!(history[0].to_status, "draft");
    assert_eq!(history[0].reason.as_deref(), Some("项目状态回退"));
    assert_eq!(history[0].changed_by.as_deref(), Some("system"));
    assert_eq!(history[0].changed_at, changed_at);
}
