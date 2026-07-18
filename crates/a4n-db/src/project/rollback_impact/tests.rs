use super::contracts::{ProjectRollbackImpactQuery, ProjectRollbackImpactReadError};
use super::facts::{ProjectRollbackImpactFacts, project_rollback_impacts};
use super::projection::rollback_impacts_from_facts;
use sea_orm::{DatabaseConnection, DbErr};
use std::error::Error;

#[test]
fn rollback_impacts_from_facts_builds_requested_impacts() {
    let query = ProjectRollbackImpactQuery {
        project_id: 10,
        delete_chapters: true,
        delete_chapter_outlines: true,
        delete_part_outlines: true,
        delete_blueprint_versions: true,
    };
    let facts = ProjectRollbackImpactFacts {
        chapter_numbers: vec![3, 1, 2],
        outline_numbers: vec![8, 5],
        part_numbers: vec![2, 1],
        media_image_count: 4,
        media_video_count: 5,
        task_count: 6,
        object_profile_count: 7,
        scene_profile_count: 8,
        scene_occurrence_count: 9,
        scene_transition_count: 10,
        scene_entity_state_count: 11,
        blueprint_character_count: 12,
        blueprint_relationship_count: 13,
        blueprint_version_count: 14,
        novel_blueprint_count: 15,
    };

    let impacts = rollback_impacts_from_facts(&query, &facts)
        .expect("small rollback impact counts should fit i32");

    let kinds: Vec<&str> = impacts.iter().map(|impact| impact.kind.as_str()).collect();
    assert_eq!(
        kinds,
        vec![
            "delete_chapters",
            "delete_chapter_outlines",
            "delete_part_outlines",
            "delete_blueprint_versions",
        ]
    );
    let chapter_impact = &impacts[0];
    assert_eq!(chapter_impact.count, Some(3));
    assert_eq!(chapter_impact.chapter_start, Some(1));
    assert_eq!(chapter_impact.chapter_end, Some(3));
    let note = chapter_impact.note.as_deref().expect("chapter note");
    assert!(note.contains("4 张章节图片"));
    assert!(note.contains("5 条视频记录"));
    assert!(note.contains("7 个物体档案"));
    assert!(note.contains("8 个场景档案"));
    assert!(note.contains("9 条场景出场"));
    assert!(note.contains("10 条场景位移"));
    assert!(note.contains("11 条场景实体状态"));
    assert!(note.contains("6 个项目任务"));

    assert_eq!(impacts[1].count, Some(2));
    assert_eq!(impacts[1].chapter_start, Some(5));
    assert_eq!(impacts[1].chapter_end, Some(8));
    assert_eq!(impacts[2].count, Some(2));
    assert_eq!(impacts[2].part_start, Some(1));
    assert_eq!(impacts[2].part_end, Some(2));
    assert_eq!(impacts[3].count, Some(54));
}

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
    let query = ProjectRollbackImpactQuery {
        project_id: 0,
        delete_chapters: true,
        delete_chapter_outlines: true,
        delete_part_outlines: true,
        delete_blueprint_versions: true,
    };

    assert!(matches!(
        project_rollback_impacts(&db, query).await,
        Err(ProjectRollbackImpactReadError::InvalidArgument(message))
            if message == "project_id 必须大于 0"
    ));
}
