use super::{BlueprintRestoreError, blueprint_restore_error_to_app};
use a4n_error::AppError;
use sea_orm::DbErr;

#[test]
fn blueprint_restore_error_mapping_preserves_structured_classification() {
    assert!(matches!(
        blueprint_restore_error_to_app(BlueprintRestoreError::VersionNotFound {
            project_id: 7,
            version: "v2".to_string(),
        }),
        AppError::NotFound(message)
            if message == "蓝图版本不存在: project_id=7, version=v2"
    ));
    assert!(matches!(
        blueprint_restore_error_to_app(BlueprintRestoreError::CharacterSync {
            source: a4n_db::project::BlueprintSyncError::Database(DbErr::Custom(
                "sync failed".to_string(),
            )),
        }),
        AppError::Database(DbErr::Custom(message)) if message == "sync failed"
    ));
    assert!(matches!(
        blueprint_restore_error_to_app(BlueprintRestoreError::StatusWrite {
            source: a4n_db::project::ProjectStatusWriteError::StatusChanged {
                expected: "draft".to_string(),
                actual: "writing".to_string(),
            },
        }),
        AppError::InvalidState { current, operation }
            if current == "writing" && operation.contains("draft")
    ));
    assert!(matches!(
        blueprint_restore_error_to_app(BlueprintRestoreError::Serialization {
            source: serde_json::from_str::<serde_json::Value>("{").unwrap_err(),
        }),
        AppError::Serialization(_)
    ));
}
