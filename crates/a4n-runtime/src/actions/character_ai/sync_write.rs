use crate::actions::character_implicit_review::ImplicitUpdateDecisionSummary;
use crate::actions::character_implicit_review::{
    AppliedImplicitDecisionAction, apply_implicit_update_decision_in,
};
use crate::state::AppState;
use a4n_core::models::character::attribute_event;
use a4n_services::character::DELETION_THRESHOLD;
use sea_orm::ConnectionTrait;
use serde_json::Value;

use super::json_utils::{
    optional_string, required_category, required_i32, required_string, required_value,
};

#[derive(Debug, Clone)]
pub(super) struct AttributeEventDraft {
    pub(super) chapter_number: i32,
    pub(super) sequence: usize,
    pub(super) stage: u8,
    pub(super) category: String,
    pub(super) key: String,
    pub(super) operation: String,
    pub(super) old_value: Value,
    pub(super) new_value: Value,
    pub(super) change_description: String,
    pub(super) cause: String,
    pub(super) evidence: String,
    pub(super) from_classification: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(super) struct SyncWriteSummary {
    pub(super) changes_applied: usize,
    pub(super) classification_suggestions_applied: usize,
    pub(super) behaviors_recorded: usize,
    pub(super) deletions_marked: usize,
    pub(super) deletions_executed: usize,
    pub(super) implicit_reviews_applied: usize,
    pub(super) attribute_event_ids: Vec<i64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum SyncWriteStepKind {
    AttributeEvent,
    Behavior,
    DeletionCandidate,
    ImplicitReview,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct SyncWriteStep {
    pub(super) chapter_number: i32,
    pub(super) kind: SyncWriteStepKind,
    pub(super) index: usize,
}

pub(super) fn sync_write_steps(
    chapter_numbers: &[i32],
    attribute_events: &[AttributeEventDraft],
    behaviors: &[Value],
    deletion_candidates: &[Value],
    implicit_review_decision_count: usize,
    target_chapter: i32,
) -> Result<Vec<SyncWriteStep>, String> {
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
            if required_i32(behavior, "chapter_number")? == *chapter_number {
                steps.push(SyncWriteStep {
                    chapter_number: *chapter_number,
                    kind: SyncWriteStepKind::Behavior,
                    index,
                });
            }
        }

        for (index, candidate) in deletion_candidates.iter().enumerate() {
            if required_i32(candidate, "chapter_number")? == *chapter_number {
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

pub(super) struct ApplySyncWritesInput<'a, C>
where
    C: ConnectionTrait,
{
    pub(super) state: &'a AppState,
    pub(super) db: &'a C,
    pub(super) profile_id: i64,
    pub(super) chapter_numbers: &'a [i32],
    pub(super) attribute_events: &'a [AttributeEventDraft],
    pub(super) behaviors: &'a [Value],
    pub(super) deletion_candidates: &'a [Value],
    pub(super) implicit_review_decisions: &'a [ImplicitUpdateDecisionSummary],
    pub(super) target_chapter: i32,
}

pub(super) async fn apply_sync_writes_by_chapter<C>(
    input: ApplySyncWritesInput<'_, C>,
) -> Result<SyncWriteSummary, String>
where
    C: ConnectionTrait,
{
    let ApplySyncWritesInput {
        state,
        db,
        profile_id,
        chapter_numbers,
        attribute_events,
        behaviors,
        deletion_candidates,
        implicit_review_decisions,
        target_chapter,
    } = input;
    let mut summary = SyncWriteSummary::default();
    let steps = sync_write_steps(
        chapter_numbers,
        attribute_events,
        behaviors,
        deletion_candidates,
        implicit_review_decisions.len(),
        target_chapter,
    )?;
    for step in steps {
        match step.kind {
            SyncWriteStepKind::AttributeEvent => {
                let event = attribute_events
                    .get(step.index)
                    .ok_or_else(|| format!("角色同步写入步骤缺少属性事件: {}", step.index))?;
                let recorded = record_attribute_event_draft(state, db, profile_id, event).await?;
                summary.attribute_event_ids.push(recorded.id);
                summary.changes_applied += 1;
                if event.from_classification {
                    summary.classification_suggestions_applied += 1;
                }
            }
            SyncWriteStepKind::Behavior => {
                let behavior = behaviors
                    .get(step.index)
                    .ok_or_else(|| format!("角色同步写入步骤缺少行为记录: {}", step.index))?;
                record_behavior_value(state, db, profile_id, behavior).await?;
                summary.behaviors_recorded += 1;
            }
            SyncWriteStepKind::DeletionCandidate => {
                let candidate = deletion_candidates
                    .get(step.index)
                    .ok_or_else(|| format!("角色同步写入步骤缺少删除候选: {}", step.index))?;
                apply_deletion_candidate(state, db, profile_id, candidate, &mut summary).await?;
            }
            SyncWriteStepKind::ImplicitReview => {
                let decision = implicit_review_decisions
                    .get(step.index)
                    .ok_or_else(|| format!("角色同步写入步骤缺少隐性复审: {}", step.index))?;
                apply_implicit_review_decision(
                    state,
                    db,
                    profile_id,
                    target_chapter,
                    decision,
                    &mut summary,
                )
                .await?;
            }
        }
    }

    Ok(summary)
}

async fn record_attribute_event_draft<C>(
    state: &AppState,
    db: &C,
    profile_id: i64,
    event: &AttributeEventDraft,
) -> Result<attribute_event::Model, String>
where
    C: ConnectionTrait,
{
    match event.operation.as_str() {
        "add" => state
            .character
            .add_attribute_in(
                db,
                a4n_services::character::AttributeValueChangeInput {
                    profile_id,
                    chapter_number: event.chapter_number,
                    category: &event.category,
                    key: &event.key,
                    value: event.new_value.clone(),
                    change_description: Some(&event.change_description),
                    cause: &event.cause,
                    evidence: &event.evidence,
                },
            )
            .await
            .map_err(|e| e.to_string()),
        "modify" => state
            .character
            .modify_attribute_in(
                db,
                a4n_services::character::AttributeValueChangeInput {
                    profile_id,
                    chapter_number: event.chapter_number,
                    category: &event.category,
                    key: &event.key,
                    value: event.new_value.clone(),
                    change_description: Some(&event.change_description),
                    cause: &event.cause,
                    evidence: &event.evidence,
                },
            )
            .await
            .map_err(|e| e.to_string()),
        other => Err(format!("属性事件草稿不允许 operation={other}")),
    }
}

async fn record_behavior_value<C>(
    state: &AppState,
    db: &C,
    profile_id: i64,
    behavior: &Value,
) -> Result<(), String>
where
    C: ConnectionTrait,
{
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
    state
        .character
        .record_behavior_in(
            db,
            a4n_services::character::BehaviorRecordInput {
                profile_id,
                chapter_number: chapter,
                behavior_type: &behavior_type,
                behavior_description: &description,
                original_text: &original_text,
                behavior_tags,
                classification_results,
                related_characters,
                impact_analysis: impact_analysis.as_deref(),
            },
        )
        .await
        .map(|_| ())
        .map_err(|e| e.to_string())
}

async fn apply_deletion_candidate<C>(
    state: &AppState,
    db: &C,
    profile_id: i64,
    candidate: &Value,
    summary: &mut SyncWriteSummary,
) -> Result<(), String>
where
    C: ConnectionTrait,
{
    let chapter = required_i32(candidate, "chapter_number")?;
    let category = required_category(candidate, "category")?;
    let key = required_string(candidate, "key")?;
    let reason = required_string(candidate, "reason")?;
    let evidence = required_string(candidate, "evidence")?;
    let mark = state
        .character
        .request_attribute_deletion_in(
            db,
            a4n_services::character::AttributeDeletionRequestInput {
                profile_id,
                category: &category,
                key: &key,
                chapter_number: chapter,
                reason: &reason,
                evidence: &evidence,
            },
        )
        .await
        .map_err(|e| e.to_string())?;
    summary.deletions_marked += 1;

    if !mark.is_executed && mark.consecutive_count >= DELETION_THRESHOLD {
        let event_cause = format!("连续{}次标记删除", mark.consecutive_count);
        let (event, _) = state
            .character
            .execute_ready_attribute_deletion_in(
                db,
                a4n_services::character::AttributeDeletionExecutionInput {
                    profile_id,
                    category: &category,
                    key: &key,
                    event_cause: &event_cause,
                    evidence: &evidence,
                    chapter_number: chapter,
                },
            )
            .await
            .map_err(|e| e.to_string())?;
        summary.attribute_event_ids.push(event.id);
        summary.deletions_executed += 1;
        summary.changes_applied += 1;
    }

    Ok(())
}

async fn apply_implicit_review_decision<C>(
    state: &AppState,
    db: &C,
    profile_id: i64,
    target_chapter: i32,
    decision: &ImplicitUpdateDecisionSummary,
    summary: &mut SyncWriteSummary,
) -> Result<(), String>
where
    C: ConnectionTrait,
{
    let applied =
        apply_implicit_update_decision_in(state, db, profile_id, Some(target_chapter), decision)
            .await?;

    match applied.action {
        AppliedImplicitDecisionAction::Keep => {}
        AppliedImplicitDecisionAction::Modify => {
            let event = applied
                .event
                .ok_or_else(|| "隐性属性复审修改缺少属性事件".to_string())?;
            summary.attribute_event_ids.push(event.id);
            summary.changes_applied += 1;
            summary.implicit_reviews_applied += 1;
        }
        AppliedImplicitDecisionAction::MarkDeletion => {
            if applied.deletion_mark.is_none() {
                return Err("隐性属性复审删除缺少删除标记".to_string());
            }
            summary.deletions_marked += 1;
            summary.implicit_reviews_applied += 1;
        }
    }
    Ok(())
}
