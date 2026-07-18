use super::delete_generation_spec_records;
use crate::source_sync::SourceSyncDbError;
use sea_orm::DatabaseConnection;

#[tokio::test]
async fn empty_generation_spec_cleanup_does_not_access_database() {
    delete_generation_spec_records(&DatabaseConnection::Disconnected, &[])
        .await
        .expect("empty cleanup must be a no-op");
}

#[tokio::test]
async fn generation_spec_cleanup_validates_all_ids_before_database_access() {
    let error = delete_generation_spec_records(&DatabaseConnection::Disconnected, &[1, 0])
        .await
        .expect_err("invalid generation spec id must be rejected");

    assert!(matches!(
        error,
        SourceSyncDbError::InvalidArgument(message)
            if message == "generation_spec_id 必须为正数"
    ));
}
