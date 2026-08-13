use super::{ProjectRollbackImpactReadError, project_rollback_impacts};
use a4n_core::models::chapter::chapter_outline;
use a4n_core::state::rollback::CleanupAction;
use chrono::Utc;
use sea_orm::{ActiveModelTrait, DatabaseConnection, DbErr, Set};
use std::error::Error;

#[test]
fn project_rollback_impact_read_error_preserves_operation_and_source() {
    let invalid =
        ProjectRollbackImpactReadError::InvalidArgument("project_id 必须大于 0".to_string());
    assert_eq!(invalid.to_string(), "project_id 必须大于 0");
    assert!(invalid.source().is_none());

    let database = ProjectRollbackImpactReadError::database(
        "统计项目任务失败",
        DbErr::Custom("count failed".to_string()),
    );
    assert!(matches!(
        database,
        ProjectRollbackImpactReadError::Database {
            operation: "统计项目任务失败",
            source: DbErr::Custom(ref message),
        } if message == "count failed"
    ));
    assert!(database.source().is_some());
}

#[tokio::test]
async fn project_rollback_impacts_rejects_invalid_id_before_database_access() {
    let db = DatabaseConnection::Disconnected;
    assert!(matches!(
        project_rollback_impacts(&db, 0, &[CleanupAction::DeleteChapters]).await,
        Err(ProjectRollbackImpactReadError::InvalidArgument(message))
            if message == "project_id 必须大于 0"
    ));
}

#[tokio::test]
async fn rollback_without_cleanup_actions_needs_no_database_query() {
    let impacts = project_rollback_impacts(&DatabaseConnection::Disconnected, 1, &[])
        .await
        .unwrap();

    assert!(impacts.is_empty());
}

#[tokio::test]
async fn rollback_queries_only_requested_outline_impact() {
    let db = a4n_migration::test_support::connect_test_database().await;
    let project = crate::project::create_project_record(
        &db,
        crate::project::CreateProjectRecordInput {
            user_id: 1,
            title: "回退影响查询范围".to_string(),
            description: String::new(),
            project_type: crate::project::PROJECT_TYPE_NOVEL_LONG.to_string(),
        },
    )
    .await
    .unwrap();
    let now = Utc::now().naive_utc();
    for chapter_number in [2, 5] {
        chapter_outline::ActiveModel {
            project_id: Set(project.id),
            part_id: Set(None),
            chapter_number: Set(chapter_number),
            title: Set(None),
            summary: Set(None),
            beats: Set(None),
            key_scenes: Set(None),
            pacing_score: Set(None),
            foreshadowing_setup: Set(None),
            foreshadowing_payoff: Set(None),
            characters_involved: Set(None),
            word_count_target: Set(None),
            status: Set(chapter_outline::STATUS_COMPLETED.to_string()),
            created_at: Set(now),
            updated_at: Set(now),
            ..Default::default()
        }
        .insert(&db)
        .await
        .unwrap();
    }

    let impacts =
        project_rollback_impacts(&db, project.id, &[CleanupAction::DeleteChapterOutlines])
            .await
            .unwrap();

    assert_eq!(impacts.len(), 1);
    assert_eq!(impacts[0].title, "删除章节大纲");
    assert_eq!(impacts[0].count, 2);
    assert_eq!(impacts[0].chapter_start, Some(2));
    assert_eq!(impacts[0].chapter_end, Some(5));
}
