use super::FreeformGeneratedResetDbError;
use crate::project::cleanup::ProjectCleanupDbError;
use sea_orm::DbErr;
use std::error::Error;

#[test]
fn freeform_generated_reset_error_classifies_and_preserves_sources() {
    let missing = FreeformGeneratedResetDbError::ProjectNotFound { project_id: 17 };
    assert_eq!(missing.to_string(), "项目不存在: 17");
    assert!(missing.source().is_none());

    let unsupported = FreeformGeneratedResetDbError::UnsupportedProject;
    assert_eq!(
        unsupported.to_string(),
        "仅自由画布视频项目支持该生成数据重置入口"
    );

    let cleanup = FreeformGeneratedResetDbError::from(ProjectCleanupDbError::Database {
        source: DbErr::Custom("cleanup failed".to_string()),
    });
    assert!(matches!(
        cleanup,
        FreeformGeneratedResetDbError::ProjectCleanup { .. }
    ));
    assert!(cleanup.source().is_some());

    let canvas = FreeformGeneratedResetDbError::from(
        crate::canvas::CanvasDbError::InvalidArgument("invalid canvas".to_string()),
    );
    assert!(matches!(
        canvas,
        FreeformGeneratedResetDbError::Canvas { .. }
    ));
    assert!(canvas.source().is_some());

    let database = FreeformGeneratedResetDbError::from(DbErr::Custom("begin failed".into()));
    assert!(matches!(
        database,
        FreeformGeneratedResetDbError::Database {
            source: DbErr::Custom(ref message),
        } if message == "begin failed"
    ));
    assert!(database.source().is_some());
}
