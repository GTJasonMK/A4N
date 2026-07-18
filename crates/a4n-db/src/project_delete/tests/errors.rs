use std::error::Error;

use sea_orm::DbErr;

use super::super::ProjectDeleteDbError;
use crate::{generation, project as project_db};

#[test]
fn project_delete_error_classifies_domain_failures() {
    let missing = ProjectDeleteDbError::ProjectNotFound { project_id: 42 };
    assert_eq!(missing.to_string(), "项目不存在: project_id=42");
    assert!(missing.source().is_none());

    let derived = ProjectDeleteDbError::DerivedProjectsExist {
        project_id: 42,
        derived_count: 3,
    };
    assert_eq!(
        derived.to_string(),
        "文本项目存在 3 个派生项目，请先删除派生项目或执行项目族删除"
    );
    assert!(derived.source().is_none());
}

#[test]
fn project_delete_error_preserves_nested_sources() {
    let database = ProjectDeleteDbError::from(DbErr::Custom("delete failed".to_string()));
    assert!(matches!(
        database,
        ProjectDeleteDbError::Database {
            source: DbErr::Custom(ref message),
        } if message == "delete failed"
    ));
    assert!(database.source().is_some());

    let generation = ProjectDeleteDbError::from(generation::GenerationDbError::InvalidArgument(
        "invalid target".to_string(),
    ));
    assert!(matches!(
        generation,
        ProjectDeleteDbError::Generation { .. }
    ));
    assert!(generation.source().is_some());

    let video = ProjectDeleteDbError::from(crate::video::VideoDbError::NotFound(
        "视频不存在".to_string(),
    ));
    assert!(matches!(video, ProjectDeleteDbError::Video { .. }));
    assert!(video.source().is_some());

    let cleanup = ProjectDeleteDbError::from(project_db::ProjectCleanupDbError::InvalidArgument(
        "invalid project".to_string(),
    ));
    assert!(matches!(
        cleanup,
        ProjectDeleteDbError::ProjectCleanup { .. }
    ));
    assert!(cleanup.source().is_some());
}
