use super::{AttributeEventSummary, DeletionMarkSummary};
use a4n_core::models::character::{attribute_event, protagonist_deletion_mark};
use a4n_orchestrator::character::DELETION_THRESHOLD;

pub(super) fn to_event_summary(e: &attribute_event::Model) -> AttributeEventSummary {
    AttributeEventSummary {
        id: e.id,
        profile_id: e.profile_id,
        chapter_number: e.chapter_number,
        timestamp: Some(e.timestamp.to_string()),
        category: e.category.clone(),
        key: e.key.clone(),
        operation: e.operation.clone(),
        change_description: e.change_description.clone(),
        old_value: e.old_value.clone(),
        new_value: e.new_value.clone(),
        cause: e.cause.clone(),
        evidence: e.evidence.clone(),
    }
}

pub(super) fn to_deletion_mark_summary(
    mark: &protagonist_deletion_mark::Model,
) -> DeletionMarkSummary {
    DeletionMarkSummary {
        id: mark.id,
        profile_id: mark.profile_id,
        category: mark.category.clone(),
        key: mark.key.clone(),
        chapter_number: mark.chapter_number,
        reason: mark.reason.clone(),
        evidence: mark.evidence.clone(),
        consecutive_count: mark.consecutive_count,
        threshold: DELETION_THRESHOLD,
        last_marked_chapter: mark.last_marked_chapter,
        is_executed: mark.is_executed,
        created_at: mark.created_at.to_string(),
        updated_at: mark.updated_at.to_string(),
    }
}
