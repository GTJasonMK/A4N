use std::collections::HashMap;

use a4n_core::error::AppError;
use serde_json::{Map, Value};

use super::CHAPTERS_PER_PART;
use super::entities::{parse_characters, parse_relationships};
use super::fields::{
    optional_string, reject_keys, reject_unknown_keys, required_array, required_bool, required_i32,
    required_object_clone, required_string, validate_chapter_count,
};
use super::types::{ParsedBlueprint, ParsedBlueprintOptimization};
use super::world::validate_world_setting;
use crate::blueprint::needs_part_outlines_for_chapters;

pub(in crate::blueprint) fn build_generation_payload(
    novel_title: &str,
    genre: &str,
    target_chapters: i32,
    variables: &HashMap<String, Value>,
) -> Result<Value, AppError> {
    validate_chapter_count(target_chapters, "requested_chapter_count")?;

    let mut payload = Map::new();
    payload.insert(
        "novel_title".to_string(),
        Value::String(novel_title.to_string()),
    );
    payload.insert("genre".to_string(), Value::String(genre.to_string()));
    payload.insert(
        "requested_chapter_count".to_string(),
        Value::from(target_chapters),
    );

    for key in [
        "inspiration_transcript",
        "conversation_state",
        "progress_summary",
        "next_question",
        "project",
    ] {
        if let Some(value) = variables.get(key) {
            payload.insert(key.to_string(), value.clone());
        }
    }

    Ok(Value::Object(payload))
}

pub(in crate::blueprint) fn parse_blueprint_json(
    content: &str,
    requested_chapters: i32,
) -> Result<ParsedBlueprint, AppError> {
    validate_chapter_count(requested_chapters, "requested_chapter_count")?;
    let raw: Value = serde_json::from_str(content.trim())
        .map_err(|e| AppError::InvalidArgument(format!("模型返回无法解析为严格蓝图 JSON: {e}")))?;
    let object = raw
        .as_object()
        .ok_or_else(|| AppError::InvalidArgument("蓝图生成结果必须是 JSON 对象".to_string()))?;
    reject_keys(object, &["novel_title", "chapter_count"], "blueprint")?;
    reject_unknown_keys(
        object,
        &[
            "title",
            "genre",
            "style",
            "tone",
            "target_audience",
            "one_sentence_summary",
            "full_synopsis",
            "total_chapters",
            "needs_part_outlines",
            "chapters_per_part",
            "chapter_outline",
            "world_setting",
            "characters",
            "relationships",
        ],
        "blueprint",
    )?;

    let total_chapters = required_i32(&raw, "total_chapters", "blueprint")?;
    validate_chapter_count(total_chapters, "blueprint.total_chapters")?;
    if total_chapters != requested_chapters {
        return Err(AppError::InvalidArgument(format!(
            "blueprint.total_chapters 必须等于 requested_chapter_count {requested_chapters}，实际为 {total_chapters}"
        )));
    }

    let needs_part_outlines = required_bool(&raw, "needs_part_outlines", "blueprint")?;
    let expected_needs_part_outlines = needs_part_outlines_for_chapters(total_chapters);
    if needs_part_outlines != expected_needs_part_outlines {
        return Err(AppError::InvalidArgument(format!(
            "blueprint.needs_part_outlines 必须为 {expected_needs_part_outlines}"
        )));
    }

    let chapters_per_part = required_i32(&raw, "chapters_per_part", "blueprint")?;
    if chapters_per_part != CHAPTERS_PER_PART {
        return Err(AppError::InvalidArgument(format!(
            "blueprint.chapters_per_part 必须为 {CHAPTERS_PER_PART}"
        )));
    }

    let chapter_outline = required_array(&raw, "chapter_outline", "blueprint")?;
    if !chapter_outline.is_empty() {
        return Err(AppError::InvalidArgument(
            "blueprint.chapter_outline 必须为空数组 []".to_string(),
        ));
    }

    let world_setting = required_object_clone(&raw, "world_setting", "blueprint")?;
    validate_world_setting(&world_setting)?;
    let characters = parse_characters(required_array(&raw, "characters", "blueprint")?)?;
    let relationships = parse_relationships(required_array(&raw, "relationships", "blueprint")?)?;

    let title = required_string(&raw, "title", "blueprint")?;
    let genre = required_string(&raw, "genre", "blueprint")?;
    let style = required_string(&raw, "style", "blueprint")?;
    let tone = required_string(&raw, "tone", "blueprint")?;
    let target_audience = required_string(&raw, "target_audience", "blueprint")?;
    let one_sentence_summary = required_string(&raw, "one_sentence_summary", "blueprint")?;
    let full_synopsis = required_string(&raw, "full_synopsis", "blueprint")?;

    Ok(ParsedBlueprint {
        raw,
        title,
        genre,
        style,
        tone,
        target_audience,
        one_sentence_summary,
        full_synopsis,
        world_setting,
        total_chapters,
        needs_part_outlines,
        chapters_per_part,
        characters,
        relationships,
    })
}

