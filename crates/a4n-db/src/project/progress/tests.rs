use super::text::text_project_progress;
use super::{
    ProjectProgressQuery, ProjectProgressReadError, project_progress_summaries,
    project_progress_summary,
};
use crate::project::SOURCE_MODE_SELF;
use a4n_domain::project::{ProjectKind, ProjectStatus};
use sea_orm::DatabaseConnection;

#[tokio::test]
async fn project_progress_rejects_invalid_id_before_database_access() {
    let error = project_progress_summary(
        &DatabaseConnection::Disconnected,
        ProjectProgressQuery {
            project_id: 0,
            project_kind: ProjectKind::Text,
            source_mode: SOURCE_MODE_SELF.to_string(),
            status: ProjectStatus::Draft,
            current_chapter: 0,
            target_chapters: 10,
        },
    )
    .await
    .expect_err("invalid project ID must fail before progress queries");

    assert!(matches!(
        error,
        ProjectProgressReadError::InvalidArgument(message)
            if message == "project_id 必须大于 0"
    ));
}

#[tokio::test]
async fn project_progress_pure_kinds_and_empty_batch_do_not_access_database() {
    let db = DatabaseConnection::Disconnected;
    let text = project_progress_summary(
        &db,
        ProjectProgressQuery {
            project_id: 1,
            project_kind: ProjectKind::Text,
            source_mode: SOURCE_MODE_SELF.to_string(),
            status: ProjectStatus::Completed,
            current_chapter: 10,
            target_chapters: 10,
        },
    )
    .await
    .unwrap();
    assert_eq!(text.label, "已完成");

    assert!(
        project_progress_summaries(&db, &[])
            .await
            .unwrap()
            .is_empty()
    );
}

#[tokio::test]
async fn project_progress_batch_rejects_duplicate_project_ids() {
    let error = project_progress_summaries(
        &DatabaseConnection::Disconnected,
        &[
            ProjectProgressQuery {
                project_id: 3,
                project_kind: ProjectKind::Text,
                source_mode: SOURCE_MODE_SELF.to_string(),
                status: ProjectStatus::Draft,
                current_chapter: 0,
                target_chapters: 10,
            },
            ProjectProgressQuery {
                project_id: 3,
                project_kind: ProjectKind::Text,
                source_mode: SOURCE_MODE_SELF.to_string(),
                status: ProjectStatus::Completed,
                current_chapter: 10,
                target_chapters: 10,
            },
        ],
    )
    .await
    .unwrap_err();

    assert!(matches!(
        error,
        ProjectProgressReadError::InvalidArgument(message)
            if message == "项目进度查询包含重复 project_id: 3"
    ));
}

#[test]
fn project_progress_text_completed_from_query() {
    let progress = text_project_progress(&ProjectProgressQuery {
        project_id: 1,
        project_kind: ProjectKind::Text,
        source_mode: "self".to_string(),
        status: ProjectStatus::Completed,
        current_chapter: 10,
        target_chapters: 10,
    });

    assert_eq!(progress.label, "已完成");
    assert_eq!(progress.detail.as_deref(), Some("当前章节 10/10"));
}
