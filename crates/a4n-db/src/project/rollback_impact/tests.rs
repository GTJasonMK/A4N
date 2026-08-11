use super::contracts::ProjectRollbackImpactReadError;
use super::facts::{ProjectRollbackImpactFacts, project_rollback_impacts};
use super::projection::rollback_impacts_from_facts;
use a4n_core::state::rollback::CleanupAction;
use sea_orm::{DatabaseConnection, DbErr};
use std::error::Error;

#[test]
fn rollback_impacts_from_facts_builds_requested_impacts() {
    let actions = [
        CleanupAction::DeleteChapters,
        CleanupAction::DeleteChapterOutlines,
        CleanupAction::DeletePartOutlines,
        CleanupAction::DeleteBlueprintVersions,
    ];
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

    let impacts = rollback_impacts_from_facts(&actions, &facts)
        .expect("small rollback impact counts should fit i32");

    let titles: Vec<&str> = impacts.iter().map(|impact| impact.title.as_str()).collect();
    assert_eq!(
        titles,
        vec![
            "删除章节正文与派生数据",
            "删除章节大纲",
            "删除分部大纲",
            "删除蓝图与版本",
        ]
    );
    let chapter_impact = &impacts[0];
    assert_eq!(chapter_impact.count, 3);
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

    assert_eq!(impacts[1].count, 2);
    assert_eq!(impacts[1].chapter_start, Some(5));
    assert_eq!(impacts[1].chapter_end, Some(8));
    assert_eq!(impacts[2].count, 2);
    assert_eq!(impacts[2].part_start, Some(1));
    assert_eq!(impacts[2].part_end, Some(2));
    assert_eq!(impacts[3].count, 54);
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
    assert!(matches!(
        project_rollback_impacts(&db, 0, &[CleanupAction::DeleteChapters]).await,
        Err(ProjectRollbackImpactReadError::InvalidArgument(message))
            if message == "project_id 必须大于 0"
    ));
}
