//! 漫画页面 layout JSON 纯字段规则。

use a4n_core::models::chapter::chapter_manga_prompt;
use a4n_domain::{DownstreamConflictStrategy, manga::MangaBoundReferenceAsset};
use serde_json::Value;

/// 将参考素材绑定进整页漫画 layout 的指定角色。
pub fn bind_reference_asset_in_layout(
    layout: Value,
    character_name: &str,
    bound_asset: MangaBoundReferenceAsset,
) -> Result<Value, String> {
    let mut layout = match layout {
        Value::Object(map) => map,
        _ => return Err("整页漫画提示词 layout 必须是 JSON 对象".into()),
    };
    let bound_asset = serde_json::to_value(bound_asset)
        .map_err(|error| format!("序列化漫画参考图绑定失败: {error}"))?;

    let mut bindings = match layout.remove("reference_bindings") {
        Some(Value::Array(items)) => items,
        None => return Err("整页漫画提示词缺少 reference_bindings".to_string()),
        Some(_) => return Err("整页漫画提示词 reference_bindings 必须是数组".to_string()),
    };
    for (index, binding) in bindings.iter_mut().enumerate() {
        let map = binding
            .as_object_mut()
            .ok_or_else(|| format!("整页漫画提示词 reference_bindings[{index}] 必须是对象"))?;
        let current_character_name = map
            .get("character_name")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .ok_or_else(|| {
                format!(
                    "整页漫画提示词 reference_bindings[{index}].character_name 必须是非空字符串"
                )
            })?;
        if current_character_name != character_name {
            continue;
        }
        map.insert("bound_asset".into(), bound_asset);
        layout.insert("reference_bindings".into(), Value::Array(bindings));
        return Ok(Value::Object(layout));
    }

    Err(format!("整页漫画提示词不存在角色参考图: {character_name}"))
}

/// 确定整页漫画提示词变更前的下游图片处理。
pub fn resolve_manga_prompt_downstream_conflict(
    page_prompt: &chapter_manga_prompt::Model,
    strategy: DownstreamConflictStrategy,
    operation: &str,
) -> Result<Option<i64>, String> {
    let Some(image_id) = page_prompt.generated_image_id else {
        return Ok(None);
    };
    match strategy {
        DownstreamConflictStrategy::Reject => Err(format!(
            "整页漫画提示词 #{} 已派生成图片 #{image_id}，{operation}前需要选择：删除下游派生产物，或保留下游只覆盖源提示词",
            page_prompt.id
        )),
        DownstreamConflictStrategy::KeepDownstream => Ok(None),
        DownstreamConflictStrategy::DeleteDownstream => Ok(Some(image_id)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use a4n_domain::manga::MangaReferenceAssetKind;
    use serde_json::json;

    #[test]
    fn bind_updates_existing_character() {
        let layout = json!({"reference_bindings": [{"character_name": "hero"}]});
        let bound = MangaBoundReferenceAsset {
            asset_kind: MangaReferenceAssetKind::GeneratedImage,
            asset_id: 7,
            image_path: "hero.png".to_string(),
        };
        let updated = bind_reference_asset_in_layout(layout, "hero", bound).unwrap();
        assert_eq!(
            updated["reference_bindings"][0]["bound_asset"]["asset_id"],
            7
        );
    }

    #[test]
    fn bind_rejects_bad_shape() {
        let bound = || MangaBoundReferenceAsset {
            asset_kind: MangaReferenceAssetKind::GeneratedImage,
            asset_id: 7,
            image_path: "hero.png".to_string(),
        };
        let err =
            bind_reference_asset_in_layout(Value::Array(vec![]), "hero", bound()).unwrap_err();
        assert_eq!(err, "整页漫画提示词 layout 必须是 JSON 对象");
        let err = bind_reference_asset_in_layout(
            json!({"reference_bindings": {"character_name": "hero"}}),
            "hero",
            bound(),
        )
        .unwrap_err();
        assert_eq!(err, "整页漫画提示词 reference_bindings 必须是数组");
        let err = bind_reference_asset_in_layout(
            json!({"reference_bindings": [{"character_name": "other"}]}),
            "hero",
            bound(),
        )
        .unwrap_err();
        assert_eq!(err, "整页漫画提示词不存在角色参考图: hero");
    }
}
