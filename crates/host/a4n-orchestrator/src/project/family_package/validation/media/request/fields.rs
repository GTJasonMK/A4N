use serde_json::{Map, Value};
use std::collections::HashSet;

pub(super) fn validate_optional_media_request_id_ref(
    request: &Map<String, Value>,
    key: &str,
    valid_ids: &HashSet<i64>,
    label: &str,
    media_asset_id: i64,
) -> Result<(), String> {
    let Some(value) = optional_media_request_i64(request, key, media_asset_id)? else {
        return Ok(());
    };
    if !valid_ids.contains(&value) {
        return Err(format!(
            "项目族导入包媒体来源 request_snapshot_json.{key} 引用的{label}不存在: media_asset_id={}, {key}={}",
            media_asset_id, value
        ));
    }
    Ok(())
}

pub(super) fn optional_media_request_i64(
    request: &Map<String, Value>,
    key: &str,
    media_asset_id: i64,
) -> Result<Option<i64>, String> {
    let Some(value) = request.get(key) else {
        return Ok(None);
    };
    if value.is_null() {
        return Ok(None);
    }
    value.as_i64().map(Some).ok_or_else(|| {
        format!(
            "项目族导入包媒体来源 request_snapshot_json.{key} 必须是整数: media_asset_id={media_asset_id}"
        )
    })
}

pub(super) fn required_media_request_i64(
    request: &Map<String, Value>,
    key: &str,
    media_asset_id: i64,
) -> Result<i64, String> {
    optional_media_request_i64(request, key, media_asset_id)?.ok_or_else(|| {
        format!(
            "项目族导入包媒体来源 request_snapshot_json.{key} 缺失: media_asset_id={media_asset_id}"
        )
    })
}

pub(super) fn required_media_request_string<'a>(
    request: &'a Map<String, Value>,
    key: &str,
    media_asset_id: i64,
) -> Result<&'a str, String> {
    let Some(value) = request.get(key) else {
        return Err(format!(
            "项目族导入包媒体来源 request_snapshot_json.{key} 缺失: media_asset_id={media_asset_id}"
        ));
    };
    value
        .as_str()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| {
            format!(
                "项目族导入包媒体来源 request_snapshot_json.{key} 必须是非空字符串: media_asset_id={media_asset_id}"
            )
        })
}
