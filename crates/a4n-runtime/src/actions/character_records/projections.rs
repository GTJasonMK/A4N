use a4n_api_contract::character::{
    AttributeCountsSummary, AttributeEventSummary, BehaviorRecordSummary, CharacterProfileSummary,
    DeletionMarkSummary,
};
use a4n_core::models::character::{
    attribute_event, protagonist_behavior_record, protagonist_deletion_mark, protagonist_profile,
};
use a4n_domain::character::{
    CharacterAttributeCategory, CharacterAttributeOperation, CharacterTimelineSourceKind,
};
use serde_json::Value;

mod snapshots;

#[cfg(test)]
pub(super) use snapshots::snapshot_data_summary;
pub(super) use snapshots::{profile_state_summary_from_snapshot, snapshot_reference_summary};
pub(super) fn attribute_counts(value: &Value) -> Result<AttributeCountsSummary, String> {
    Ok(AttributeCountsSummary {
        explicit: category_count(value, "explicit")?,
        implicit: category_count(value, "implicit")?,
        social: category_count(value, "social")?,
    })
}

pub(super) fn profile_summary(
    p: &protagonist_profile::Model,
    timeline_started: bool,
    initial_profile_editable: bool,
) -> Result<CharacterProfileSummary, String> {
    Ok(CharacterProfileSummary {
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
        timeline_started,
        initial_profile_editable,
        attribute_counts: attribute_counts(&p.current_state)?,
        created_at: Some(p.created_at.to_string()),
        updated_at: Some(p.updated_at.to_string()),
    })
}

pub(super) fn attribute_event_summary(
    event: &attribute_event::Model,
) -> Result<AttributeEventSummary, String> {
    Ok(AttributeEventSummary {
        id: event.id,
        chapter_number: event.chapter_number,
        source_kind: timeline_source_kind(&event.source_kind)?,
        timestamp: Some(event.timestamp.to_string()),
        category: attribute_category(&event.category)?,
        key: event.key.clone(),
        operation: attribute_operation(&event.operation)?,
        change_description: event.change_description.clone(),
        old_value: event.old_value.clone(),
        new_value: event.new_value.clone(),
        cause: event.cause.clone(),
        evidence: event.evidence.clone(),
    })
}

pub(super) fn behavior_record_summary(
    record: &protagonist_behavior_record::Model,
) -> Result<BehaviorRecordSummary, String> {
    Ok(BehaviorRecordSummary {
        id: record.id,
        chapter_number: record.chapter_number,
        source_kind: timeline_source_kind(&record.source_kind)?,
        behavior_description: record.behavior_description.clone(),
        behavior_type: record.behavior_type.clone(),
        original_text: record.original_text.clone(),
        behavior_tags: record.behavior_tags.clone(),
        classification_results: record.classification_results.clone(),
        related_characters: record.related_characters.clone(),
        impact_analysis: record.impact_analysis.clone(),
    })
}

pub(crate) fn deletion_mark_summary(
    mark: &protagonist_deletion_mark::Model,
) -> Result<DeletionMarkSummary, String> {
    Ok(DeletionMarkSummary {
        id: mark.id,
        source_kind: timeline_source_kind(&mark.source_kind)?,
        category: attribute_category(&mark.category)?,
        key: mark.key.clone(),
        reason: mark.reason.clone(),
        evidence: mark.evidence.clone(),
        consecutive_count: mark.consecutive_count,
        last_marked_chapter: mark.last_marked_chapter,
        is_executed: mark.is_executed,
        created_at: mark.created_at.to_string(),
    })
}

fn timeline_source_kind(value: &str) -> Result<CharacterTimelineSourceKind, String> {
    CharacterTimelineSourceKind::parse(value)
        .ok_or_else(|| format!("角色时间线 source_kind 无效: {value}"))
}

fn attribute_category(value: &str) -> Result<CharacterAttributeCategory, String> {
    CharacterAttributeCategory::parse(value).ok_or_else(|| format!("角色属性分类无效: {value}"))
}

fn attribute_operation(value: &str) -> Result<CharacterAttributeOperation, String> {
    CharacterAttributeOperation::parse(value).ok_or_else(|| format!("角色属性操作无效: {value}"))
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
