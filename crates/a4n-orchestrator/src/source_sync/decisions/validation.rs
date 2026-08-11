use std::collections::BTreeSet;

use a4n_core::error::AppError;

use crate::galgame::{
    GalgameSceneRegenerationPlan, LINE_TYPE_CHOICE, SCENE_ENTRY_MAIN, normalize_line_type,
    normalize_position, parse_and_validate_presentation_projection, validate_line_semantics,
    validate_source_sync_scene_regeneration_stage_outputs,
};

pub(in crate::source_sync) fn validate_galgame_scene_regeneration_plan_for_apply(
    llm_plan: &GalgameSceneRegenerationPlan,
    expected_scene_index: i32,
    expected_source_snapshot_chapter_id: i64,
    valid_text_anchor_ids: &BTreeSet<i64>,
) -> Result<(), AppError> {
    validate_source_sync_scene_regeneration_stage_outputs(&llm_plan.stage_outputs_json)?;
    let scene = &llm_plan.scene;
    if scene.scene_index != expected_scene_index {
        return Err(AppError::InvalidArgument(format!(
            "Galgame source sync 重建计划 scene_index 不匹配: expected={}, actual={}",
            expected_scene_index, scene.scene_index
        )));
    }
    if scene.source_snapshot_chapter_id != expected_source_snapshot_chapter_id {
        return Err(AppError::InvalidArgument(format!(
            "Galgame source sync 重建计划 source_snapshot_chapter_id 不匹配: expected={}, actual={}",
            expected_source_snapshot_chapter_id, scene.source_snapshot_chapter_id
        )));
    }
    if scene.entry_kind != SCENE_ENTRY_MAIN {
        return Err(AppError::InvalidArgument(format!(
            "Galgame source sync 重建计划只能写回 main 场，当前为 {}",
            scene.entry_kind
        )));
    }
    if scene.next_scene_index.is_some() {
        return Err(AppError::InvalidArgument(
            "Galgame source sync 重建计划不能改写场跳转关系".to_string(),
        ));
    }
    if scene.title.trim().is_empty() {
        return Err(AppError::InvalidArgument(
            "Galgame source sync 重建计划场标题不能为空".to_string(),
        ));
    }
    if scene
        .background_note
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .is_none()
    {
        return Err(AppError::InvalidArgument(
            "Galgame source sync 重建计划必须保留 background_note".to_string(),
        ));
    }
    if !scene.participants_json.is_array() {
        return Err(AppError::InvalidArgument(
            "Galgame source sync 重建计划 participants_json 必须是 JSON 数组".to_string(),
        ));
    }
    if scene.lines.is_empty() {
        return Err(AppError::InvalidArgument(
            "Galgame source sync 重建计划不能写入空行集合".to_string(),
        ));
    }
    for (index, line) in scene.lines.iter().enumerate() {
        let expected_line_index = i32::try_from(index + 1).map_err(|_| {
            AppError::InvalidArgument(
                "Galgame source sync 重建计划 line_index 超出 i32 范围".to_string(),
            )
        })?;
        if line.line_index != expected_line_index {
            return Err(AppError::InvalidArgument(format!(
                "Galgame source sync 重建计划 line_index 必须连续: expected={}, actual={}",
                expected_line_index, line.line_index
            )));
        }
        let normalized_line_type = normalize_line_type(&line.line_type)?;
        if normalized_line_type != line.line_type {
            return Err(AppError::InvalidArgument(format!(
                "Galgame source sync 重建计划 line_type 未规范化: {}",
                line.line_type
            )));
        }
        if normalized_line_type == LINE_TYPE_CHOICE || !line.choices.is_empty() {
            return Err(AppError::InvalidArgument(
                "Galgame source sync 场级重建不允许写入选择支；请重新运行完整 Galgame 管线"
                    .to_string(),
            ));
        }
        if line.text.trim().is_empty() {
            return Err(AppError::InvalidArgument(format!(
                "Galgame source sync 重建计划第 {} 行文本不能为空",
                line.line_index
            )));
        }
        if let Some(speaker_profile_id) = line.speaker_profile_id
            && speaker_profile_id <= 0
        {
            return Err(AppError::InvalidArgument(format!(
                "Galgame source sync 重建计划第 {} 行 speaker_profile_id 必须是正整数",
                line.line_index
            )));
        }
        if let Some(expression) = line.expression.as_deref()
            && expression.trim() != expression
        {
            return Err(AppError::InvalidArgument(format!(
                "Galgame source sync 重建计划第 {} 行 expression 未规范化",
                line.line_index
            )));
        }
        validate_line_semantics(
            normalized_line_type,
            line.speaker_profile_id,
            line.expression.as_deref(),
            line.pose.as_deref(),
        )?;
        if normalize_position(line.position.clone())? != line.position {
            return Err(AppError::InvalidArgument(format!(
                "Galgame source sync 重建计划第 {} 行 position 未规范化",
                line.line_index
            )));
        }
        parse_and_validate_presentation_projection(
            &line.presentation_json,
            line.speaker_profile_id,
            line.expression.as_deref(),
            line.pose.as_deref(),
            line.position.as_deref(),
            &format!("Galgame source sync 重建计划第 {} 行", line.line_index),
        )?;
        let Some(text_anchor_id) = line.text_anchor_id else {
            return Err(AppError::InvalidArgument(format!(
                "Galgame source sync 重建计划第 {} 行缺少 text_anchor_id",
                line.line_index
            )));
        };
        if !valid_text_anchor_ids.contains(&text_anchor_id) {
            return Err(AppError::InvalidArgument(format!(
                "Galgame source sync 重建计划第 {} 行 text_anchor_id 不属于目标源快照章节: {}",
                line.line_index, text_anchor_id
            )));
        }
    }

    Ok(())
}
