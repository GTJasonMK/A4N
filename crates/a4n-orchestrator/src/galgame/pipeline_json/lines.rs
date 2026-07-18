use a4n_core::error::AppError;

use super::super::{GALGAME_EXPRESSIONS, LINE_TYPE_DIALOGUE, LINE_TYPE_MONOLOGUE};
use super::contracts::*;
use super::rules::*;

pub(super) fn line_draft_from_raw(
    line: LineDraftRaw,
    context: &str,
) -> Result<GalgamePipelineLineDraft, AppError> {
    require_positive(line.index, &format!("{context}.lines[].index"))?;
    let line_type =
        normalize_script_line_type(line.line_type, &format!("{context}.lines[].line_type"))?;
    let speaker_name = optional_text(line.speaker_name);
    if matches!(line_type.as_str(), LINE_TYPE_DIALOGUE | LINE_TYPE_MONOLOGUE)
        && speaker_name.is_none()
    {
        return Err(invalid(format!("{context}.lines[].speaker_name 不能为空")));
    }
    Ok(GalgamePipelineLineDraft {
        index: line.index,
        beat_ref: validated_beat_ref(line.beat_ref, context)?,
        line_type,
        speaker_name,
        text: required_text(line.text, &format!("{context}.lines[].text"))?,
        source_span_ids: normalize_source_span_ids(
            line.source_span_ids,
            &format!("{context}.lines[].source_span_ids"),
            true,
        )?,
    })
}

pub(super) fn line_staged_from_raw(
    line: LineStagedRaw,
    context: &str,
) -> Result<GalgamePipelineLineStaged, AppError> {
    require_positive(line.index, &format!("{context}.lines[].index"))?;
    let line_type =
        normalize_script_line_type(line.line_type, &format!("{context}.lines[].line_type"))?;
    let speaker_name = optional_text(line.speaker_name);
    let speaker_profile_id = match line.speaker_profile_id {
        Some(id) if id <= 0 => {
            return Err(invalid(format!(
                "{context}.lines[].speaker_profile_id 必须是正整数"
            )));
        }
        other => other,
    };
    if matches!(line_type.as_str(), LINE_TYPE_DIALOGUE | LINE_TYPE_MONOLOGUE)
        && speaker_name.is_none()
    {
        return Err(invalid(format!("{context}.lines[].speaker_name 不能为空")));
    }
    if !matches!(line_type.as_str(), LINE_TYPE_DIALOGUE | LINE_TYPE_MONOLOGUE)
        && speaker_profile_id.is_some()
    {
        return Err(invalid(format!(
            "{context}.lines[].speaker_profile_id 只能用于 dialogue/monologue"
        )));
    }
    validate_staged_presentation(&line.presentation, speaker_profile_id, context)?;
    Ok(GalgamePipelineLineStaged {
        index: line.index,
        beat_ref: validated_beat_ref(line.beat_ref, context)?,
        line_type,
        speaker_profile_id,
        speaker_name,
        text: required_text(line.text, &format!("{context}.lines[].text"))?,
        presentation: line.presentation,
        source_span_ids: normalize_source_span_ids(
            line.source_span_ids,
            &format!("{context}.lines[].source_span_ids"),
            true,
        )?,
    })
}

fn validated_beat_ref(
    beat_ref: GalgamePipelineBeatRefV3,
    context: &str,
) -> Result<GalgamePipelineBeatRefV3, AppError> {
    require_positive(
        beat_ref.scene_index,
        &format!("{context}.lines[].beat_ref.scene_index"),
    )?;
    if beat_ref.shot_index == 0 {
        return Err(invalid(format!(
            "{context}.lines[].beat_ref.shot_index 必须大于 0"
        )));
    }
    if beat_ref.beat_index == 0 {
        return Err(invalid(format!(
            "{context}.lines[].beat_ref.beat_index 必须大于 0"
        )));
    }
    Ok(beat_ref)
}

pub(in crate::galgame) fn validate_staged_presentation(
    presentation: &GalgameLinePresentationV2,
    speaker_profile_id: Option<i64>,
    context: &str,
) -> Result<(), AppError> {
    if presentation.schema_version != "galgame_line_presentation_v2" {
        return Err(invalid(format!(
            "{context}.lines[].presentation.schema_version 必须是 galgame_line_presentation_v2"
        )));
    }
    let mut visible = std::collections::BTreeSet::new();
    for cast in &presentation.visible_cast {
        if cast.character_profile_id <= 0 || !visible.insert(cast.character_profile_id) {
            return Err(invalid(format!(
                "{context}.lines[].presentation.visible_cast 角色 ID 必须为正整数且不能重复"
            )));
        }
        if !GALGAME_EXPRESSIONS.contains(&cast.expression.trim()) {
            return Err(invalid(format!(
                "{context}.lines[].presentation.visible_cast.expression 无效: {}",
                cast.expression
            )));
        }
        if cast.pose.trim().is_empty() {
            return Err(invalid(format!(
                "{context}.lines[].presentation.visible_cast.pose 不能为空"
            )));
        }
    }
    for (field, ids) in [("enter", &presentation.enter), ("exit", &presentation.exit)] {
        let mut unique = std::collections::BTreeSet::new();
        for id in ids {
            if *id <= 0 || !unique.insert(*id) || !visible.contains(id) {
                return Err(invalid(format!(
                    "{context}.lines[].presentation.{field} 必须引用 visible_cast 中不重复的正整数角色 ID"
                )));
            }
        }
    }
    if let Some(focus) = presentation.focus_character_profile_id
        && !visible.contains(&focus)
    {
        return Err(invalid(format!(
            "{context}.lines[].presentation.focus_character_profile_id 必须属于 visible_cast"
        )));
    }
    if let Some(speaker) = speaker_profile_id
        && !visible.contains(&speaker)
    {
        return Err(invalid(format!(
            "{context}.lines[].presentation.visible_cast 必须包含当前 speaker_profile_id"
        )));
    }
    let mut knowledge = std::collections::BTreeSet::new();
    for value in &presentation.player_knowledge {
        let value = value.trim();
        if value.is_empty() || !knowledge.insert(value) {
            return Err(invalid(format!(
                "{context}.lines[].presentation.player_knowledge 不能包含空值或重复值"
            )));
        }
    }
    Ok(())
}
