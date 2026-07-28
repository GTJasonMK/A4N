use std::collections::BTreeSet;

use a4n_core::{
    json_contract::{
        optional_string, positive_i64_array, reject_unknown_fields, required_string, string_array,
    },
    models::{chapter::chapter, project::text_anchor},
    profile_text::normalize_importance_value,
    source_spans::anchor_quotes_with_chapter_numbers,
};
use serde_json::Value;

use crate::{
    asset::{OBJECT_TYPES, ObjectProfileDraft, normalize_object_type_value},
    object_discovery::rules::validate_object_discovery_chapter_scope,
};

use super::contracts::ParsedObjectProfileDraft;

pub(super) fn parse_and_validate_object_discovery_output(
    content: &str,
    analyzed_chapters: &BTreeSet<i32>,
    chapters: &[chapter::Model],
    text_anchors: &[text_anchor::Model],
) -> Result<(Vec<ParsedObjectProfileDraft>, Value), String> {
    let value: Value = serde_json::from_str(content.trim()).map_err(|error| {
        let mut message = format!("物体档案发现结果必须是 JSON 对象: {error}");
        if error.is_eof() {
            message.push_str("；响应在 JSON 闭合前结束，通常是模型输出被截断或 max_tokens 不足");
        }
        message
    })?;
    if !value.is_object() {
        return Err("物体档案发现结果必须是 JSON 对象".to_string());
    }
    reject_unknown_fields(&value, "root", &["objects"])?;
    let objects = value
        .get("objects")
        .and_then(Value::as_array)
        .ok_or_else(|| "物体档案发现字段 objects 必须是数组".to_string())?;
    let mut seen = BTreeSet::new();
    let mut drafts = Vec::with_capacity(objects.len());
    for (index, object) in objects.iter().enumerate() {
        let draft = normalize_object(index, object, chapters, text_anchors)?;
        let key = draft.draft.name.trim().to_lowercase();
        if !seen.insert(key) {
            return Err(format!("objects[{index}] 与前序物体重复"));
        }
        drafts.push(draft);
    }
    let chapter_ranges = drafts
        .iter()
        .map(|item| (item.draft.first_seen_chapter, item.draft.last_seen_chapter))
        .collect::<Vec<_>>();
    validate_object_discovery_chapter_scope(&chapter_ranges, analyzed_chapters)?;
    Ok((drafts, value))
}

fn normalize_object(
    index: usize,
    object: &Value,
    chapters: &[chapter::Model],
    text_anchors: &[text_anchor::Model],
) -> Result<ParsedObjectProfileDraft, String> {
    let path = format!("objects[{index}]");
    if !object.is_object() {
        return Err(format!("{path} 必须是 JSON 对象"));
    }
    let name = required_string(object, "name", &path)?;
    reject_unknown_fields(
        object,
        &path,
        &[
            "name",
            "aliases",
            "object_type",
            "importance",
            "visual_description",
            "material_and_state",
            "story_usage",
            "visual_usage",
            "generation_usage",
            "location_context",
            "evidence_anchor_ids",
        ],
    )?;
    let evidence_anchor_ids = positive_i64_array(object, "evidence_anchor_ids", &path, true)?;
    let evidence_with_chapters = anchor_quotes_with_chapter_numbers(
        &format!("{path}.evidence_anchor_ids"),
        &evidence_anchor_ids,
        chapters,
        text_anchors,
    )?;
    let first_seen_chapter = evidence_with_chapters
        .iter()
        .map(|anchor| anchor.chapter_number)
        .min()
        .ok_or_else(|| format!("{path}.evidence_anchor_ids 不能为空"))?;
    let last_seen_chapter = evidence_with_chapters
        .iter()
        .map(|anchor| anchor.chapter_number)
        .max()
        .ok_or_else(|| format!("{path}.evidence_anchor_ids 不能为空"))?;
    let evidence = evidence_with_chapters
        .iter()
        .map(|anchor| anchor.quote.clone())
        .collect::<Vec<_>>();
    let aliases = string_array(object, "aliases", &path, false)?;
    let object_type = object_type_string(object, "object_type", &path)?;
    let importance = importance_string(object, "importance", &path)?;
    let visual_description = required_string(object, "visual_description", &path)?;
    let material_and_state = required_string(object, "material_and_state", &path)?;
    let story_usage = required_string(object, "story_usage", &path)?;
    let visual_usage = required_string(object, "visual_usage", &path)?;
    let generation_usage = required_string(object, "generation_usage", &path)?;
    let location_context = optional_string(object, "location_context", &path)?;
    let image_prompt_seed = object_image_prompt_seed(
        &name,
        &visual_description,
        &material_and_state,
        &generation_usage,
    );
    Ok(ParsedObjectProfileDraft {
        evidence_anchor_ids,
        draft: ObjectProfileDraft {
            aliases,
            object_type,
            importance,
            visual_description,
            material_and_state,
            story_usage,
            visual_usage,
            generation_usage,
            location_context,
            first_seen_chapter,
            last_seen_chapter,
            evidence,
            image_prompt_seed,
            name,
        },
    })
}

fn object_image_prompt_seed(
    name: &str,
    visual_description: &str,
    material_and_state: &str,
    generation_usage: &str,
) -> String {
    format!(
        "{name}，{visual_description}，{material_and_state}，{generation_usage}，可复用物体素材设定图"
    )
}

fn object_type_string(value: &Value, key: &str, path: &str) -> Result<String, String> {
    let text = required_string(value, key, path)?;
    normalize_object_type_value(&text).map_err(|_| {
        format!(
            "{path}.{key} 无效：{}；允许值：{}",
            text,
            OBJECT_TYPES.join("/")
        )
    })
}

fn importance_string(value: &Value, key: &str, path: &str) -> Result<String, String> {
    let text = required_string(value, key, path)?;
    normalize_importance_value(&text).map_err(|_| {
        format!(
            "{path}.{key} 无效：{}；允许值：primary/secondary/minor",
            text
        )
    })
}

#[cfg(test)]
pub(super) fn parse_and_validate_object_discovery_content(
    content: &str,
    analyzed_chapters: &BTreeSet<i32>,
    chapters: &[chapter::Model],
    text_anchors: &[text_anchor::Model],
) -> Result<Vec<ParsedObjectProfileDraft>, String> {
    parse_and_validate_object_discovery_output(content, analyzed_chapters, chapters, text_anchors)
        .map(|(drafts, _)| drafts)
}
