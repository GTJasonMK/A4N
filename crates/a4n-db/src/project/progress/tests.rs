use super::text::text_project_progress;
use super::{
    ProjectProgressQuery, ProjectProgressReadError, ProjectProgressStatus,
    project_progress_summaries, project_progress_summary,
};
use crate::project::{PROJECT_KIND_TEXT, SOURCE_MODE_SELF};
use sea_orm::DatabaseConnection;

#[tokio::test]
async fn project_progress_rejects_invalid_id_before_database_access() {
    let error = project_progress_summary(
        &DatabaseConnection::Disconnected,
        ProjectProgressQuery {
            project_id: 0,
            project_kind: PROJECT_KIND_TEXT.to_string(),
            source_mode: SOURCE_MODE_SELF.to_string(),
            status: "draft".to_string(),
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
            project_kind: PROJECT_KIND_TEXT.to_string(),
            source_mode: SOURCE_MODE_SELF.to_string(),
            status: "completed".to_string(),
            current_chapter: 10,
            target_chapters: 10,
        },
    )
    .await
    .unwrap();
    assert_eq!(text.status, ProjectProgressStatus::Completed);

    let unknown = project_progress_summary(
        &db,
        ProjectProgressQuery {
            project_id: 2,
            project_kind: "unknown_kind".to_string(),
            source_mode: SOURCE_MODE_SELF.to_string(),
            status: "draft".to_string(),
            current_chapter: 0,
            target_chapters: 0,
        },
    )
    .await
    .unwrap();
    assert_eq!(unknown.status, ProjectProgressStatus::Blocked);

    assert!(
        project_progress_summaries(&db, &[])
            .await
            .unwrap()
            .is_empty()
    );
}

#[tokio::test]
async fn project_progress_batch_keeps_duplicate_id_last_write_wins() {
    let summaries = project_progress_summaries(
        &DatabaseConnection::Disconnected,
        &[
            ProjectProgressQuery {
                project_id: 3,
                project_kind: PROJECT_KIND_TEXT.to_string(),
                source_mode: SOURCE_MODE_SELF.to_string(),
                status: "draft".to_string(),
                current_chapter: 0,
                target_chapters: 10,
            },
            ProjectProgressQuery {
                project_id: 3,
                project_kind: PROJECT_KIND_TEXT.to_string(),
                source_mode: SOURCE_MODE_SELF.to_string(),
                status: "completed".to_string(),
                current_chapter: 10,
                target_chapters: 10,
            },
        ],
    )
    .await
    .unwrap();

    assert_eq!(
        summaries.get(&3).map(|summary| summary.status),
        Some(ProjectProgressStatus::Completed)
    );
}

#[test]
fn project_progress_text_completed_from_query() {
    let progress = text_project_progress(&ProjectProgressQuery {
        project_id: 1,
        project_kind: PROJECT_KIND_TEXT.to_string(),
        source_mode: "self".to_string(),
        status: "completed".to_string(),
        current_chapter: 10,
        target_chapters: 10,
    });

    assert_eq!(progress.kind, PROJECT_KIND_TEXT);
    assert_eq!(progress.stage, "completed");
    assert_eq!(progress.label, "已完成");
    assert_eq!(progress.current, Some(10));
    assert_eq!(progress.total, Some(10));
    assert_eq!(progress.status, ProjectProgressStatus::Completed);
    assert_eq!(progress.detail.as_deref(), Some("当前章节 10/10"));
}
