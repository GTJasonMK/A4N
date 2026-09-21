use super::*;
use crate::character::CharacterDbError;
use sea_orm::DatabaseConnection;

#[tokio::test]
async fn reset_rejects_invalid_input_before_database_access() {
    let disconnected = DatabaseConnection::default();
    let invalid_argument = reset_deletion_marks_in(&disconnected, 7, "explicit", "   ")
        .await
        .expect_err("blank key must fail before database access");
    assert!(matches!(
        invalid_argument,
        CharacterDbError::InvalidArgument(message) if message == "属性 key 不能为空"
    ));
}

#[test]
fn count_rules_handle_continuity_and_rewind() {
    assert_eq!(next_deletion_mark_count(2, 4, 5), 3);
    assert_eq!(next_deletion_mark_count(2, 4, 4), 1);
    assert_eq!(next_deletion_mark_count(2, 4, 6), 1);

    assert_eq!(retained_deletion_mark_count(2, 4), Some(3));
    assert_eq!(retained_deletion_mark_count(5, 4), None);
    assert_eq!(retained_deletion_mark_count(1, 0), None);
}
