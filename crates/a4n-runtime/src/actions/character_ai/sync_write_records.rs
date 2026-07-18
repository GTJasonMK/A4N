use super::json_utils::{
    optional_string, required_category, required_i32, required_string, required_value,
};
use super::sync_write::{AttributeEventDraft, SyncWriteStepKind, sync_write_steps};
use crate::actions::character_implicit_review::ImplicitUpdateDecisionSummary;
use a4n_orchestrator::character::{
    CharacterAttributeDeletionRequestInput, CharacterAttributeValueChangeInput,
    CharacterImplicitReviewDecisionRecordInput, CharacterSyncAttributeEventRecordInput,
    CharacterSyncWriteStepRecordInput, InsertBehaviorRecordInput,
};
use serde_json::Value;

pub(super) struct BuildCharacterSyncWriteStepsInput<'a> {
    pub(super) profile_id: i64,
    pub(super) chapter_numbers: &'a [i32],
    pub(super) attribute_events: &'a [AttributeEventDraft],
    pub(super) behaviors: &'a [Value],
    pub(super) deletion_candidates: &'a [Value],
    pub(super) implicit_review_decisions: &'a [ImplicitUpdateDecisionSummary],
    pub(super) target_chapter: i32,
}

pub(super) fn build_character_sync_write_step_records(
    input: BuildCharacterSyncWriteStepsInput<'_>,
) -> Result<Vec<CharacterSyncWriteStepRecordInput>, String> {
    let BuildCharacterSyncWriteStepsInput {
        profile_id,
        chapter_numbers,
        attribute_events,
        behaviors,
        deletion_candidates,
        implicit_review_decisions,
        target_chapter,
    } = input;
    let steps = sync_write_steps(
        chapter_numbers,
        attribute_events,
        behaviors,
        deletion_candidates,
        implicit_review_decisions.len(),
        target_chapter,
    )?;
    let mut records = Vec::with_capacity(steps.len());
    for step in steps {
        let record = match step.kind {
            SyncWriteStepKind::AttributeEvent => {
                let event = attribute_events
                    .get(step.index)
                    .ok_or_else(|| format!("角色同步写入步骤缺少属性事件: {}", step.index))?;
                CharacterSyncWriteStepRecordInput::AttributeEvent(attribute_event_record_input(
                    profile_id, event,
                ))
            }
            SyncWriteStepKind::Behavior => {
                let behavior = behaviors
                    .get(step.index)
                    .ok_or_else(|| format!("角色同步写入步骤缺少行为记录: {}", step.index))?;
                CharacterSyncWriteStepRecordInput::Behavior(behavior_record_input(
                    profile_id, behavior,
                )?)
            }
            SyncWriteStepKind::DeletionCandidate => {
                let candidate = deletion_candidates
                    .get(step.index)
                    .ok_or_else(|| format!("角色同步写入步骤缺少删除候选: {}", step.index))?;
                CharacterSyncWriteStepRecordInput::DeletionCandidate(
                    deletion_candidate_record_input(profile_id, candidate)?,
                )
            }
            SyncWriteStepKind::ImplicitReview => {
                let decision = implicit_review_decisions
                    .get(step.index)
                    .ok_or_else(|| format!("角色同步写入步骤缺少隐性复审: {}", step.index))?;
                CharacterSyncWriteStepRecordInput::ImplicitReview(implicit_review_record_input(
                    profile_id,
                    target_chapter,
                    decision,
                ))
            }
        };
        records.push(record);
    }
    Ok(records)
}

fn attribute_event_record_input(
    profile_id: i64,
    event: &AttributeEventDraft,
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
) -> Result<InsertBehaviorRecordInput, String> {
    let chapter = required_i32(behavior, "chapter_number")?;
    let behavior_type = required_string(behavior, "behavior_type")?;
    let description = required_string(behavior, "behavior_description")?;
    let original_text = required_string(behavior, "original_text")?;
    let behavior_tags = required_value(behavior, "behavior_tags")?.clone();
    let classification_results = behavior
        .get("classification_results")
        .cloned()
        .ok_or_else(|| "行为记录缺少 classification_results".to_string())?;
    let related_characters = Some(required_value(behavior, "related_characters")?.clone());
    let impact_analysis = optional_string(behavior, "impact_analysis");

    Ok(InsertBehaviorRecordInput {
        profile_id,
        chapter_number: chapter,
        behavior_type,
        behavior_description: description,
        original_text,
        behavior_tags,
        classification_results,
        related_characters,
        impact_analysis,
    })
}

fn deletion_candidate_record_input(
    profile_id: i64,
    candidate: &Value,
) -> Result<CharacterAttributeDeletionRequestInput, String> {
    let chapter = required_i32(candidate, "chapter_number")?;
    let category = required_category(candidate, "category")?;
    let key = required_string(candidate, "key")?;
    let reason = required_string(candidate, "reason")?;
    let evidence = required_string(candidate, "evidence")?;

    Ok(CharacterAttributeDeletionRequestInput {
        profile_id,
        category,
        key,
        chapter_number: chapter,
        reason,
        evidence,
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
