//! 漫画 layout / pipeline_status JSON 纯字段规则。

use a4n_core::{
    dependency_conflict::DownstreamConflictStrategy, models::chapter::chapter_manga_prompt,
};
use serde_json::{Value, json};

/// 将参考素材绑定进整页漫画 layout 的指定 slot。
pub fn bind_reference_asset_in_layout(
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

/// 必填非空字符串字段。
pub fn required_layout_string(value: &Value, key: &str) -> Result<String, String> {
    value
        .get(key)
        .and_then(Value::as_str)
        .map(str::to_string)
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| format!("漫画流水线状态字段 {key} 缺失或不是非空字符串"))
}

/// 可选非空字符串字段（null/缺省 → None）。
pub fn optional_layout_string(value: &Value, key: &str) -> Result<Option<String>, String> {
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

/// 必填 i32 字段。
pub fn required_layout_i32(value: &Value, key: &str) -> Result<i32, String> {
    let raw = value
        .get(key)
        .and_then(Value::as_i64)
        .ok_or_else(|| format!("漫画流水线状态字段 {key} 缺失或不是整数"))?;
    i32::try_from(raw).map_err(|_| format!("漫画流水线状态字段 {key} 超出 i32 范围"))
}

/// 必填布尔字段。
pub fn required_layout_bool(value: &Value, key: &str) -> Result<bool, String> {
    value
        .get(key)
        .and_then(Value::as_bool)
        .ok_or_else(|| format!("漫画流水线状态字段 {key} 缺失或不是布尔值"))
}

/// 确定整页漫画提示词变更前的下游图片处理。
pub fn resolve_manga_prompt_downstream_conflict(
    panel: &chapter_manga_prompt::Model,
    strategy: DownstreamConflictStrategy,
    operation: &str,
) -> Result<Option<i64>, String> {
    let Some(image_id) = panel.generated_image_id else {
        return Ok(None);
    };
    match strategy {
        DownstreamConflictStrategy::Reject => Err(format!(
            "整页漫画提示词 #{} 已派生成图片 #{image_id}，{operation}前需要选择：删除下游派生产物，或保留下游只覆盖源提示词",
            panel.id
        )),
        DownstreamConflictStrategy::KeepDownstream => Ok(None),
        DownstreamConflictStrategy::DeleteDownstream => Ok(Some(image_id)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bind_updates_or_appends_slot() {
        let layout = json!({"reference_bindings": [{"slot_id": "hero", "status": "missing"}]});
        let bound = json!({"asset_kind": "generated_image", "image_id": 7});
        let updated = bind_reference_asset_in_layout(Some(layout), "hero", bound).unwrap();
        assert_eq!(updated["reference_bindings"][0]["status"], json!("bound"));
        assert_eq!(
            updated["reference_bindings"][0]["asset_kind"],
            json!("generated_image")
        );

        let bound = json!({"asset_kind": "character_portrait", "portrait_id": 9});
        let updated = bind_reference_asset_in_layout(None, "hero", bound).unwrap();
        assert_eq!(updated["reference_bindings"].as_array().unwrap().len(), 1);
        assert_eq!(updated["reference_bindings"][0]["slot_id"], json!("hero"));
    }

    #[test]
    fn bind_rejects_bad_shape() {
        let err = bind_reference_asset_in_layout(Some(Value::Array(vec![])), "hero", json!({}))
            .unwrap_err();
        assert_eq!(err, "整页漫画提示词 layout 必须是 JSON 对象");
        let err = bind_reference_asset_in_layout(
            Some(json!({"reference_bindings": {"slot_id": "hero"}})),
            "hero",
            json!({"asset_kind": "generated_image"}),
        )
        .unwrap_err();
        assert_eq!(err, "整页漫画提示词 reference_bindings 必须是数组");
        let err = bind_reference_asset_in_layout(None, "hero", json!({"image_id": 1})).unwrap_err();
        assert_eq!(err, "绑定参考图素材缺少 asset_kind");
    }

    #[test]
    fn pipeline_layout_fields() {
        let layout = json!({
            "pipeline_status": "running",
            "current": 2,
            "can_cancel": true,
            "task_id": null,
        });
        assert_eq!(
            required_layout_string(&layout, "pipeline_status").unwrap(),
            "running"
        );
        assert_eq!(required_layout_i32(&layout, "current").unwrap(), 2);
        assert!(required_layout_bool(&layout, "can_cancel").unwrap());
        assert_eq!(optional_layout_string(&layout, "task_id").unwrap(), None);
    }
}
