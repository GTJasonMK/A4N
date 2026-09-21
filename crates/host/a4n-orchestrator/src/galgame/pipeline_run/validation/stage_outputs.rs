use a4n_error::AppError;
use serde_json::Value;

use crate::galgame::{
    branch_design_from_value, character_staging_from_value, script_adaptation_from_value,
    shooting_script_projection_from_value, visual_plan_from_value,
};

use super::super::contracts::{
    SOURCE_SYNC_REGENERATION_STAGE_OUTPUT_KEYS, SOURCE_SYNC_STAGE_BRANCH_DESIGN,
    SOURCE_SYNC_STAGE_CHARACTER_STAGING, SOURCE_SYNC_STAGE_SCRIPT_ADAPTATION,
    SOURCE_SYNC_STAGE_SHOOTING_SCRIPT, SOURCE_SYNC_STAGE_VISUAL_PLANNING,
};
use super::regeneration::{
    ensure_single_regenerated_scene_lines, ensure_single_regenerated_staged_lines,
    normalize_single_scene_visual_plan,
};

pub(crate) fn validate_source_sync_scene_regeneration_stage_outputs(
    stage_outputs_json: &Value,
) -> Result<(), AppError> {
    let object = stage_outputs_json.as_object().ok_or_else(|| {
        AppError::InvalidArgument("Galgame source sync stage_outputs 必须是 JSON 对象".to_string())
    })?;
    for key in SOURCE_SYNC_REGENERATION_STAGE_OUTPUT_KEYS {
        if !object.contains_key(*key) {
            return Err(AppError::InvalidArgument(format!(
                "Galgame source sync stage_outputs 缺少 {key}"
            )));
        }
    }
    if let Some(key) = object
        .keys()
        .find(|key| !SOURCE_SYNC_REGENERATION_STAGE_OUTPUT_KEYS.contains(&key.as_str()))
    {
        return Err(AppError::InvalidArgument(format!(
            "Galgame source sync stage_outputs 包含未知阶段: {key}"
        )));
    }

    let shooting_script_projection = shooting_script_projection_from_value(
        required_stage_output_value(object, SOURCE_SYNC_STAGE_SHOOTING_SCRIPT)?,
        "stage_outputs.shooting_script_projection",
    )?;
    if shooting_script_projection.scenes.len() != 1
        || shooting_script_projection
            .scenes
            .first()
            .is_none_or(|scene| scene.index != 1 || scene.source_span_ids.is_empty())
    {
        return Err(AppError::InvalidArgument(
            "Galgame source sync stage_outputs.shooting_script_projection 必须只包含 scene_index=1 且带 source_span_ids 的单场"
                .to_string(),
        ));
    }

    let script_adaptation = script_adaptation_from_value(
        required_stage_output_value(object, SOURCE_SYNC_STAGE_SCRIPT_ADAPTATION)?,
        "stage_outputs.script_adaptation",
    )?;
    ensure_single_regenerated_scene_lines(&script_adaptation)?;

    let character_staging = character_staging_from_value(
        required_stage_output_value(object, SOURCE_SYNC_STAGE_CHARACTER_STAGING)?,
        "stage_outputs.character_staging",
    )?;
    ensure_single_regenerated_staged_lines(&character_staging)?;

    let visual_plan = visual_plan_from_value(
        required_stage_output_value(object, SOURCE_SYNC_STAGE_VISUAL_PLANNING)?,
        "stage_outputs.visual_planning",
    )?;
    let visual_plan = normalize_single_scene_visual_plan(visual_plan)?;
    if visual_plan.backgrounds[0].source_span_ids.is_empty()
        || visual_plan
            .event_cgs
            .iter()
            .any(|event_cg| event_cg.source_span_ids.is_empty())
    {
        return Err(AppError::InvalidArgument(
            "Galgame source sync stage_outputs.visual_planning 必须保留背景/事件 CG 的 source_span_ids"
                .to_string(),
        ));
    }

    let branch_design = branch_design_from_value(
        required_stage_output_value(object, SOURCE_SYNC_STAGE_BRANCH_DESIGN)?,
        "stage_outputs.branch_design",
    )?;
    if !branch_design.choices.is_empty() {
        return Err(AppError::InvalidArgument(
            "Galgame source sync stage_outputs.branch_design 不允许包含 choices".to_string(),
        ));
    }

    Ok(())
}

fn required_stage_output_value(
    object: &serde_json::Map<String, Value>,
    key: &str,
) -> Result<Value, AppError> {
    object
        .get(key)
        .filter(|value| !value.is_null())
        .cloned()
        .ok_or_else(|| {
            AppError::InvalidArgument(format!("Galgame source sync stage_outputs.{key} 不能为空"))
        })
}
