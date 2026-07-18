use super::{attribute_counts, category_storage_key};
use crate::actions::character_records::{
    AttributeCountsSummary, ProfileChapterStateSummary, SnapshotReferenceSummary,
};
use a4n_core::models::character::snapshot_reference;
use a4n_orchestrator::character::snapshot_attributes_from_data;
use serde_json::{Value, json};

pub(in crate::actions::character_records) fn initial_snapshot_data() -> Value {
    json!({
        "synced_chapter": 0,
        "state_summary": "初始空状态",
        "changes_in_chapter": 0,
        "behaviors_in_chapter": 0,
        "attributes": {
            "explicit_attributes": {},
            "implicit_attributes": {},
            "social_attributes": {},
        },
        "explicit_attributes": {},
        "implicit_attributes": {},
        "social_attributes": {},
    })
}

pub(in crate::actions::character_records) fn snapshot_data_summary(
    snapshot: &snapshot_reference::Model,
) -> Result<SnapshotDataSummary, String> {
    let data = &snapshot.data;
    let attributes = snapshot_attributes_from_data(data, snapshot.chapter_number)
        .map_err(|err| err.to_string())?;
    let label = format!("第 {} 章角色快照", snapshot.chapter_number);
    Ok(SnapshotDataSummary {
        attribute_counts: attribute_counts(&attributes)?,
        state_summary: value_string(data.get("state_summary")),
        current_location: value_string(data.get("current_location")),
        current_goal: value_string(data.get("current_goal")),
        emotional_state: value_string(data.get("emotional_state")),
        changes_in_chapter: required_usize_field(data, "changes_in_chapter", &label)?,
        behaviors_in_chapter: required_usize_field(data, "behaviors_in_chapter", &label)?,
    })
}

#[derive(Debug, Clone)]
pub(in crate::actions::character_records) struct SnapshotDataSummary {
    pub(in crate::actions::character_records) attribute_counts: AttributeCountsSummary,
    pub(in crate::actions::character_records) state_summary: Option<String>,
    pub(in crate::actions::character_records) current_location: Option<String>,
    pub(in crate::actions::character_records) current_goal: Option<String>,
    pub(in crate::actions::character_records) emotional_state: Option<String>,
    pub(in crate::actions::character_records) changes_in_chapter: usize,
    pub(in crate::actions::character_records) behaviors_in_chapter: usize,
}

pub(in crate::actions::character_records) fn snapshot_reference_summary(
    snapshot: &snapshot_reference::Model,
) -> Result<SnapshotReferenceSummary, String> {
    let data = snapshot_data_summary(snapshot)?;
    Ok(SnapshotReferenceSummary {
        id: snapshot.id,
        profile_id: snapshot.profile_id,
        chapter_number: snapshot.chapter_number,
        snapshot_hash: snapshot.snapshot_hash.clone(),
        reference_count: snapshot.reference_count,
        attribute_counts: data.attribute_counts,
        state_summary: data.state_summary,
        current_location: data.current_location,
        current_goal: data.current_goal,
        emotional_state: data.emotional_state,
        changes_in_chapter: data.changes_in_chapter,
        behaviors_in_chapter: data.behaviors_in_chapter,
        created_at: Some(snapshot.created_at.to_string()),
    })
}

pub(in crate::actions::character_records) fn profile_state_summary_from_snapshot(
    snapshot: &snapshot_reference::Model,
) -> Result<ProfileChapterStateSummary, String> {
    let attributes = snapshot_attributes_from_data(&snapshot.data, snapshot.chapter_number)
        .map_err(|err| err.to_string())?;
    let data = snapshot_data_summary(snapshot)?;
    Ok(ProfileChapterStateSummary {
        id: snapshot.id,
        profile_id: snapshot.profile_id,
        chapter_number: snapshot.chapter_number,
        snapshot_hash: snapshot.snapshot_hash.clone(),
        attribute_counts: data.attribute_counts,
        explicit_attributes: snapshot_category_value(&attributes, "explicit")?,
        implicit_attributes: snapshot_category_value(&attributes, "implicit")?,
        social_attributes: snapshot_category_value(&attributes, "social")?,
        state_summary: data.state_summary,
        current_location: data.current_location,
        current_goal: data.current_goal,
        emotional_state: data.emotional_state,
        changes_in_chapter: data.changes_in_chapter,
        behaviors_in_chapter: data.behaviors_in_chapter,
        created_at: Some(snapshot.created_at.to_string()),
    })
}

fn snapshot_category_value(attributes: &Value, category: &str) -> Result<Value, String> {
    let key = category_storage_key(category);
    match attributes.get(key) {
        Some(Value::Object(object)) => Ok(Value::Object(object.clone())),
        Some(_) => Err(format!("角色快照属性分类 {key} 必须是对象")),
        None => Err(format!("角色快照属性缺少分类 {key}")),
    }
}

fn required_usize_field(value: &Value, field: &str, label: &str) -> Result<usize, String> {
    let number = value
        .get(field)
        .ok_or_else(|| format!("{label}缺少字段 {field}"))?
        .as_u64()
        .ok_or_else(|| format!("{label}字段 {field} 必须是非负整数"))?;
    usize::try_from(number).map_err(|_| format!("{label}字段 {field} 超出 usize 范围"))
}

fn value_string(value: Option<&Value>) -> Option<String> {
    value
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|text| !text.is_empty())
        .map(ToOwned::to_owned)
}
