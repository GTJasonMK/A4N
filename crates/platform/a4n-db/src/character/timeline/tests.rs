use super::*;
use crate::character::CharacterDbError;
use sea_orm::DatabaseConnection;

#[tokio::test]
async fn timeline_range_validation_precedes_database_access() {
    let disconnected = DatabaseConnection::default();
    let error = delete_timeline_artifacts_from_chapter_checked(&disconnected, 7, 3, Some(2))
        .await
        .expect_err("reversed timeline range must fail before database access");

    assert!(matches!(
        error,
        CharacterDbError::InvalidArgument(message)
            if message == "时间线清理起始章节不能大于结束章节"
    ));
}

#[tokio::test]
async fn profile_rollback_preserves_invalid_argument_and_database_errors() {
    let disconnected = DatabaseConnection::default();
    let invalid_argument = rollback_profile_to_chapter(&disconnected, 7, -1)
        .await
        .expect_err("negative rollback chapter must fail before database access");
    assert!(matches!(
        invalid_argument,
        CharacterDbError::InvalidArgument(message) if message == "回滚章节号不能小于 0"
    ));

    let database_error = rollback_profile_to_chapter(&disconnected, 7, 0)
        .await
        .expect_err("transaction begin on a disconnected database must fail");
    assert!(matches!(database_error, CharacterDbError::Database(_)));
}
