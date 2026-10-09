use std::collections::{HashMap, HashSet};

use a4n_domain::manga::MangaPageV4;
use a4n_domain::manga::{
    MangaPageReferenceImageV1, MangaReferenceAssetKind, MangaReferenceBindingSummary,
};
use a4n_domain::media::GeneratedImageRecord;
use a4n_error::AppError;
use serde_json::{Value, json};

use super::super::page_context::{
    character_names_for_page, filter_characters, filter_reference_assets,
};
pub(in crate::manga) fn manga_page_slots_input(
    extraction_summary: Value,
    adaptation_plan: Value,
) -> Result<Value, AppError> {
    let required = |key: &str| {
        extraction_summary
            .get(key)
            .cloned()
            .ok_or_else(|| AppError::InvalidArgument(format!("漫画分页规划上下文缺少 {key}")))
    };
    Ok(json!({
        "extraction_summary": {
            "source_events": required("source_events")?,
            "characters": required("characters")?,
            "speech": required("speech")?,
            "visual_anchors": required("visual_anchors")?,
            "items": required("items")?,
            "comic_synthesis": required("comic_synthesis")?,
        },
        "adaptation_plan": adaptation_plan
    }))
}

pub(in crate::manga) fn manga_page_drawing_prompt_page_input(
    page_plan: &MangaPageV4,
    extraction_summary: &Value,
    reference_assets: &Value,
    reference_bindings: &[MangaReferenceBindingSummary],
    system_reference_images: &[MangaPageReferenceImageV1],
    available_reference_images: &[GeneratedImageRecord],
) -> Result<Value, AppError> {
    let character_names = character_names_for_page(page_plan);
    let characters = filter_characters(
        extraction_summary.get("characters").ok_or_else(|| {
            AppError::InvalidArgument("漫画绘图上下文缺少 characters".to_string())
        })?,
        &character_names,
    )?;
    let reference_assets = filter_reference_assets(reference_assets, &character_names)?;
    let reference_images = reference_images_for_prompt(
        &reference_assets,
        reference_bindings,
        system_reference_images,
        available_reference_images,
    );
    Ok(json!({
        "current_page": page_plan,
        "page_context": {
            "characters": characters,
        },
        "reference_assets": reference_assets,
        "reference_images": reference_images,
    }))
}

fn reference_images_for_prompt(
    reference_assets: &Value,
    bindings: &[MangaReferenceBindingSummary],
    system_reference_images: &[MangaPageReferenceImageV1],
    available_images: &[GeneratedImageRecord],
) -> Value {
    let portraits = reference_assets
        .get("character_portraits")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|portrait| {
            Some((
                portrait.get("portrait_id")?.as_i64()?,
                portrait.get("prompt")?.as_str()?.trim(),
            ))
        })
        .collect::<HashMap<_, _>>();
    let image_descriptions = available_images
        .iter()
        .map(|image| (image.id, image.prompt.trim()))
        .collect::<HashMap<_, _>>();
    let mut seen = HashSet::new();
    let mut slots = Vec::new();

    for binding in bindings {
        let Some(asset) = binding
            .bound_asset
            .as_ref()
            .filter(|asset| asset.asset_kind == MangaReferenceAssetKind::CharacterPortrait)
        else {
            continue;
        };
        if !seen.insert((asset.asset_kind, asset.asset_id)) {
            continue;
        }
        slots.push(json!({
            "slot_label": format!("参考图 {}", slots.len() + 1),
            "source": "system",
            "target_kind": "character",
            "target_name": binding.character_name,
            "description": portraits.get(&asset.asset_id).copied().unwrap_or("角色主立绘"),
            "usage": "用于保持角色身份、面部外观、发型和主要服装一致",
        }));
    }

    for reference in system_reference_images {
        let key = (reference.asset.asset_kind, reference.asset.asset_id);
        if !seen.insert(key) {
            continue;
        }
        slots.push(json!({
            "slot_label": format!("参考图 {}", slots.len() + 1),
            "source": "system",
            "target_kind": reference.target_kind,
            "target_name": reference.target_name,
            "description": reference.description,
            "usage": reference.usage,
        }));
    }

    for binding in bindings {
        let mut user_assets = Vec::new();
        if let Some(asset) = binding
            .bound_asset
            .as_ref()
            .filter(|asset| asset.asset_kind == MangaReferenceAssetKind::GeneratedImage)
        {
            user_assets.push(asset);
        }
        user_assets.extend(binding.user_added_assets.iter());
        for asset in user_assets {
            if !seen.insert((asset.asset_kind, asset.asset_id)) {
                continue;
            }
            let description = match asset.asset_kind {
                MangaReferenceAssetKind::CharacterPortrait => portraits
                    .get(&asset.asset_id)
                    .copied()
                    .unwrap_or("角色立绘补充参考"),
                MangaReferenceAssetKind::GeneratedImage => image_descriptions
                    .get(&asset.asset_id)
                    .copied()
                    .filter(|text| !text.is_empty())
                    .unwrap_or("用户添加的图片参考"),
            };
            slots.push(json!({
                "slot_label": format!("参考图 {}", slots.len() + 1),
                "source": "user",
                "target_kind": "character",
                "target_name": binding.character_name,
                "description": description,
                "usage": "用户补充参考，仅辅助本页视觉一致性",
            }));
        }
    }
    Value::Array(slots)
}
