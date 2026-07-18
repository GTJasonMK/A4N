use super::contracts::*;
use a4n_core::models::character::{
    attribute_event, protagonist_behavior_record, protagonist_deletion_mark, protagonist_profile,
};
use a4n_orchestrator::character::DELETION_THRESHOLD;
use serde_json::Value;

mod snapshots;

#[cfg(test)]
pub(super) use snapshots::snapshot_data_summary;
pub(super) use snapshots::{
    initial_snapshot_data, profile_state_summary_from_snapshot, snapshot_reference_summary,
};
pub(super) fn attribute_counts(value: &Value) -> Result<AttributeCountsSummary, String> {
    Ok(AttributeCountsSummary {
        explicit: category_count(value, "explicit")?,
        implicit: category_count(value, "implicit")?,
        social: category_count(value, "social")?,
    })
}

pub(super) fn profile_summary(
    p: &protagonist_profile::Model,
) -> Result<ProtagonistProfileSummary, String> {
    Ok(ProtagonistProfileSummary {
        id: p.id,
        project_id: p.project_id,
        name: p.name.clone(),
        gender: p.gender.clone(),
        age: p.age.clone(),
        base_attributes: p.base_attributes.clone(),
        current_state: p.current_state.clone(),
        character_arc: p.character_arc.clone(),
        voice_fingerprint: p.voice_fingerprint.clone(),
        appearance: p.appearance.clone(),
        background: p.background.clone(),
        personality: p.personality.clone(),
        last_synced_chapter: p.last_synced_chapter,
        attribute_counts: attribute_counts(&p.current_state)?,
        created_at: Some(p.created_at.to_string()),
        updated_at: Some(p.updated_at.to_string()),
    })
}

pub(super) fn attribute_event_summary(event: &attribute_event::Model) -> AttributeEventSummary {
    AttributeEventSummary {
        id: event.id,
        profile_id: event.profile_id,
        chapter_number: event.chapter_number,
        timestamp: Some(event.timestamp.to_string()),
        category: event.category.clone(),
        key: event.key.clone(),
        operation: event.operation.clone(),
        change_description: event.change_description.clone(),
        old_value: event.old_value.clone(),
        new_value: event.new_value.clone(),
        cause: event.cause.clone(),
        evidence: event.evidence.clone(),
    }
}

pub(super) fn behavior_record_summary(
    record: &protagonist_behavior_record::Model,
) -> BehaviorRecordSummary {
    BehaviorRecordSummary {
        id: record.id,
        profile_id: record.profile_id,
        chapter_number: record.chapter_number,
        behavior_description: record.behavior_description.clone(),
        behavior_type: record.behavior_type.clone(),
        original_text: record.original_text.clone(),
        behavior_tags: record.behavior_tags.clone(),
        classification_results: record.classification_results.clone(),
        related_characters: record.related_characters.clone(),
        impact_analysis: record.impact_analysis.clone(),
        created_at: Some(record.created_at.to_string()),
    }
}

pub(super) fn deletion_mark_summary(
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

pub(super) fn category_storage_key(category: &str) -> &str {
    match category {
        "explicit" => "explicit_attributes",
        "implicit" => "implicit_attributes",
        "social" => "social_attributes",
        other => other,
    }
}

pub(super) fn category_count(value: &Value, category: &str) -> Result<usize, String> {
    let key = category_storage_key(category);
    match value.get(key) {
        Some(Value::Object(object)) => Ok(object.len()),
        Some(_) => Err(format!("角色属性分类 {key} 必须是对象")),
        None => Err(format!("角色属性缺少分类 {key}")),
    }
}

pub(super) fn required_text(value: &str, label: &str) -> Result<String, String> {
    let text = value.trim();
    if text.is_empty() {
        return Err(format!("{label}不能为空"));
    }
    Ok(text.to_string())
}
