use super::super::{
    deletion_marks::request_attribute_deletion_in,
    events::modify_profile_attribute_in,
    types::{
        AppliedCharacterImplicitDecisionAction, AppliedCharacterImplicitDecisionRecord,
        CharacterAttributeDeletionRequestInput, CharacterAttributeValueChangeInput,
        CharacterImplicitReviewDecisionRecordInput,
    },
};
use super::validation::{
    required_positive_chapter_number, validate_implicit_review_decision_record,
};
use crate::character::{CharacterDbError, require_profile};
use sea_orm::{ConnectionTrait, TransactionTrait};

pub async fn apply_implicit_review_decision_record<C>(
    db: &C,
    input: CharacterImplicitReviewDecisionRecordInput,
) -> Result<AppliedCharacterImplicitDecisionRecord, CharacterDbError>
where
    C: ConnectionTrait + TransactionTrait,
{
    let tx = db.begin().await?;
    let applied = apply_implicit_review_decision_record_in(&tx, input).await?;
    tx.commit().await?;
    Ok(applied)
}

pub async fn apply_implicit_review_decision_record_in<C>(
    db: &C,
    input: CharacterImplicitReviewDecisionRecordInput,
) -> Result<AppliedCharacterImplicitDecisionRecord, CharacterDbError>
where
    C: ConnectionTrait,
{
    let profile = require_profile(db, input.profile_id).await?;
    let chapter_number =
        required_positive_chapter_number(input.chapter_number, "应用隐性属性复审")?;
    validate_implicit_review_decision_record(&profile, &input)?;

    match input.decision.as_str() {
        "keep" => Ok(AppliedCharacterImplicitDecisionRecord {
            profile,
            action: AppliedCharacterImplicitDecisionAction::Keep,
            event: None,
            deletion_mark: None,
        }),
        "modify" => {
            let attribute_key = input.attribute_key.trim().to_string();
            let change_description = format!("隐性属性复审更新: {attribute_key}");
            let cause = format!("隐性属性复审: {}", input.reasoning.trim());
            let event = modify_profile_attribute_in(
                db,
                CharacterAttributeValueChangeInput {
                    profile_id: input.profile_id,
                    chapter_number,
                    category: "implicit".to_string(),
                    key: attribute_key,
                    value: input.suggested_new_value.ok_or_else(|| {
                        CharacterDbError::InvalidArgument("复审修改建议缺少新值".to_string())
                    })?,
                    change_description: Some(change_description),
                    cause,
                    evidence: input.evidence_summary.trim().to_string(),
                },
            )
            .await?;
            Ok(AppliedCharacterImplicitDecisionRecord {
                profile,
                action: AppliedCharacterImplicitDecisionAction::Modify,
                event: Some(event),
                deletion_mark: None,
            })
        }
        "delete" => {
            let mark = request_attribute_deletion_in(
                db,
                CharacterAttributeDeletionRequestInput {
                    profile_id: input.profile_id,
                    category: "implicit".to_string(),
                    key: input.attribute_key.trim().to_string(),
                    chapter_number,
                    reason: input.reasoning.trim().to_string(),
                    evidence: input.evidence_summary.trim().to_string(),
                },
            )
            .await?;
            Ok(AppliedCharacterImplicitDecisionRecord {
                profile,
                action: AppliedCharacterImplicitDecisionAction::MarkDeletion,
                event: None,
                deletion_mark: Some(mark),
            })
        }
        other => Err(CharacterDbError::InvalidArgument(format!(
            "未知隐性属性复审决策: {other}"
        ))),
    }
}
