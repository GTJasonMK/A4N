use a4n_core::error::AppError;
use serde_json::Value;

use super::super::{
    CharacterAttributeDeletionRequestInput, CharacterAttributeValueChangeInput,
    CharacterImplicitReviewDecisionRecordInput, CharacterSyncAttributeEventDraft,
    CharacterSyncAttributeEventRecordInput, CharacterSyncWriteStepRecordInput,
    ImplicitUpdateDecisionSummary, InsertBehaviorRecordInput, optional_string, required_category,
    required_i32, required_string, required_value,
};

pub(super) struct BuildCharacterSyncWriteStepsInput<'a> {
    pub(super) profile_id: i64,
    pub(super) chapter_numbers: &'a [i32],
    pub(super) attribute_events: &'a [CharacterSyncAttributeEventDraft],
    pub(super) behaviors: &'a [Value],
    pub(super) deletion_candidates: &'a [Value],
    pub(super) implicit_review_decisions: &'a [ImplicitUpdateDecisionSummary],
    pub(super) target_chapter: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SyncWriteStepKind {
    AttributeEvent,
    Behavior,
    DeletionCandidate,
    ImplicitReview,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct SyncWriteStep {
    chapter_number: i32,
    kind: SyncWriteStepKind,
    index: usize,
}

pub(super) fn build_character_sync_write_step_records(
    input: BuildCharacterSyncWriteStepsInput<'_>,
) -> Result<Vec<CharacterSyncWriteStepRecordInput>, AppError> {
    let steps = sync_write_steps(
        input.chapter_numbers,
        input.attribute_events,
        input.behaviors,
        input.deletion_candidates,
        input.implicit_review_decisions.len(),
        input.target_chapter,
    )?;
    let mut records = Vec::with_capacity(steps.len());
    for step in steps {
        let record = match step.kind {
            SyncWriteStepKind::AttributeEvent => {
                let event = input.attribute_events.get(step.index).ok_or_else(|| {
                    invalid_argument(format!("角色同步写入步骤缺少属性事件: {}", step.index))
                })?;
                CharacterSyncWriteStepRecordInput::AttributeEvent(attribute_event_record_input(
                    input.profile_id,
                    event,
                ))
            }
            SyncWriteStepKind::Behavior => {
                let behavior = input.behaviors.get(step.index).ok_or_else(|| {
                    invalid_argument(format!("角色同步写入步骤缺少行为记录: {}", step.index))
                })?;
                CharacterSyncWriteStepRecordInput::Behavior(behavior_record_input(
                    input.profile_id,
                    behavior,
                )?)
            }
            SyncWriteStepKind::DeletionCandidate => {
                let candidate = input.deletion_candidates.get(step.index).ok_or_else(|| {
                    invalid_argument(format!("角色同步写入步骤缺少删除候选: {}", step.index))
                })?;
                CharacterSyncWriteStepRecordInput::DeletionCandidate(
                    deletion_candidate_record_input(input.profile_id, candidate)?,
                )
            }
            SyncWriteStepKind::ImplicitReview => {
                let decision =
                    input
                        .implicit_review_decisions
                        .get(step.index)
                        .ok_or_else(|| {
                            invalid_argument(format!(
                                "角色同步写入步骤缺少隐性复审: {}",
                                step.index
                            ))
                        })?;
                CharacterSyncWriteStepRecordInput::ImplicitReview(implicit_review_record_input(
                    input.profile_id,
                    input.target_chapter,
                    decision,
                ))
            }
        };
        records.push(record);
    }
    Ok(records)
}

fn sync_write_steps(
    chapter_numbers: &[i32],
    attribute_events: &[CharacterSyncAttributeEventDraft],
    behaviors: &[Value],
    deletion_candidates: &[Value],
    implicit_review_decision_count: usize,
    target_chapter: i32,
) -> Result<Vec<SyncWriteStep>, AppError> {
    let mut steps = Vec::new();
    for chapter_number in chapter_numbers {
        for (index, event) in attribute_events.iter().enumerate() {
            if event.chapter_number == *chapter_number {
                steps.push(SyncWriteStep {
                    chapter_number: *chapter_number,
                    kind: SyncWriteStepKind::AttributeEvent,
                    index,
                });
            }
        }
        for (index, behavior) in behaviors.iter().enumerate() {
            if required_i32(behavior, "chapter_number").map_err(invalid_argument)?
                == *chapter_number
            {
                steps.push(SyncWriteStep {
                    chapter_number: *chapter_number,
                    kind: SyncWriteStepKind::Behavior,
                    index,
                });
            }
        }
        for (index, candidate) in deletion_candidates.iter().enumerate() {
            if required_i32(candidate, "chapter_number").map_err(invalid_argument)?
                == *chapter_number
            {
                steps.push(SyncWriteStep {
                    chapter_number: *chapter_number,
                    kind: SyncWriteStepKind::DeletionCandidate,
                    index,
                });
            }
        }
        if *chapter_number == target_chapter {
            for index in 0..implicit_review_decision_count {
                steps.push(SyncWriteStep {
                    chapter_number: *chapter_number,
                    kind: SyncWriteStepKind::ImplicitReview,
                    index,
                });
            }
        }
    }
    Ok(steps)
}

fn attribute_event_record_input(
    profile_id: i64,
    event: &CharacterSyncAttributeEventDraft,
) -> CharacterSyncAttributeEventRecordInput {
    CharacterSyncAttributeEventRecordInput {
        operation: event.operation.clone(),
        change: CharacterAttributeValueChangeInput {
            profile_id,
            chapter_number: event.chapter_number,
            category: event.category.clone(),
            key: event.key.clone(),
            value: event.new_value.clone(),
            change_description: Some(event.change_description.clone()),
            cause: event.cause.clone(),
            evidence: event.evidence.clone(),
        },
        from_classification: event.from_classification,
    }
}

fn behavior_record_input(
    profile_id: i64,
    behavior: &Value,
) -> Result<InsertBehaviorRecordInput, AppError> {
    Ok(InsertBehaviorRecordInput {
        profile_id,
        chapter_number: required_i32(behavior, "chapter_number").map_err(invalid_argument)?,
        behavior_type: required_string(behavior, "behavior_type").map_err(invalid_argument)?,
        behavior_description: required_string(behavior, "behavior_description")
            .map_err(invalid_argument)?,
        original_text: required_string(behavior, "original_text").map_err(invalid_argument)?,
        behavior_tags: required_value(behavior, "behavior_tags")
            .map_err(invalid_argument)?
            .clone(),
        classification_results: behavior
            .get("classification_results")
            .cloned()
            .ok_or_else(|| invalid_argument("行为记录缺少 classification_results".to_string()))?,
        related_characters: Some(
            required_value(behavior, "related_characters")
                .map_err(invalid_argument)?
                .clone(),
        ),
        impact_analysis: optional_string(behavior, "impact_analysis"),
    })
}

fn deletion_candidate_record_input(
    profile_id: i64,
    candidate: &Value,
) -> Result<CharacterAttributeDeletionRequestInput, AppError> {
    Ok(CharacterAttributeDeletionRequestInput {
        profile_id,
        category: required_category(candidate, "category").map_err(invalid_argument)?,
        key: required_string(candidate, "key").map_err(invalid_argument)?,
        chapter_number: required_i32(candidate, "chapter_number").map_err(invalid_argument)?,
        reason: required_string(candidate, "reason").map_err(invalid_argument)?,
        evidence: required_string(candidate, "evidence").map_err(invalid_argument)?,
    })
}

fn implicit_review_record_input(
    profile_id: i64,
    target_chapter: i32,
    decision: &ImplicitUpdateDecisionSummary,
) -> CharacterImplicitReviewDecisionRecordInput {
    CharacterImplicitReviewDecisionRecordInput {
        profile_id,
        chapter_number: Some(target_chapter),
        attribute_key: decision.attribute_key.clone(),
        current_value: decision.current_value.clone(),
        stats_attribute_key: decision.stats.attribute_key.clone(),
        stats_current_value: decision.stats.current_value.clone(),
        decision: decision.decision.clone(),
        suggested_new_value: decision.suggested_new_value.clone(),
        reasoning: decision.reasoning.clone(),
        evidence_summary: decision.evidence_summary.clone(),
    }
}

fn invalid_argument(message: String) -> AppError {
    AppError::InvalidArgument(message)
}
