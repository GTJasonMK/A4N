use super::super::{GalgameVisualSpecPlan, generation_spec};
use serde_json::{Map, Value};

pub(in crate::actions::galgame_records::asset_generation) fn validate_galgame_visual_media_request_snapshot(
    spec: &generation_spec::Model,
    plan: &GalgameVisualSpecPlan,
    media_asset_id: i64,
    request_snapshot: Option<&Value>,
) -> Result<(), String> {
    let request = request_snapshot
        .ok_or_else(|| format!("Galgame 视觉生成意图已完成但媒体来源缺少 request_snapshot_json: spec_id={}, media_asset_id={}", spec.id, media_asset_id))?
        .as_object()
        .ok_or_else(|| format!("Galgame 视觉生成意图已完成但媒体来源 request_snapshot_json 必须是对象: spec_id={}, media_asset_id={}", spec.id, media_asset_id))?;
    let source = request.get("source").and_then(Value::as_str).map(str::trim);
    if source != Some("galgame_asset_generation") {
        return Err(format!(
            "Galgame 视觉生成意图已完成但媒体来源 source 无效: spec_id={}, media_asset_id={}, source={:?}",
            spec.id, media_asset_id, source
        ));
    }
    validate_request_string(
        request,
        "target_kind",
        spec.target_kind.as_str(),
        spec.id,
        media_asset_id,
    )?;
    validate_request_string(
        request,
        "target_entity_kind",
        spec.target_entity_kind.as_str(),
        spec.id,
        media_asset_id,
    )?;
    validate_request_i64(
        request,
        "target_entity_id",
        plan.target_entity_id,
        spec.id,
        media_asset_id,
    )
}

fn validate_request_string(
    request: &Map<String, Value>,
    key: &str,
    expected: &str,
    spec_id: i64,
    media_asset_id: i64,
) -> Result<(), String> {
    let actual = request
        .get(key)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| format!("Galgame 视觉生成意图已完成但媒体来源 {key} 必须是非空字符串: spec_id={}, media_asset_id={}", spec_id, media_asset_id))?;
    if actual != expected {
        return Err(format!(
            "Galgame 视觉生成意图已完成但媒体来源 {key} 与 spec 目标不一致: spec_id={}, media_asset_id={}, actual={}, expected={}",
            spec_id, media_asset_id, actual, expected
        ));
    }
    Ok(())
}

fn validate_request_i64(
    request: &Map<String, Value>,
    key: &str,
    expected: i64,
    spec_id: i64,
    media_asset_id: i64,
) -> Result<(), String> {
    let actual = request.get(key).and_then(Value::as_i64).ok_or_else(|| {
        format!(
            "Galgame 视觉生成意图已完成但媒体来源 {key} 必须是整数: spec_id={}, media_asset_id={}",
            spec_id, media_asset_id
        )
    })?;
    if actual != expected {
        return Err(format!(
            "Galgame 视觉生成意图已完成但媒体来源 {key} 与 spec 目标不一致: spec_id={}, media_asset_id={}, actual={}, expected={}",
            spec_id, media_asset_id, actual, expected
        ));
    }
    Ok(())
}
