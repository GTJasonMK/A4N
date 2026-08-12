use super::{
    CharacterSyncAttributeEventRecordInput, CharacterSyncProtectedAttribute,
    CharacterSyncWriteStepRecordInput, CharacterSyncWriteSummary,
};
use crate::character::{
    AppliedCharacterImplicitDecisionAction, CharacterAttributeDeletionExecutionInput,
    CharacterAttributeDeletionRequestInput, CharacterDbError,
    CharacterImplicitReviewDecisionRecordInput, add_profile_attribute_from_source_in,
    apply_implicit_review_decision_record_from_source_in,
    execute_ready_attribute_deletion_from_source_in, modify_profile_attribute_from_source_in,
    record_behavior_in, request_attribute_deletion_from_source_in,
};
use a4n_core::models::character::attribute_event::SOURCE_KIND_AI_SYNC;
use a4n_domain::character::CHARACTER_DELETION_THRESHOLD;
use sea_orm::ConnectionTrait;

use crate::character::CharacterAttributeOwnership;

pub(super) async fn apply_character_sync_write_steps_in<C>(
    db: &C,
    profile_id: i64,
    steps: Vec<CharacterSyncWriteStepRecordInput>,
    ownership: &CharacterAttributeOwnership,
) -> Result<CharacterSyncWriteSummary, CharacterDbError>
where
    C: ConnectionTrait,
{
    let mut summary = CharacterSyncWriteSummary::default();
    for step in steps {
        match step {
            CharacterSyncWriteStepRecordInput::AttributeEvent(event) => {
                if event.change.profile_id != profile_id {
                    return Err(CharacterDbError::InvalidArgument(format!(
                        "角色同步属性事件 profile_id 不匹配: expected={}, actual={}",
                        profile_id, event.change.profile_id
                    )));
                }
                if ownership.is_protected(
                    event.change.chapter_number,
                    &event.change.category,
                    &event.change.key,
                ) {
                    record_protected_attribute(
                        &mut summary,
                        event.change.chapter_number,
                        &event.change.category,
                        &event.change.key,
                        &event.operation,
                    );
                    continue;
                }
                let recorded = record_character_sync_attribute_event(db, event).await?;
                summary.attribute_event_ids.push(recorded.id);
                summary.changes_applied += 1;
                if recorded.from_classification {
                    summary.classification_suggestions_applied += 1;
                }
            }
            CharacterSyncWriteStepRecordInput::Behavior(behavior) => {
                if behavior.profile_id != profile_id {
                    return Err(CharacterDbError::InvalidArgument(format!(
                        "角色同步行为记录 profile_id 不匹配: expected={}, actual={}",
                        profile_id, behavior.profile_id
                    )));
                }
                record_behavior_in(db, behavior).await?;
                summary.behaviors_recorded += 1;
            }
            CharacterSyncWriteStepRecordInput::DeletionCandidate(candidate) => {
                if candidate.profile_id != profile_id {
                    return Err(CharacterDbError::InvalidArgument(format!(
                        "角色同步删除候选 profile_id 不匹配: expected={}, actual={}",
                        profile_id, candidate.profile_id
                    )));
                }
                if ownership.is_protected(
                    candidate.chapter_number,
                    &candidate.category,
                    &candidate.key,
                ) {
                    record_protected_attribute(
                        &mut summary,
                        candidate.chapter_number,
                        &candidate.category,
                        &candidate.key,
                        "delete",
                    );
                    continue;
                }
                apply_character_sync_deletion_candidate(db, candidate, &mut summary).await?;
            }
            CharacterSyncWriteStepRecordInput::ImplicitReview(decision) => {
                if decision.profile_id != profile_id {
                    return Err(CharacterDbError::InvalidArgument(format!(
                        "角色同步隐性复审 profile_id 不匹配: expected={}, actual={}",
                        profile_id, decision.profile_id
                    )));
                }
                if implicit_review_mutates_protected_attribute(&decision, ownership, &mut summary)?
                {
                    continue;
                }
                apply_character_sync_implicit_review_decision(db, decision, &mut summary).await?;
            }
        }
    }
    Ok(summary)
}

fn record_protected_attribute(
    summary: &mut CharacterSyncWriteSummary,
    chapter_number: i32,
    category: &str,
    key: &str,
    operation: &str,
) {
    let protected = CharacterSyncProtectedAttribute {
        chapter_number,
        category: category.to_string(),
        key: key.to_string(),
        operation: operation.to_string(),
    };
    if !summary.protected_attributes.contains(&protected) {
        summary.protected_attributes.push(protected);
    }
}

