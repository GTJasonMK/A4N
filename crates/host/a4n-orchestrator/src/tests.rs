use super::{freeform_generated_reset_error_to_app_error, project_delete_error_to_app_error};

#[test]
fn project_delete_error_keeps_typed_app_error_mapping() {
    let missing = project_delete_error_to_app_error(
        a4n_db::project::delete::ProjectDeleteDbError::ProjectNotFound { project_id: 9 },
    );
    assert!(matches!(
        missing,
        a4n_error::AppError::NotFound(message)
            if message == "项目不存在: project_id=9"
    ));

    let database = project_delete_error_to_app_error(
        a4n_db::project::delete::ProjectDeleteDbError::Database {
            source: sea_orm::DbErr::Custom("delete failed".to_string()),
        },
    );
    assert!(matches!(&database, a4n_error::AppError::Database(_)));
    assert!(database.to_string().contains("delete failed"));

    let cleanup_database = project_delete_error_to_app_error(
        a4n_db::project::delete::ProjectDeleteDbError::ProjectCleanup {
            source: a4n_db::project::ProjectCleanupDbError::Database {
                source: sea_orm::DbErr::Custom("cleanup failed".to_string()),
            },
        },
    );
    assert!(matches!(
        &cleanup_database,
        a4n_error::AppError::Database(_)
    ));
    assert!(cleanup_database.to_string().contains("cleanup failed"));
}

#[test]
fn freeform_generated_reset_error_keeps_typed_app_error_mapping() {
    let missing = freeform_generated_reset_error_to_app_error(
        a4n_db::project::FreeformGeneratedResetDbError::ProjectNotFound { project_id: 11 },
    );
    assert!(matches!(
        missing,
        a4n_error::AppError::NotFound(message) if message == "项目不存在: 11"
    ));

    let unsupported = freeform_generated_reset_error_to_app_error(
        a4n_db::project::FreeformGeneratedResetDbError::UnsupportedProject,
    );
    assert!(matches!(
        unsupported,
        a4n_error::AppError::InvalidArgument(message)
            if message == "仅自由画布视频项目支持该生成数据重置入口"
    ));

    let database = freeform_generated_reset_error_to_app_error(
        a4n_db::project::FreeformGeneratedResetDbError::Database {
            source: sea_orm::DbErr::Custom("reset failed".to_string()),
        },
    );
    assert!(matches!(&database, a4n_error::AppError::Database(_)));
    assert!(database.to_string().contains("reset failed"));
}
