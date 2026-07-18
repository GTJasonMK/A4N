use sea_orm::DatabaseConnection;

use super::generation_specs_for_source_chapter;
use crate::source_sync::SourceSyncDbError;

#[tokio::test]
async fn source_chapter_candidates_validate_project_id_before_database_access() {
    let error = generation_specs_for_source_chapter(&DatabaseConnection::Disconnected, 0, 1)
        .await
        .expect_err("invalid project id must be rejected");

    assert!(matches!(
        error,
        SourceSyncDbError::InvalidArgument(message)
            if message == "derived_project_id 必须为正数"
    ));
}

#[tokio::test]
async fn source_chapter_candidates_validate_source_chapter_before_database_access() {
    let error = generation_specs_for_source_chapter(&DatabaseConnection::Disconnected, 1, 0)
        .await
        .expect_err("invalid source chapter id must be rejected");

    assert!(matches!(
        error,
        SourceSyncDbError::InvalidArgument(message)
            if message == "source_chapter_id 必须为正数"
    ));
}
