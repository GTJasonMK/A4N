use super::{
    InsertBehaviorRecordInput, record_behavior_in, validate_behavior_classification_results,
    validation::validate_behavior_record_input,
};
use crate::character::CharacterDbError;
use serde_json::json;

#[test]
fn behavior_classification_accepts_strict_values() {
    validate_behavior_classification_results(&json!({
        "谨慎": "conform",
        "冲动": "non-conform"
    }))
    .expect("strict classifications should pass");

    validate_behavior_classification_results(&json!({}))
        .expect("empty classifications should pass");
}

#[test]
fn behavior_classification_contract_errors_are_structured() {
    let shape_error = validate_behavior_classification_results(&json!([]))
        .expect_err("non-object classification results must fail");
    assert!(matches!(
        shape_error,
        CharacterDbError::InvalidArgument(message) if message == "行为分类结果必须是 JSON 对象"
    ));

    let value_error = validate_behavior_classification_results(&json!({
        "personality": "unknown"
    }))
    .expect_err("unsupported classification result must fail");
    assert!(matches!(
        value_error,
        CharacterDbError::InvalidArgument(message)
            if message == "行为分类结果 personality 必须是 conform/non-conform"
    ));

    let empty_key_error = validate_behavior_classification_results(&json!({
        " ": "conform"
    }))
    .expect_err("blank classification key must fail");
    assert!(matches!(
        empty_key_error,
        CharacterDbError::InvalidArgument(message)
            if message == "行为分类结果包含空属性 key"
    ));

    let non_string_error = validate_behavior_classification_results(&json!({
        "personality": true
    }))
    .expect_err("non-string classification result must fail");
    assert!(matches!(
        non_string_error,
        CharacterDbError::InvalidArgument(message)
            if message == "行为分类结果 personality 必须是 conform/non-conform"
    ));
}

#[test]
fn behavior_record_validation_normalizes_only_required_text_fields() {
    let validated = validate_behavior_record_input(InsertBehaviorRecordInput {
        profile_id: 7,
        chapter_number: 3,
        source_kind: "ai_sync".to_string(),
        behavior_type: " dialogue ".to_string(),
        behavior_description: " 回应 ".to_string(),
        original_text: " 收到 ".to_string(),
        behavior_tags: json!(["应答"]),
        classification_results: json!({"谨慎": "conform"}),
        related_characters: Some(json!([9])),
        impact_analysis: Some(" 保留原空白 ".to_string()),
    })
    .expect("valid behavior record should normalize");

    assert_eq!(validated.behavior_type, "dialogue");
    assert_eq!(validated.behavior_description, "回应");
    assert_eq!(validated.original_text, "收到");
    assert_eq!(validated.impact_analysis.as_deref(), Some(" 保留原空白 "));
}

#[tokio::test]
async fn behavior_validation_precedes_database_access() {
    let disconnected = sea_orm::DatabaseConnection::default();
    let error = record_behavior_in(
        &disconnected,
        InsertBehaviorRecordInput {
            profile_id: 7,
            chapter_number: 0,
            source_kind: "ai_sync".to_string(),
            behavior_type: "dialogue".to_string(),
            behavior_description: "回应".to_string(),
            original_text: "收到".to_string(),
            behavior_tags: json!([]),
            classification_results: json!({}),
            related_characters: None,
            impact_analysis: None,
        },
    )
    .await
    .expect_err("invalid chapter must fail before database access");
    assert!(matches!(
        error,
        CharacterDbError::InvalidArgument(message) if message == "行为记录章节号必须大于 0"
    ));
}
