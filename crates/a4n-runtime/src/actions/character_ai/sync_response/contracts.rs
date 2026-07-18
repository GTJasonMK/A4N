use super::*;

pub(in crate::actions::character_ai) const CHARACTER_SYNC_ATTRIBUTE_CHANGES_PER_CHAPTER_MAX: usize =
    6;
pub(in crate::actions::character_ai) const CHARACTER_SYNC_BEHAVIORS_PER_CHAPTER_MAX: usize = 6;
pub(in crate::actions::character_ai) const CHARACTER_SYNC_DELETION_CANDIDATES_PER_CHAPTER_MAX:
    usize = 3;
pub(super) const CHARACTER_SYNC_DESCRIPTION_MAX_CHARS: usize = 120;
pub(super) const CHARACTER_SYNC_IMPACT_ANALYSIS_MAX_CHARS: usize = 160;
pub(super) const CHARACTER_SYNC_STATE_SUMMARY_MAX_CHARS: usize = 160;
pub(super) const CHARACTER_SYNC_BEHAVIOR_TAGS_MAX_ITEMS: usize = 5;
pub(super) const CHARACTER_SYNC_RELATED_CHARACTERS_MAX_ITEMS: usize = 6;

#[derive(Debug, Clone)]
pub(in crate::actions::character_ai) struct ChapterStateDraft {
    pub(in crate::actions::character_ai) chapter_number: i32,
    pub(in crate::actions::character_ai) state_summary: String,
    pub(in crate::actions::character_ai) current_location: Option<String>,
    pub(in crate::actions::character_ai) current_goal: Option<String>,
    pub(in crate::actions::character_ai) emotional_state: Option<String>,
}

pub(in crate::actions::character_ai) fn validate_unique_deletion_candidates(
    candidates: &[Value],
) -> Result<(), String> {
    let mut seen = BTreeSet::new();
    for (index, candidate) in candidates.iter().enumerate() {
        let chapter = required_i32_at(
            candidate,
            "chapter_number",
            &format!("deletion_candidates[{index}].chapter_number"),
        )?;
        let category = required_category(candidate, "category")?;
        let key = required_string(candidate, "key")?;
        if !seen.insert((chapter, category.clone(), key.clone())) {
            return Err(format!(
                "deletion_candidates[{index}] 与前序删除候选重复：第 {chapter} 章 {category}.{key}"
            ));
        }
    }
    Ok(())
}

#[cfg(test)]
pub(in crate::actions::character_ai) fn parse_character_sync_output(
    content: &str,
) -> Result<Value, String> {
    let value: Value = serde_json::from_str(content.trim())
        .map_err(|e| format!("角色同步结果必须是 JSON 对象: {e}"))?;
    if !value.is_object() {
        return Err("角色同步结果必须是 JSON 对象".to_string());
    }
    if value.get("updated_attributes").is_some() {
        return Err(
            "角色 AI 顶层字段 updated_attributes 已废弃；最终状态由系统根据 attribute_changes 推导"
                .to_string(),
        );
    }
    for key in [
        "state_summary",
        "current_location",
        "current_goal",
        "emotional_state",
    ] {
        if value.get(key).is_some() {
            return Err(format!(
                "角色 AI 顶层字段 {key} 已废弃；每章状态必须写入 chapter_states"
            ));
        }
    }
    validate_top_level_fields(&value)?;
    for (index, change) in required_array(&value, "attribute_changes")?
        .iter()
        .enumerate()
    {
        validate_attribute_change(index, change)?;
    }
    for (index, behavior) in required_array(&value, "behaviors")?.iter().enumerate() {
        validate_behavior(index, behavior)?;
    }
    for (index, candidate) in required_array(&value, "deletion_candidates")?
        .iter()
        .enumerate()
    {
        validate_deletion_candidate(index, candidate)?;
    }
    validate_unique_deletion_candidates(required_array(&value, "deletion_candidates")?)?;
    for (index, state) in required_array(&value, "chapter_states")?.iter().enumerate() {
        validate_chapter_state(index, state)?;
    }
    Ok(value)
}

#[cfg(test)]
fn validate_top_level_fields(value: &Value) -> Result<(), String> {
    let object = value
        .as_object()
        .ok_or_else(|| "角色同步结果必须是 JSON 对象".to_string())?;
    for key in object.keys() {
        if matches!(
            key.as_str(),
            "profile_name"
                | "attribute_changes"
                | "behaviors"
                | "deletion_candidates"
                | "chapter_states"
        ) {
            continue;
        }
        return Err(format!(
            "角色 AI 顶层字段 {key} 不在当前角色同步合同中；只允许 profile_name、attribute_changes、behaviors、deletion_candidates、chapter_states"
        ));
    }
    Ok(())
}
