use super::implicit_review::require_positive_chapter_number;
use super::*;
use crate::character::{CharacterDbError, validate_category};
use sea_orm::DatabaseConnection;
use serde_json::json;

#[test]
fn attribute_state_contract_errors_are_structured() {
    let mut invalid_state = json!("invalid");
    let error = apply_attribute_to_state(
        &mut invalid_state,
        "explicit",
        "appearance",
        "modify",
        json!("白衣"),
    )
    .expect_err("non-object state must fail");

    assert!(matches!(
        error,
        CharacterDbError::InvalidState { current, operation }
            if current == "角色当前状态不是 JSON 对象"
                && operation == "应用角色属性变更"
    ));
}

#[test]
fn attribute_state_rejects_corrupt_category_shape() {
    let mut state = json!({
        "explicit_attributes": [],
        "implicit_attributes": {},
        "social_attributes": {}
    });
    let error = apply_attribute_to_state(
        &mut state,
        "explicit",
        "appearance",
        "modify",
        json!("白衣"),
    )
    .expect_err("non-object category must fail");

    assert!(matches!(
        error,
        CharacterDbError::InvalidState { current, operation }
            if current.contains("explicit_attributes")
                && operation == "应用角色属性变更"
    ));
}

#[tokio::test]
async fn attribute_mutation_contract_preserves_invalid_argument_and_database_errors() {
    let disconnected = DatabaseConnection::default();
    let invalid_argument = record_attribute_event_in(
        &disconnected,
        CharacterAttributeEventInsertInput {
            profile_id: 7,
            chapter_number: 0,
            source_kind: "user".to_string(),
            category: "explicit".to_string(),
            key: "appearance".to_string(),
            operation: "modify".to_string(),
            old_value: json!("白衣"),
            new_value: json!("黑衣"),
            change_description: "服装变化".to_string(),
            cause: "剧情推进".to_string(),
            evidence: "角色换装".to_string(),
        },
    )
    .await
    .expect_err("invalid chapter must fail before database access");
    assert!(matches!(
        invalid_argument,
        CharacterDbError::InvalidArgument(message) if message == "章节号必须大于 0"
    ));

    let database_error = add_profile_attribute(
        &disconnected,
        CharacterAttributeValueChangeInput {
            profile_id: 7,
            chapter_number: 1,
            category: "explicit".to_string(),
            key: "appearance".to_string(),
            value: json!("白衣"),
            change_description: None,
            cause: "初次登场".to_string(),
            evidence: "正文描写".to_string(),
        },
    )
    .await
    .expect_err("transaction begin on a disconnected database must fail");
    assert!(matches!(database_error, CharacterDbError::Database(_)));
}

#[tokio::test]
async fn attribute_deletion_request_preserves_invalid_argument_and_database_errors() {
    let disconnected = DatabaseConnection::default();
    let invalid_argument = request_attribute_deletion_in(
        &disconnected,
        CharacterAttributeDeletionRequestInput {
            profile_id: 7,
            category: "explicit".to_string(),
            key: "appearance".to_string(),
            chapter_number: 0,
            reason: "设定失效".to_string(),
            evidence: "正文冲突".to_string(),
        },
    )
    .await
    .expect_err("invalid chapter must fail before database access");
    assert!(matches!(
        invalid_argument,
        CharacterDbError::InvalidArgument(message) if message == "章节号必须大于 0"
    ));

    let database_error = request_attribute_deletion(
        &disconnected,
        CharacterAttributeDeletionRequestInput {
            profile_id: 7,
            category: "explicit".to_string(),
            key: "appearance".to_string(),
            chapter_number: 1,
            reason: "设定失效".to_string(),
            evidence: "正文冲突".to_string(),
        },
    )
    .await
    .expect_err("transaction begin on a disconnected database must fail");
    assert!(matches!(database_error, CharacterDbError::Database(_)));
}

#[tokio::test]
async fn attribute_deletion_execution_preserves_invalid_argument_and_database_errors() {
    let disconnected = DatabaseConnection::default();
    let invalid_argument = execute_ready_attribute_deletion_in(
        &disconnected,
        CharacterAttributeDeletionExecutionInput {
            profile_id: 7,
            category: "explicit".to_string(),
            key: "appearance".to_string(),
            event_cause: "连续标记删除".to_string(),
            evidence: "正文冲突".to_string(),
            chapter_number: 0,
        },
    )
    .await
    .expect_err("invalid chapter must fail before database access");
    assert!(matches!(
        invalid_argument,
        CharacterDbError::InvalidArgument(message) if message == "章节号必须大于 0"
    ));

    let database_error = execute_ready_attribute_deletion(
        &disconnected,
        CharacterAttributeDeletionExecutionInput {
            profile_id: 7,
            category: "explicit".to_string(),
            key: "appearance".to_string(),
            event_cause: "连续标记删除".to_string(),
            evidence: "正文冲突".to_string(),
            chapter_number: 3,
        },
    )
    .await
    .expect_err("transaction begin on a disconnected database must fail");
    assert!(matches!(database_error, CharacterDbError::Database(_)));
}

#[tokio::test]
async fn implicit_review_contract_preserves_invalid_argument_and_database_errors() {
    let invalid_argument = require_positive_chapter_number(0, "应用隐性属性复审")
        .expect_err("non-positive chapter must fail");
    assert!(matches!(
        invalid_argument,
        CharacterDbError::InvalidArgument(message) if message == "应用隐性属性复审章节号必须大于 0"
    ));

    let disconnected = DatabaseConnection::default();
    let database_error = apply_implicit_review_decision_record(
        &disconnected,
        CharacterImplicitReviewDecisionRecordInput {
            profile_id: 7,
            chapter_number: 3,
            attribute_key: "personality".to_string(),
            current_value: json!("谨慎"),
            decision: "keep".to_string(),
            suggested_new_value: None,
            reasoning: String::new(),
            evidence_summary: String::new(),
        },
    )
    .await
    .expect_err("transaction begin on a disconnected database must fail");
    assert!(matches!(database_error, CharacterDbError::Database(_)));
}

#[test]
fn attribute_category_validator_preserves_typed_error() {
    let error = validate_category("unknown").expect_err("unsupported category must fail");
    assert!(matches!(
        error,
        CharacterDbError::InvalidArgument(message)
            if message == "属性分类必须是 explicit/implicit/social"
    ));
}
