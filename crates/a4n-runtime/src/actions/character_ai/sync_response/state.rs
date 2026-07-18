use super::*;

pub(in crate::actions::character_ai) fn validate_deletion_candidate(
    index: usize,
    candidate: &Value,
) -> Result<(), String> {
    if !candidate.is_object() {
        return Err(format!("deletion_candidates[{index}] 必须是 JSON 对象"));
    }
    required_i32_at(
        candidate,
        "chapter_number",
        &format!("deletion_candidates[{index}].chapter_number"),
    )?;
    required_category(candidate, "category")?;
    required_string(candidate, "key")?;
    required_limited_string(
        candidate,
        "reason",
        &format!("deletion_candidates[{index}].reason"),
        CHARACTER_SYNC_DESCRIPTION_MAX_CHARS,
    )?;
    reject_legacy_anchor_field(
        candidate,
        "evidence_span_id",
        &format!("deletion_candidates[{index}].evidence_span_id"),
        "evidence_anchor_id",
    )?;
    required_positive_i64(
        candidate,
        "evidence_anchor_id",
        &format!("deletion_candidates[{index}].evidence_anchor_id"),
    )?;
    if candidate.get("evidence").is_some() {
        return Err(format!(
            "deletion_candidates[{index}].evidence 已废弃；必须输出 evidence_anchor_id"
        ));
    }
    Ok(())
}

pub(in crate::actions::character_ai) fn validate_chapter_state(
    index: usize,
    state: &Value,
) -> Result<(), String> {
    if !state.is_object() {
        return Err(format!("chapter_states[{index}] 必须是 JSON 对象"));
    }
    required_i32_at(
        state,
        "chapter_number",
        &format!("chapter_states[{index}].chapter_number"),
    )?;
    required_limited_string(
        state,
        "state_summary",
        &format!("chapter_states[{index}].state_summary"),
        CHARACTER_SYNC_STATE_SUMMARY_MAX_CHARS,
    )?;
    optional_string_or_null(state, "current_location")?;
    optional_string_or_null(state, "current_goal")?;
    optional_string_or_null(state, "emotional_state")?;
    Ok(())
}

pub(in crate::actions::character_ai) fn chapter_state_drafts(
    value: &Value,
    chapter_numbers: &[i32],
) -> Result<Vec<ChapterStateDraft>, String> {
    let expected = chapter_numbers.iter().copied().collect::<BTreeSet<_>>();
    let mut by_chapter = BTreeMap::new();
    for (index, state) in required_array(value, "chapter_states")?.iter().enumerate() {
        validate_chapter_state(index, state)?;
        let chapter_number = required_i32_at(
            state,
            "chapter_number",
            &format!("chapter_states[{index}].chapter_number"),
        )?;
        if !expected.contains(&chapter_number) {
            return Err(format!(
                "chapter_states[{index}].chapter_number 超出本次同步章节: 第 {chapter_number} 章"
            ));
        }
        let draft = ChapterStateDraft {
            chapter_number,
            state_summary: required_string(state, "state_summary")?,
            current_location: optional_string(state, "current_location"),
            current_goal: optional_string(state, "current_goal"),
            emotional_state: optional_string(state, "emotional_state"),
        };
        if by_chapter.insert(chapter_number, draft).is_some() {
            return Err(format!(
                "chapter_states 包含重复章节: 第 {chapter_number} 章"
            ));
        }
    }

    let mut missing = Vec::new();
    let mut drafts = Vec::with_capacity(chapter_numbers.len());
    for chapter_number in chapter_numbers {
        if let Some(draft) = by_chapter.remove(chapter_number) {
            drafts.push(draft);
        } else {
            missing.push(*chapter_number);
        }
    }
    if !missing.is_empty() {
        return Err(format!(
            "chapter_states 缺少章节状态: {}",
            missing
                .iter()
                .map(|chapter| format!("第 {chapter} 章"))
                .collect::<Vec<_>>()
                .join("、")
        ));
    }
    Ok(drafts)
}
