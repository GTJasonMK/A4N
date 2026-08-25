use super::{ProjectRollbackImpactReadError, project_rollback_impacts};
use a4n_core::models::chapter::{chapter, chapter_outline};
use a4n_core::models::project::{
    blueprint_character, blueprint_relationship, blueprint_version, novel_blueprint,
};
use a4n_core::state::rollback::CleanupAction;
use chrono::Utc;
use sea_orm::{ActiveModelTrait, DatabaseConnection, DbErr, Set};
use serde_json::json;
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
            project_type: "novel_long".to_string(),
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

#[tokio::test]
async fn rollback_blueprint_impact_aggregates_all_material_counts() {
    let db = a4n_migration::test_support::connect_test_database().await;
    let project = crate::project::create_project_record(
        &db,
        crate::project::CreateProjectRecordInput {
            user_id: 1,
            title: "蓝图回滚聚合计数".to_string(),
            description: String::new(),
            project_type: "novel_long".to_string(),
        },
    )
    .await
    .unwrap();
    let now = Utc::now().naive_utc();

    novel_blueprint::ActiveModel {
        project_id: Set(project.id),
        title: Set("测试蓝图".to_string()),
        genre: Set("测试".to_string()),
        current_version: Set("1.0".to_string()),
        needs_part_outlines: Set(false),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(&db)
    .await
    .unwrap();
    blueprint_version::ActiveModel {
        project_id: Set(project.id),
        version: Set("1.0".to_string()),
        blueprint_data: Set(json!({"title": "测试蓝图"})),
        created_at: Set(now),
        ..Default::default()
    }
    .insert(&db)
    .await
    .unwrap();
    blueprint_character::ActiveModel {
        project_id: Set(project.id),
        name: Set("角色甲".to_string()),
        character_type: Set("protagonist".to_string()),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(&db)
    .await
    .unwrap();
    blueprint_relationship::ActiveModel {
        project_id: Set(project.id),
        relationship_type: Set("friend".to_string()),
        character_a: Set("角色甲".to_string()),
        character_b: Set("角色乙".to_string()),
        created_at: Set(now),
        ..Default::default()
    }
    .insert(&db)
    .await
    .unwrap();

    let impacts =
        project_rollback_impacts(&db, project.id, &[CleanupAction::DeleteBlueprintVersions])
            .await
            .unwrap();

    assert_eq!(impacts.len(), 1);
    assert_eq!(impacts[0].title, "删除蓝图与版本");
    assert_eq!(impacts[0].count, 4);
}

#[tokio::test]
async fn rollback_chapter_impact_aggregates_counts_and_range() {
    let db = a4n_migration::test_support::connect_test_database().await;
    let project = crate::project::create_project_record(
        &db,
        crate::project::CreateProjectRecordInput {
            user_id: 1,
            title: "章节回滚聚合计数".to_string(),
            description: String::new(),
            project_type: "novel_long".to_string(),
        },
    )
    .await
    .unwrap();
    let now = Utc::now().naive_utc();
    chapter::ActiveModel {
        project_id: Set(project.id),
        chapter_number: Set(3),
        content: Set("测试正文".to_string()),
        word_count: Set(4),
        status: Set(chapter::STATUS_DRAFT.to_string()),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(&db)
    .await
    .unwrap();

    let impacts = project_rollback_impacts(&db, project.id, &[CleanupAction::DeleteChapters])
        .await
        .unwrap();

    assert_eq!(impacts.len(), 1);
    assert_eq!(impacts[0].title, "删除章节正文与派生数据");
    assert_eq!(impacts[0].count, 1);
    assert_eq!(impacts[0].chapter_start, Some(3));
    assert_eq!(impacts[0].chapter_end, Some(3));
    assert!(
        impacts[0]
            .note
            .as_deref()
            .is_some_and(|note| note.contains("删除 0 个项目任务"))
    );
}
