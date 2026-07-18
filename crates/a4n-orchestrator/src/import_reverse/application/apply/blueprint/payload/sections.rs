use serde_json::Value;

use crate::blueprint::BLUEPRINT_GENERATION_STAGES;

fn validate_stage_payload(stage_key: &str, payload: Value) -> Result<(), String> {
    let spec = BLUEPRINT_GENERATION_STAGES
        .iter()
        .find(|spec| spec.key == stage_key)
        .ok_or_else(|| format!("未知蓝图生成阶段: {stage_key}"))?;
    (spec.validate)(&payload).map_err(|err| err.to_string())
}

pub(in crate::import_reverse::application::apply) fn validate_reverse_blueprint_sections(
    data: &Value,
    world_setting: &Value,
) -> Result<(), String> {
    validate_stage_payload(
        "world",
        serde_json::json!({ "world_setting": world_setting }),
    )?;
    let characters = data
        .get("characters")
        .ok_or_else(|| "reverse_blueprint 缺少必填数组字段 characters".to_string())?;
    validate_stage_payload(
        "characters",
        serde_json::json!({ "characters": characters }),
    )?;
    let relationships = data
        .get("relationships")
        .ok_or_else(|| "reverse_blueprint 缺少必填数组字段 relationships".to_string())?;
    validate_stage_payload(
        "relationships",
        serde_json::json!({ "relationships": relationships }),
    )?;
    Ok(())
}

#[cfg(test)]
mod tests;