pub(in crate::blueprint) fn parse_blueprint_optimization_json(
    content: &str,
) -> Result<ParsedBlueprintOptimization, AppError> {
    let value: Value = serde_json::from_str(content.trim())
        .map_err(|e| AppError::InvalidArgument(format!("模型返回无法解析为蓝图优化 JSON: {e}")))?;

    let Some(object) = value.as_object() else {
        return Err(AppError::InvalidArgument(
            "蓝图优化结果必须是 JSON 对象".to_string(),
        ));
    };
    reject_unknown_keys(
        object,
        &[
            "style",
            "full_synopsis",
            "world_setting",
            "characters",
            "relationships",
        ],
        "blueprint_optimization",
    )?;

    if !object.contains_key("style")
        && !object.contains_key("full_synopsis")
        && !object.contains_key("world_setting")
        && !object.contains_key("characters")
        && !object.contains_key("relationships")
    {
        return Err(AppError::InvalidArgument(
            "蓝图优化结果缺少可应用字段：style、full_synopsis、world_setting、characters 或 relationships"
                .to_string(),
        ));
    }

    let style = optional_string(&value, "style")?;
    let full_synopsis = optional_string(&value, "full_synopsis")?;
    let world_setting = match value.get("world_setting") {
        Some(value) if !value.is_null() => {
            if !value.is_object() {
                return Err(AppError::InvalidArgument(
                    "blueprint_optimization.world_setting 必须是 JSON 对象".to_string(),
                ));
            }
            validate_world_setting(value)?;
            Some(value.clone())
        }
        _ => None,
    };
    let characters = match value.get("characters") {
        Some(value) if !value.is_null() => {
            Some(parse_characters(value.as_array().ok_or_else(|| {
                AppError::InvalidArgument(
                    "blueprint_optimization.characters 必须是数组".to_string(),
                )
            })?)?)
        }
        _ => None,
    };
    let relationships = match value.get("relationships") {
        Some(value) if !value.is_null() => {
            Some(parse_relationships(value.as_array().ok_or_else(|| {
                AppError::InvalidArgument(
                    "blueprint_optimization.relationships 必须是数组".to_string(),
                )
            })?)?)
        }
        _ => None,
    };

    if style.is_none()
        && full_synopsis.is_none()
        && world_setting.is_none()
        && characters.is_none()
        && relationships.is_none()
    {
        return Err(AppError::InvalidArgument(
            "蓝图优化结果必须包含至少一个非空可应用字段".to_string(),
        ));
    }

    Ok(ParsedBlueprintOptimization {
        raw: value,
        style,
        full_synopsis,
        world_setting,
        characters,
        relationships,
    })
}

pub(in crate::blueprint) fn validate_story_core_stage(value: &Value) -> Result<(), AppError> {
    for key in [
        "title",
        "target_audience",
        "genre",
        "style",
        "tone",
        "one_sentence_summary",
        "full_synopsis",
    ] {
        required_string(value, key, "blueprint")?;
    }
    Ok(())
}

pub(in crate::blueprint) fn validate_world_stage(value: &Value) -> Result<(), AppError> {
    let world_setting = required_object_clone(value, "world_setting", "blueprint")?;
    validate_world_setting(&world_setting)
}

pub(in crate::blueprint) fn validate_world_setting_payload(value: &Value) -> Result<(), AppError> {
    validate_world_setting(value)
}

pub(in crate::blueprint) fn validate_characters_stage(value: &Value) -> Result<(), AppError> {
    parse_characters(required_array(value, "characters", "blueprint")?)?;
    Ok(())
}

pub(in crate::blueprint) fn validate_characters_payload(value: &Value) -> Result<(), AppError> {
    let items = value
        .as_array()
        .ok_or_else(|| AppError::InvalidArgument("blueprint.characters 必须是数组".to_string()))?;
    parse_characters(items)?;
    Ok(())
}

pub(in crate::blueprint) fn validate_relationships_stage(value: &Value) -> Result<(), AppError> {
    parse_relationships(required_array(value, "relationships", "blueprint")?)?;
    Ok(())
}

pub(in crate::blueprint) fn validate_relationships_payload(value: &Value) -> Result<(), AppError> {
    let items = value.as_array().ok_or_else(|| {
        AppError::InvalidArgument("blueprint.relationships 必须是数组".to_string())
    })?;
    parse_relationships(items)?;
    Ok(())
}
