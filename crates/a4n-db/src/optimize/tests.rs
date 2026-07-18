use super::*;
use sea_orm::DatabaseConnection;

#[tokio::test]
async fn empty_patch_set_returns_none_without_database_access() {
    let result = create_optimization_patch_set(
        &DatabaseConnection::Disconnected,
        OptimizationPatchSetCreateInput {
            turn_id: 1,
            session_id: 2,
            chapter_id: 3,
            base_content_hash: "hash".to_string(),
            status: "pending".to_string(),
            summary: "summary".to_string(),
            patch_decision: "pending".to_string(),
            patch_apply_status: "pending".to_string(),
            patches: Vec::new(),
        },
    )
    .await
    .expect("empty patch sets should be rejected before database access");
    assert!(result.is_none());
}

#[test]
fn optimize_database_error_keeps_variant_and_message() {
    let error = OptimizeDbError::from(DbErr::Custom("query failed".to_string()));
    assert!(matches!(
        error,
        OptimizeDbError::Database(DbErr::Custom(ref message)) if message == "query failed"
    ));
    assert_eq!(error.to_string(), "Custom Error: query failed");
}
