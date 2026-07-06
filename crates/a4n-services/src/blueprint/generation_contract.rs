use std::collections::HashMap;

use serde_json::{Map, Value, json};
use sha2::{Digest, Sha256};

use a4n_core::error::AppError;
use a4n_engine::prompt::engine::PromptEngine;

use super::json::CHAPTERS_PER_PART;
use crate::blueprint::needs_part_outlines_for_chapters;

pub(super) const STORY_CORE_STAGE_KEYS: &[&str] = &[
    "title",
    "target_audience",
    "genre",
    "style",
    "tone",
    "one_sentence_summary",
    "full_synopsis",
];
pub(super) const WORLD_STAGE_KEYS: &[&str] = &["world_setting"];
pub(super) const CHARACTERS_STAGE_KEYS: &[&str] = &["characters"];
pub(super) const RELATIONSHIPS_STAGE_KEYS: &[&str] = &["relationships"];

pub(super) fn blueprint_generation_source_hash(value: &Value) -> Result<String, AppError> {
    let raw = serde_json::to_string(value)?;
    let mut hasher = Sha256::new();
    hasher.update(raw.as_bytes());
    Ok(format!("{:x}", hasher.finalize()))
}

pub(super) fn build_blueprint_stage_manifest_user_message(
    prompt_engine: &dyn PromptEngine,
    project_id: Option<i64>,
    stage_key: &str,
    stage_name: &str,
    output_schema: &str,
) -> Result<String, AppError> {
    let output_schema: Value = serde_json::from_str(output_schema).map_err(|e| {
        AppError::InvalidArgument(format!("{stage_name} 输出 schema 不是合法 JSON: {e}"))
    })?;
    let payload = json!({
        "context_refs": {
            "source_input": "稳定上下文包 blueprint_project_seed_context.source_input",
            "previous_context": "稳定上下文包 blueprint_previous_stage_outputs，仅在当前阶段需要上游阶段时存在",
        },
        "current_stage": {
            "key": stage_key,
            "name": stage_name,
            "output_schema": output_schema,
        },
    });
    let payload = serde_json::to_string_pretty(&payload)?;
    let variables = HashMap::from([
        ("stage_key".to_string(), json!(stage_key)),
        ("payload".to_string(), json!(payload)),
    ]);
    prompt_engine.render("blueprint_stage_user", &variables, project_id)
}

pub(super) fn parse_stage_json_object(
    content: &str,
    context: &str,
    expected_top_level_keys: &[&str],
) -> Result<Value, AppError> {
    let value: Value = serde_json::from_str(content.trim()).map_err(|error| {
        AppError::InvalidArgument(format!("{context} 输出无法解析为严格 JSON 对象: {error}"))
    })?;
    validate_stage_json_object(value, context, expected_top_level_keys)
}

pub(super) fn validate_stage_json_object(
    value: Value,
    context: &str,
    expected_top_level_keys: &[&str],
) -> Result<Value, AppError> {
    let object = value
        .as_object()
        .ok_or_else(|| AppError::InvalidArgument(format!("{context} 输出必须是 JSON 对象")))?;
    for key in object.keys() {
        if !expected_top_level_keys.contains(&key.as_str()) {
            return Err(AppError::InvalidArgument(format!(
                "{context} 输出包含未声明顶层字段 {key}"
            )));
        }
    }
    for key in expected_top_level_keys {
        if !object.contains_key(*key) {
            return Err(AppError::InvalidArgument(format!(
                "{context} 输出缺少必填顶层字段 {key}"
            )));
        }
    }
    if object.is_empty() {
        return Err(AppError::InvalidArgument(format!(
            "{context} 输出不能为空 JSON 对象"
        )));
    }
    Ok(value)
}

pub(super) fn blueprint_refine_manifest_user_message(
    prompt_engine: &dyn PromptEngine,
    project_id: Option<i64>,
    focus: Option<&str>,
) -> Result<String, AppError> {
    let mut payload = serde_json::Map::new();
    payload.insert(
        "context_refs".to_string(),
        json!({
            "blueprint_context": "稳定上下文包 existing_blueprint_context",
        }),
    );
    if let Some(focus) = focus.map(str::trim).filter(|value| !value.is_empty()) {
        payload.insert("focus".to_string(), json!(focus));
    }
    payload.insert(
        "output_contract".to_string(),
        json!({
            "root_type": "object",
            "allowed_fields": ["title", "genre", "style", "tone", "target_audience", "one_sentence_summary", "full_synopsis", "world_setting", "characters", "relationships"],
            "forbidden": ["markdown", "explanation", "legacy_alias_fields"],
        }),
    );
    let payload = serde_json::to_string_pretty(&Value::Object(payload))?;
    let variables = HashMap::from([("payload".to_string(), json!(payload))]);
    prompt_engine.render("blueprint_optimization_user", &variables, project_id)
}

pub(super) fn assemble_staged_blueprint_json(
    novel_title: &str,
    genre: &str,
    target_length: i32,
    core_output: &Value,
    world_output: &Value,
    characters_output: &Value,
    relationships_output: &Value,
) -> Value {
    let mut blueprint = Map::new();
    for key in [
        "title",
        "target_audience",
        "genre",
        "style",
        "tone",
        "one_sentence_summary",
        "full_synopsis",
    ] {
        if let Some(value) = core_output.get(key) {
            blueprint.insert(key.to_string(), value.clone());
        }
    }
    blueprint
        .entry("title".to_string())
        .or_insert_with(|| Value::String(novel_title.to_string()));
    blueprint
        .entry("genre".to_string())
        .or_insert_with(|| Value::String(genre.to_string()));

    if let Some(value) = world_output.get("world_setting") {
        blueprint.insert("world_setting".to_string(), value.clone());
    }
    if let Some(value) = characters_output.get("characters") {
        blueprint.insert("characters".to_string(), value.clone());
    }
    if let Some(value) = relationships_output.get("relationships") {
        blueprint.insert("relationships".to_string(), value.clone());
    }

    blueprint.insert("chapter_outline".to_string(), Value::Array(Vec::new()));
    blueprint.insert(
        "needs_part_outlines".to_string(),
        Value::Bool(needs_part_outlines_for_chapters(target_length)),
    );
    blueprint.insert("total_chapters".to_string(), Value::from(target_length));
    blueprint.insert(
        "chapters_per_part".to_string(),
        Value::from(CHAPTERS_PER_PART),
    );

    Value::Object(blueprint)
}
