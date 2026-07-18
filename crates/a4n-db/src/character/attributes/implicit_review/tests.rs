use super::super::types::CharacterImplicitReviewDecisionRecordInput;
use super::validation::validate_implicit_review_decision_record;
use crate::character::CharacterDbError;
use a4n_core::models::character::protagonist_profile;
use serde_json::{Value, json};

fn profile_with_implicit_state(state: Value) -> protagonist_profile::Model {
    let now = chrono::Utc::now().naive_utc();
    protagonist_profile::Model {
        id: 7,
        project_id: 3,
        name: "林岚".to_string(),
        gender: None,
        age: None,
        base_attributes: json!({}),
        current_state: state,
        character_arc: None,
        voice_fingerprint: None,
        appearance: None,
        background: None,
        personality: None,
        last_synced_chapter: 2,
        created_at: now,
        updated_at: now,
    }
}

fn profile() -> protagonist_profile::Model {
    profile_with_implicit_state(json!({
        "implicit_attributes": {
            "personality": "谨慎"
        }
    }))
}

fn input(decision: &str) -> CharacterImplicitReviewDecisionRecordInput {
    CharacterImplicitReviewDecisionRecordInput {
        profile_id: 7,
        chapter_number: Some(3),
        attribute_key: "personality".to_string(),
        current_value: json!("谨慎"),
        stats_attribute_key: "personality".to_string(),
        stats_current_value: json!("谨慎"),
        decision: decision.to_string(),
        suggested_new_value: None,
        reasoning: String::new(),
        evidence_summary: String::new(),
    }
}

#[test]
fn keep_decision_accepts_matching_profile_state_and_stats() {
    validate_implicit_review_decision_record(&profile(), &input("keep"))
        .expect("matching keep decision should pass");
}

#[test]
fn shared_validation_preserves_profile_key_and_stats_first_error_order() {
    let mut decision = input("keep");
    decision.profile_id = 8;
    decision.attribute_key = String::new();
    let profile_error = validate_implicit_review_decision_record(&profile(), &decision)
        .expect_err("profile mismatch must fail first");
    assert!(matches!(
        profile_error,
        CharacterDbError::InvalidArgument(message)
            if message == "复审决策 profile_id 不匹配: expected=7, actual=8"
    ));

    let mut decision = input("keep");
    decision.stats_attribute_key = "temperament".to_string();
    decision.stats_current_value = json!("冒进");
    let stats_key_error = validate_implicit_review_decision_record(&profile(), &decision)
        .expect_err("stats key mismatch must precede stats value mismatch");
    assert!(matches!(
        stats_key_error,
        CharacterDbError::InvalidArgument(message)
            if message == "复审统计 attribute_key 不匹配: expected=personality, actual=temperament"
    ));
}

#[test]
fn shared_validation_distinguishes_missing_and_stale_current_state() {
    let missing_error = validate_implicit_review_decision_record(
        &profile_with_implicit_state(json!({"implicit_attributes": {}})),
        &input("keep"),
    )
    .expect_err("missing implicit attribute must fail");
    assert!(matches!(
        missing_error,
        CharacterDbError::NotFound(message) if message == "隐性属性不存在: personality"
    ));

    let mut stale = input("keep");
    stale.current_value = json!("冒进");
    stale.stats_current_value = json!("冒进");
    let stale_error = validate_implicit_review_decision_record(&profile(), &stale)
        .expect_err("stale implicit attribute must fail");
    assert!(matches!(
        stale_error,
        CharacterDbError::InvalidState { current, operation }
            if current == "隐性属性 personality 当前值已变化"
                && operation == "应用隐性属性复审建议"
    ));
}

#[test]
fn action_validation_preserves_modify_delete_and_unknown_contracts() {
    let mut modify = input("modify");
    modify.suggested_new_value = Some(Value::Null);
    let null_error = validate_implicit_review_decision_record(&profile(), &modify)
        .expect_err("null modify value must fail");
    assert!(matches!(
        null_error,
        CharacterDbError::InvalidArgument(message)
            if message == "复审修改建议的新值不能为 null"
    ));

    modify.suggested_new_value = Some(json!("沉着"));
    let reason_error = validate_implicit_review_decision_record(&profile(), &modify)
        .expect_err("modify reason must be required");
    assert!(matches!(
        reason_error,
        CharacterDbError::InvalidArgument(message) if message == "复审修改建议缺少原因"
    ));

    let delete_error = validate_implicit_review_decision_record(&profile(), &input("delete"))
        .expect_err("delete reason must be required");
    assert!(matches!(
        delete_error,
        CharacterDbError::InvalidArgument(message) if message == "复审删除建议缺少原因"
    ));

    let unknown_error = validate_implicit_review_decision_record(&profile(), &input("archive"))
        .expect_err("unknown decision must fail");
    assert!(matches!(
        unknown_error,
        CharacterDbError::InvalidArgument(message)
            if message == "未知隐性属性复审决策: archive"
    ));
}
