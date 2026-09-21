use super::*;
use sea_orm::{DatabaseConnection, DbErr};
use std::error::Error;

#[test]
fn project_cleanup_error_preserves_structured_and_nested_sources() {
    let invalid = ProjectCleanupDbError::InvalidArgument("project_id 必须大于 0".to_string());
    assert_eq!(invalid.to_string(), "project_id 必须大于 0");
    assert!(invalid.source().is_none());

    let database = ProjectCleanupDbError::from(DbErr::Custom("cleanup failed".to_string()));
    assert!(matches!(
        database,
        ProjectCleanupDbError::Database {
            source: DbErr::Custom(ref message),
        }
            if message == "cleanup failed"
    ));
    assert!(database.source().is_some());

    let video = ProjectCleanupDbError::from(crate::video::VideoDbError::NotFound(
        "视频不存在".to_string(),
    ));
    assert!(matches!(video, ProjectCleanupDbError::Video { .. }));
    assert!(video.source().is_some());
}

#[tokio::test]
async fn project_domain_cleanup_rejects_invalid_project_before_database_access() {
    let db = DatabaseConnection::Disconnected;

    let optimization = delete_project_optimization_tree(&db, 0).await;
    let conversations = delete_project_conversations(&db, 0).await;
    let blueprints = delete_project_blueprints(&db, 0, true).await;
    let text_anchors =
        delete_project_text_anchors(&db, 0, ProjectTextAnchorDeleteScope::CurrentProjectContent)
            .await;

    for result in [
        optimization.map(|_| ()),
        conversations.map(|_| ()),
        blueprints.map(|_| ()),
        text_anchors.map(|_| ()),
    ] {
        assert!(matches!(
            result,
            Err(ProjectCleanupDbError::InvalidArgument(ref message))
                if message == "project_id 必须大于 0"
        ));
    }
}

#[tokio::test]
async fn project_execution_cleanup_rejects_invalid_project_before_database_access() {
    let db = DatabaseConnection::Disconnected;

    let generation_runs = delete_project_generation_runs(&db, 0, &[], &[]).await;
    let tasks = delete_project_task_queue_records(&db, 0).await;
    let workflows = delete_project_workflow_runs(&db, 0).await;

    for result in [
        generation_runs.map(|_| ()),
        tasks.map(|_| ()),
        workflows.map(|_| ()),
    ] {
        assert!(matches!(
            result,
            Err(ProjectCleanupDbError::InvalidArgument(ref message))
                if message == "project_id 必须大于 0"
        ));
    }
}