fn implicit_review_mutates_protected_attribute(
    decision: &CharacterImplicitReviewDecisionRecordInput,
    ownership: &CharacterAttributeOwnership,
    summary: &mut CharacterSyncWriteSummary,
) -> Result<bool, CharacterDbError> {
    if decision.decision == "keep" {
        return Ok(false);
    }
    if !matches!(decision.decision.as_str(), "modify" | "delete") {
        return Ok(false);
    }
    let chapter_number = decision.chapter_number;
    if !ownership.is_protected(chapter_number, "implicit", &decision.attribute_key) {
        return Ok(false);
    }
    record_protected_attribute(
        summary,
        chapter_number,
        "implicit",
        &decision.attribute_key,
        &decision.decision,
    );
    Ok(true)
}

struct RecordedCharacterSyncAttributeEvent {
    id: i64,
    from_classification: bool,
}

async fn record_character_sync_attribute_event<C>(
    db: &C,
    event: CharacterSyncAttributeEventRecordInput,
) -> Result<RecordedCharacterSyncAttributeEvent, CharacterDbError>
where
    C: ConnectionTrait,
{
    let from_classification = event.from_classification;
    let operation = event.operation.trim();
    let recorded = match operation {
        "add" => {
            add_profile_attribute_from_source_in(db, event.change, SOURCE_KIND_AI_SYNC).await?
        }
        "modify" => {
            modify_profile_attribute_from_source_in(db, event.change, SOURCE_KIND_AI_SYNC).await?
        }
        other => {
            return Err(CharacterDbError::InvalidArgument(format!(
                "属性事件草稿不允许 operation={other}"
            )));
        }
    };
    Ok(RecordedCharacterSyncAttributeEvent {
        id: recorded.id,
        from_classification,
    })
}

async fn apply_character_sync_deletion_candidate<C>(
    db: &C,
    candidate: CharacterAttributeDeletionRequestInput,
    summary: &mut CharacterSyncWriteSummary,
) -> Result<(), CharacterDbError>
where
    C: ConnectionTrait,
{
    let evidence = candidate.evidence.clone();
    let chapter_number = candidate.chapter_number;
    let mark =
        request_attribute_deletion_from_source_in(db, candidate, SOURCE_KIND_AI_SYNC).await?;
    summary.deletions_marked += 1;

    if !mark.is_executed && mark.consecutive_count >= CHARACTER_DELETION_THRESHOLD {
        let event_cause = format!("连续{}次标记删除", mark.consecutive_count);
        let (event, _) = execute_ready_attribute_deletion_from_source_in(
            db,
            CharacterAttributeDeletionExecutionInput {
                profile_id: mark.profile_id,
                category: mark.category.clone(),
                key: mark.key.clone(),
                event_cause,
                evidence,
                chapter_number,
            },
            SOURCE_KIND_AI_SYNC,
        )
        .await?;
        summary.attribute_event_ids.push(event.id);
        summary.deletions_executed += 1;
        summary.changes_applied += 1;
    }

    Ok(())
}

async fn apply_character_sync_implicit_review_decision<C>(
    db: &C,
    decision: CharacterImplicitReviewDecisionRecordInput,
    summary: &mut CharacterSyncWriteSummary,
) -> Result<(), CharacterDbError>
where
    C: ConnectionTrait,
{
    let applied =
        apply_implicit_review_decision_record_from_source_in(db, decision, SOURCE_KIND_AI_SYNC)
            .await?;

    match applied.action {
        AppliedCharacterImplicitDecisionAction::Keep => {}
        AppliedCharacterImplicitDecisionAction::Modify => {
            let event = applied.event.ok_or_else(|| {
                CharacterDbError::Internal("隐性属性复审修改缺少属性事件".to_string())
            })?;
            summary.attribute_event_ids.push(event.id);
            summary.changes_applied += 1;
            summary.implicit_reviews_applied += 1;
        }
        AppliedCharacterImplicitDecisionAction::MarkDeletion => {
            if applied.deletion_mark.is_none() {
                return Err(CharacterDbError::Internal(
                    "隐性属性复审删除缺少删除标记".to_string(),
                ));
            }
            summary.deletions_marked += 1;
            summary.implicit_reviews_applied += 1;
        }
    }
    Ok(())
}
