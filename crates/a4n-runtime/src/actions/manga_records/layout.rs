use serde_json::{Value, json};

pub(super) fn bind_reference_asset_in_layout(
    layout: Option<Value>,
    slot_id: &str,
    bound_asset: Value,
) -> Result<Value, String> {
    let mut layout = match layout {
        Some(Value::Object(map)) => map,
        Some(_) => return Err("整页漫画提示词 layout 必须是 JSON 对象".into()),
        None => serde_json::Map::new(),
    };
    let asset_kind = bound_asset
        .get("asset_kind")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToString::to_string)
        .ok_or_else(|| "绑定参考图素材缺少 asset_kind".to_string())?;

    let mut bindings = match layout.remove("reference_bindings") {
        None => Vec::new(),
        Some(Value::Array(items)) => items,
        Some(_) => return Err("整页漫画提示词 reference_bindings 必须是数组".to_string()),
    };
    let mut found = false;
    for binding in &mut bindings {
        let Some(map) = binding.as_object_mut() else {
            continue;
        };
        let current_slot = map
            .get("slot_id")
            .and_then(Value::as_str)
            .map(str::trim)
            .unwrap_or("");
        if current_slot != slot_id {
            continue;
        }
        map.insert("status".into(), json!("bound"));
        map.insert("asset_kind".into(), json!(asset_kind));
        map.insert("bound_asset".into(), bound_asset.clone());
        found = true;
        break;
    }
    if !found {
        bindings.push(json!({
            "slot_id": slot_id,
            "label": slot_id,
            "target_type": "visual",
            "target_name": slot_id,
            "purpose": "生成整页漫画时使用绑定参考图",
            "required": true,
            "status": "bound",
            "asset_kind": asset_kind,
            "bound_asset": bound_asset,
        }));
    }

    layout.insert("reference_bindings".into(), Value::Array(bindings));
    Ok(Value::Object(layout))
}

pub(super) fn required_layout_string(value: &Value, key: &str) -> Result<String, String> {
    value
        .get(key)
        .and_then(Value::as_str)
        .map(str::to_string)
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| format!("漫画流水线状态字段 {key} 缺失或不是非空字符串"))
}

pub(super) fn optional_layout_string(value: &Value, key: &str) -> Result<Option<String>, String> {
    match value.get(key) {
        Some(Value::Null) | None => Ok(None),
        Some(value) => value
            .as_str()
            .map(str::to_string)
            .filter(|value| !value.trim().is_empty())
            .map(Some)
            .ok_or_else(|| format!("漫画流水线状态字段 {key} 必须是非空字符串或 null")),
    }
}

pub(super) fn required_layout_i32(value: &Value, key: &str) -> Result<i32, String> {
    let raw = value
        .get(key)
        .and_then(Value::as_i64)
        .ok_or_else(|| format!("漫画流水线状态字段 {key} 缺失或不是整数"))?;
    i32::try_from(raw).map_err(|_| format!("漫画流水线状态字段 {key} 超出 i32 范围"))
}

pub(super) fn required_layout_bool(value: &Value, key: &str) -> Result<bool, String> {
    value
        .get(key)
        .and_then(Value::as_bool)
        .ok_or_else(|| format!("漫画流水线状态字段 {key} 缺失或不是布尔值"))
}
