use super::{
    CharacterSyncAttributeEventRecordInput, CharacterSyncWriteStepRecordInput,
    CharacterSyncWriteSummary,
};
use crate::character::{
    AppliedCharacterImplicitDecisionAction, CharacterAttributeDeletionExecutionInput,
    CharacterAttributeDeletionRequestInput, CharacterDbError,
    CharacterImplicitReviewDecisionRecordInput, DELETION_THRESHOLD, add_profile_attribute_in,
    apply_implicit_review_decision_record_in, execute_ready_attribute_deletion_in,
    modify_profile_attribute_in, record_behavior_in, request_attribute_deletion_in,
};
use sea_orm::ConnectionTrait;

pub async fn apply_character_sync_write_steps_in<C>(
    db: &C,
    profile_id: i64,
    steps: Vec<CharacterSyncWriteStepRecordInput>,
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
                apply_character_sync_deletion_candidate(db, candidate, &mut summary).await?;
            }
            CharacterSyncWriteStepRecordInput::ImplicitReview(decision) => {
                if decision.profile_id != profile_id {
                    return Err(CharacterDbError::InvalidArgument(format!(
                        "角色同步隐性复审 profile_id 不匹配: expected={}, actual={}",
                        profile_id, decision.profile_id
                    )));
                }
                apply_character_sync_implicit_review_decision(db, decision, &mut summary).await?;
            }
        }
    }
    Ok(summary)
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
        "add" => add_profile_attribute_in(db, event.change).await?,
        "modify" => modify_profile_attribute_in(db, event.change).await?,
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
    let mark = request_attribute_deletion_in(db, candidate).await?;
    summary.deletions_marked += 1;

    if !mark.is_executed && mark.consecutive_count >= DELETION_THRESHOLD {
        let event_cause = format!("连续{}次标记删除", mark.consecutive_count);
        let (event, _) = execute_ready_attribute_deletion_in(
            db,
            CharacterAttributeDeletionExecutionInput {
                profile_id: mark.profile_id,
                category: mark.category.clone(),
                key: mark.key.clone(),
                event_cause,
                evidence,
                chapter_number,
            },
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
    let applied = apply_implicit_review_decision_record_in(db, decision).await?;

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
