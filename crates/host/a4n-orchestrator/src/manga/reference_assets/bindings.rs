use std::collections::{HashMap, HashSet};

use a4n_domain::manga::{
    MangaBoundReferenceAsset, MangaPageV4, MangaReferenceAssetKind, MangaReferenceBindingSummary,
};
use serde_json::Value;

pub(in crate::manga) fn page_reference_bindings(
    page: &MangaPageV4,
    reference_assets_metadata: &Value,
) -> Result<Vec<MangaReferenceBindingSummary>, String> {
    let portraits = reference_assets_metadata
        .get("character_portraits")
        .and_then(Value::as_array)
        .ok_or_else(|| "漫画参考素材元数据 character_portraits 必须是数组".to_string())?;
    let portrait_by_name = portrait_index(portraits);
    let mut bindings = Vec::new();

    let mut character_names = page
        .panels
        .iter()
        .flat_map(|panel| panel.participants.iter().cloned())
        .collect::<HashSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    character_names.sort();
    for name in character_names {
        bindings.push(MangaReferenceBindingSummary {
            character_name: name.clone(),
            bound_asset: character_portrait_if_available(
                &name,
                portrait_by_name.get(name.as_str()).copied(),
            )?,
        });
    }

    Ok(bindings)
}

fn character_portrait_if_available(
    name: &str,
    portrait: Option<&Value>,
) -> Result<Option<MangaBoundReferenceAsset>, String> {
    let Some(portrait) = portrait else {
        return Ok(None);
    };
    let image_path = portrait
        .get("image_path")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|path| !path.is_empty())
        .ok_or_else(|| format!("角色 {name} 的漫画参考立绘缺少 image_path"))?;
    let portrait_id = portrait
        .get("portrait_id")
        .and_then(Value::as_i64)
        .filter(|value| *value > 0)
        .ok_or_else(|| format!("角色 {name} 的漫画参考立绘 portrait_id 必须大于 0"))?;
    Ok(Some(MangaBoundReferenceAsset {
        asset_kind: MangaReferenceAssetKind::CharacterPortrait,
        asset_id: portrait_id,
        image_path: image_path.to_string(),
    }))
}

fn portrait_index(portraits: &[Value]) -> HashMap<&str, &Value> {
    portraits.iter().fold(
        HashMap::<&str, &Value>::with_capacity(portraits.len()),
        |mut index, portrait| {
            if let Some(name) = portrait
                .get("character_name")
                .and_then(Value::as_str)
                .map(str::trim)
            {
                index.entry(name).or_insert(portrait);
            }
            index
        },
    )
}

#[cfg(test)]
mod tests {
    use super::portrait_index;
    use serde_json::json;

    #[test]
    fn portrait_index_keeps_first_trimmed_name() {
        let portraits = vec![
            json!({"character_name": " hero ", "portrait_id": 1}),
            json!({"character_name": "hero", "portrait_id": 2}),
            json!({"portrait_id": 3}),
        ];
        let index = portrait_index(&portraits);

        assert_eq!(
            index
                .get("hero")
                .and_then(|value| value.get("portrait_id"))
                .and_then(serde_json::Value::as_i64),
            Some(1)
        );
        assert!(!index.contains_key(""));
    }
}
