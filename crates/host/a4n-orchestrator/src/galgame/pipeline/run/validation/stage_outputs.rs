use a4n_error::AppError;
use serde_json::Value;

use crate::galgame::{
    adaptation_projection_from_value, branch_design_from_value, character_staging_from_value,
    script_adaptation_from_value, visual_plan_from_value,
};

use super::super::contracts::{
    SOURCE_SYNC_REGENERATION_STAGE_OUTPUT_KEYS, SOURCE_SYNC_STAGE_ADAPTATION,
    SOURCE_SYNC_STAGE_BRANCH_DESIGN, SOURCE_SYNC_STAGE_CHARACTER_STAGING,
    SOURCE_SYNC_STAGE_SCRIPT_ADAPTATION, SOURCE_SYNC_STAGE_VISUAL_PLANNING,
};
use super::regeneration::{
    ensure_single_regenerated_scene_lines, ensure_single_regenerated_staged_lines,
    ensure_single_regenerated_visual_plan,
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

    let adaptation_projection = adaptation_projection_from_value(
        required_stage_output_value(object, SOURCE_SYNC_STAGE_ADAPTATION)?,
        "stage_outputs.adaptation_projection",
    )?;
    if adaptation_projection.scenes.len() != 1
        || adaptation_projection
            .scenes
            .first()
            .is_none_or(|scene| scene.index != 1)
    {
        return Err(AppError::InvalidArgument(
            "Galgame source sync stage_outputs.adaptation_projection 必须只包含 scene_index=1 的单场"
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
    ensure_single_regenerated_visual_plan(&visual_plan)?;
    let scene = &adaptation_projection.scenes[0];
    a4n_domain::media_authoring::validate_adaptation_evidence(
        scene.origin,
        &visual_plan.backgrounds[0].source_span_ids,
        &scene.source_span_ids.iter().cloned().collect(),
    )
    .map_err(|error| {
        AppError::InvalidArgument(format!(
            "stage_outputs.visual_planning.backgrounds[0].source_span_ids: {error}"
        ))
    })?;
    for cg in &visual_plan.event_cgs {
        let line = character_staging.scene_lines[0]
            .lines
            .iter()
            .find(|line| line.index == cg.line_index)
            .ok_or_else(|| AppError::InvalidArgument("来源同步 CG 引用了未知剧本行".into()))?;
        a4n_domain::media_authoring::validate_adaptation_evidence(
            line.origin,
            &cg.source_span_ids,
            &line.source_span_ids.iter().cloned().collect(),
        )
        .map_err(AppError::InvalidArgument)?;
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
