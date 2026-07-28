use serde_json::Value;

use super::super::*;

pub(super) fn validate_snapshot_i64_ref(
    media_asset_id: i64,
    object: &serde_json::Map<String, Value>,
    label: &str,
    key: &str,
    ids: &HashSet<i64>,
    target_label: &str,
) -> Result<(), String> {
    let Some(value) = object.get(key) else {
        return Ok(());
    };
    let id = value.as_i64().ok_or_else(|| {
        format!(
            "媒体来源 request_snapshot_json.{label} 必须是整数 ID: media_asset_id={media_asset_id}"
        )
    })?;
    if !ids.contains(&id) {
        return Err(format!(
            "媒体来源 request_snapshot_json.{label} 引用了不存在的{target_label}: media_asset_id={media_asset_id}, id={id}"
        ));
    }
    Ok(())
}

pub(super) fn validate_snapshot_i64_array_ref(
    media_asset_id: i64,
    object: &serde_json::Map<String, Value>,
    label: &str,
    key: &str,
    ids: &HashSet<i64>,
    target_label: &str,
) -> Result<(), String> {
    let Some(value) = object.get(key) else {
        return Ok(());
    };
    let values = value.as_array().ok_or_else(|| {
        format!(
            "媒体来源 request_snapshot_json.{label} 必须是整数 ID 数组: media_asset_id={media_asset_id}"
        )
    })?;
    for (index, value) in values.iter().enumerate() {
        let id = value.as_i64().ok_or_else(|| {
            format!(
                "媒体来源 request_snapshot_json.{label}[{index}] 必须是整数 ID: media_asset_id={media_asset_id}"
            )
        })?;
        if !ids.contains(&id) {
            return Err(format!(
                "媒体来源 request_snapshot_json.{label}[{index}] 引用了不存在的{target_label}: media_asset_id={media_asset_id}, id={id}"
            ));
        }
    }
    Ok(())
}
