use std::collections::{HashMap, HashSet};

use a4n_domain::manga::{MangaPageContentV1, MangaPageV4};
use a4n_domain::manga::{
    MangaPageReferenceImageV1, MangaPageReferenceNeedV1, MangaReferenceAssetKind,
    MangaReferenceBindingSummary,
};
use a4n_domain::media::GeneratedImageRecord;
use a4n_domain::source_spans::SourceSpan;
use a4n_error::AppError;
use serde_json::{Value, json};

use super::super::page_context::{
    character_names_for_page, filter_characters, filter_reference_assets,
};
use super::super::validation::required_manga_string;
use crate::source_markers::SourceMarkerMap;

pub(in crate::manga) fn manga_page_storyboard_source_input(
    content: &MangaPageContentV1,
    extraction: &Value,
    source_spans: &[SourceSpan],
    chapter_content: &str,
    source_markers: &SourceMarkerMap,
) -> Result<Value, AppError> {
    manga_page_source_input(
        &content.source_span_ids,
        extraction,
        source_spans,
        chapter_content,
        source_markers,
    )
}

pub(super) fn manga_page_source_input(
    source_span_ids: &[String],
    extraction: &Value,
    source_spans: &[SourceSpan],
    chapter_content: &str,
    source_markers: &SourceMarkerMap,
) -> Result<Value, AppError> {
    let selected_ids = source_span_ids
        .iter()
        .map(String::as_str)
        .collect::<HashSet<_>>();
    let selected_spans = source_spans
        .iter()
        .filter(|span| selected_ids.contains(span.id.as_str()))
        .cloned()
        .collect::<Vec<_>>();
    if selected_spans.len() != selected_ids.len() {
        return Err(AppError::InvalidArgument(
            "漫画分镜来源不在当前章节中".into(),
        ));
    }
    let events = extraction
        .pointer("/source_events/events")
        .and_then(Value::as_array)
        .ok_or_else(|| {
            AppError::InvalidArgument("漫画分镜上下文缺少 source_events.events 数组".into())
        })?;
    let mut selected_events = Vec::new();
    let mut event_ids = HashSet::new();
    for event in events {
        let ids = event
            .get("source_span_ids")
            .and_then(Value::as_array)
            .ok_or_else(|| {
                AppError::InvalidArgument("漫画分镜原文事件缺少 source_span_ids 数组".into())
            })?;
        if ids
            .iter()
            .any(|id| id.as_str().is_some_and(|id| selected_ids.contains(id)))
        {
            event_ids.insert(required_manga_string(
                event,
                "event_id",
                "漫画分镜原文事件",
            )?);
            selected_events.push(event);
        }
    }
    let mut speech = serde_json::Map::new();
    for kind in ["dialogues", "narrations"] {
        let entries = extraction
            .get("speech")
            .and_then(|speech| speech.get(kind))
            .and_then(Value::as_array)
            .ok_or_else(|| {
                AppError::InvalidArgument(format!("漫画分镜上下文缺少 speech.{kind} 数组"))
            })?;
        let mut selected = Vec::new();
        for entry in entries {
            let event_id =
                required_manga_string(entry, "source_event_id", &format!("speech.{kind}"))?;
            if event_ids.contains(&event_id) {
                selected.push(entry);
            }
        }
        speech.insert(kind.into(), json!(selected));
    }
    Ok(json!({
        "source": source_markers.mark_excerpt(chapter_content, &selected_spans)?,
        "source_events": {"events": selected_events},
        "speech": speech,
    }))
}

pub(in crate::manga) fn manga_page_outline_input(
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
    reference_requirements: &[MangaPageReferenceNeedV1],
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
        "reference_requirements": reference_requirements,
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
