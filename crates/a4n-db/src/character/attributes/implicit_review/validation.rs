use super::super::{
    state::attribute_value_in_state, types::CharacterImplicitReviewDecisionRecordInput,
};
use crate::character::CharacterDbError;
use a4n_core::models::character::protagonist_profile;

pub(super) fn validate_implicit_review_decision_record(
    profile: &protagonist_profile::Model,
    input: &CharacterImplicitReviewDecisionRecordInput,
) -> Result<(), CharacterDbError> {
    if input.profile_id != profile.id {
        return Err(CharacterDbError::InvalidArgument(format!(
            "复审决策 profile_id 不匹配: expected={}, actual={}",
            profile.id, input.profile_id
        )));
    }
    let attribute_key = input.attribute_key.trim();
    if attribute_key.is_empty() {
        return Err(CharacterDbError::InvalidArgument(
            "隐性属性 key 不能为空".to_string(),
        ));
    }
    let current_value = attribute_value_in_state(&profile.current_state, "implicit", attribute_key)
        .ok_or_else(|| CharacterDbError::NotFound(format!("隐性属性不存在: {attribute_key}")))?;
    if current_value != input.current_value {
        return Err(CharacterDbError::InvalidState {
            current: format!("隐性属性 {attribute_key} 当前值已变化"),
            operation: "应用隐性属性复审建议".to_string(),
        });
    }

    validate_implicit_review_action(input)
}

fn validate_implicit_review_action(
    input: &CharacterImplicitReviewDecisionRecordInput,
) -> Result<(), CharacterDbError> {
    match input.decision.as_str() {
        "keep" => Ok(()),
        "modify" => {
            let new_value = input.suggested_new_value.as_ref().ok_or_else(|| {
                CharacterDbError::InvalidArgument("复审修改建议缺少新值".to_string())
            })?;
            if new_value.is_null() {
                return Err(CharacterDbError::InvalidArgument(
                    "复审修改建议的新值不能为 null".to_string(),
                ));
            }
            validate_review_reason_and_evidence(input, "修改")
        }
        "delete" => validate_review_reason_and_evidence(input, "删除"),
        other => Err(CharacterDbError::InvalidArgument(format!(
            "未知隐性属性复审决策: {other}"
        ))),
    }
}

fn validate_review_reason_and_evidence(
    input: &CharacterImplicitReviewDecisionRecordInput,
    action: &str,
) -> Result<(), CharacterDbError> {
    if input.reasoning.trim().is_empty() {
        return Err(CharacterDbError::InvalidArgument(format!(
            "复审{action}建议缺少原因"
        )));
    }
    if input.evidence_summary.trim().is_empty() {
        return Err(CharacterDbError::InvalidArgument(format!(
            "复审{action}建议缺少证据"
        )));
    }
    Ok(())
}

pub(in crate::character::attributes) fn require_positive_chapter_number(
    chapter_number: i32,
    label: &str,
) -> Result<i32, CharacterDbError> {
    if chapter_number <= 0 {
        return Err(CharacterDbError::InvalidArgument(format!(
            "{label}章节号必须大于 0"
        )));
    }
    Ok(chapter_number)
}
