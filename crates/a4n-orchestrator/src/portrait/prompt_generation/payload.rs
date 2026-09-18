#[cfg(test)]
use std::collections::HashMap;

use a4n_error::AppError;
use serde_json::{Value, json};

#[cfg(test)]
use a4n_prompt::rendering::PromptRenderer;

use super::contracts::PortraitPromptSpec;
use a4n_domain::character::character_sheet_generation_fields;

pub(super) struct PortraitPromptPayloadInput<'a> {
    pub(super) spec: PortraitPromptSpec<'a>,
    pub(super) portrait_type_text: &'a str,
}

#[cfg(test)]
pub(in crate::portrait) fn portrait_prompt_request(
    project_id: i64,
    spec: PortraitPromptSpec<'_>,
    prompt_renderer: &dyn PromptRenderer,
) -> Result<String, AppError> {
    let payload = portrait_prompt_payload_value(PortraitPromptPayloadInput {
        spec,
        portrait_type_text: super::request::portrait_type_description(spec.portrait_type),
    })?;
    let mut variables = HashMap::new();
    variables.insert(
        "payload_json".to_string(),
        Value::String(serde_json::to_string_pretty(&payload)?),
    );
    futures::executor::block_on(prompt_renderer.render_prompt(
        "character_portrait_generation_user",
        &variables,
        Some(project_id),
    ))
}

pub(super) fn portrait_prompt_payload_value(
    input: PortraitPromptPayloadInput<'_>,
) -> Result<Value, AppError> {
    let PortraitPromptPayloadInput {
        spec,
        portrait_type_text,
    } = input;
    let mut payload = character_sheet_generation_fields();
    payload.insert(
        "context_refs".to_string(),
        json!({
            "profile_and_project": "稳定上下文包 portrait_profile_project_context",
        }),
    );
    payload.insert(
        "character_name".to_string(),
        Value::String(spec.profile_name.to_string()),
    );
    payload.insert(
        "portrait_type".to_string(),
        Value::String(portrait_type_text.to_string()),
    );
    payload.insert(
        "appearance".to_string(),
        spec.appearance
            .map(str::trim)
            .filter(|text| !text.is_empty())
            .map(|text| Value::String(text.to_string()))
            .unwrap_or(Value::Null),
    );
    payload.insert(
        "gender".to_string(),
        spec.gender
            .map(str::trim)
            .filter(|text| !text.is_empty())
            .map(|text| Value::String(text.to_string()))
            .unwrap_or(Value::Null),
    );
    payload.insert(
        "age".to_string(),
        spec.age
            .map(str::trim)
            .filter(|text| !text.is_empty())
            .map(|text| Value::String(text.to_string()))
            .unwrap_or(Value::Null),
    );
    payload.insert(
        "personality".to_string(),
        spec.personality
            .map(str::trim)
            .filter(|text| !text.is_empty())
            .map(|text| Value::String(text.to_string()))
            .unwrap_or(Value::Null),
    );
    payload.insert(
        "background".to_string(),
        spec.background
            .map(str::trim)
            .filter(|text| !text.is_empty())
            .map(|text| Value::String(text.to_string()))
            .unwrap_or(Value::Null),
    );
    payload.insert(
        "profile_context".to_string(),
        spec.profile_context
            .map(|_| json!("稳定上下文包 portrait_profile_project_context.profile_context"))
            .unwrap_or(Value::Null),
    );
    payload.insert(
        "project_context".to_string(),
        spec.project_context
            .map(|_| json!("稳定上下文包 portrait_profile_project_context.project_context"))
            .unwrap_or(Value::Null),
    );
    payload.insert(
        "art_style".to_string(),
        optional_prompt_text(spec.art_style)
            .map(Value::String)
            .unwrap_or(Value::Null),
    );
    payload.insert(
        "expression".to_string(),
        optional_prompt_text(spec.expression)
            .map(Value::String)
            .unwrap_or(Value::Null),
    );
    payload.insert(
        "existing_portrait_count".to_string(),
        spec.existing_portrait_count
            .map(|count| Value::Number(count.into()))
            .unwrap_or(Value::Null),
    );
    Ok(Value::Object(payload))
}

pub(super) fn optional_prompt_text(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|text| !text.is_empty())
        .map(ToOwned::to_owned)
}
