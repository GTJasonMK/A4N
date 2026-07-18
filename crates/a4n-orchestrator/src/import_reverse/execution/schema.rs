use a4n_core::error::AppError;
use serde_json::{Value, json};

use crate::structured_llm::{json_shape_schema_from_example, parse_strict_json_response};

pub(super) fn reverse_engineer_output_schema(prompt_name: &str) -> Result<Value, AppError> {
    match prompt_name {
        "reverse_outline" => Ok(json!({
            "type": "object",
            "additionalProperties": false,
            "required": ["outlines"],
            "properties": {
                "outlines": {
                    "type": "array",
                    "items": {
                        "type": "object",
                        "additionalProperties": false,
                        "required": ["chapter_number", "title", "summary"],
                        "properties": {
                            "chapter_number": {"type": "integer"},
                            "title": {"type": "string"},
                            "summary": {"type": "string"},
                            "word_count_target": {"type": "integer"}
                        }
                    }
                }
            }
        })),
        "reverse_part_outline" => Ok(json!({
            "type": "object",
            "additionalProperties": false,
            "required": ["parts"],
            "properties": {
                "parts": {
                    "type": "array",
                    "items": {
                        "type": "object",
                        "additionalProperties": false,
                        "required": [
                            "part_number", "title", "start_chapter", "end_chapter",
                            "summary", "theme", "key_events", "character_focus"
                        ],
                        "properties": {
                            "part_number": {"type": "integer"},
                            "title": {"type": "string"},
                            "start_chapter": {"type": "integer"},
                            "end_chapter": {"type": "integer"},
                            "summary": {"type": "string"},
                            "theme": {"type": "string"},
                            "key_events": {"type": "array", "items": {}},
                            "character_focus": {"type": "object"}
                        }
                    }
                }
            }
        })),
        "reverse_blueprint" => reverse_blueprint_output_schema(),
        other => Err(AppError::InvalidArgument(format!(
            "未注册的导入反推结构化输出合同: {other}"
        ))),
    }
}

fn reverse_blueprint_output_schema() -> Result<Value, AppError> {
    let mut properties = serde_json::Map::new();
    let mut required = Vec::new();
    for spec in crate::blueprint::BLUEPRINT_GENERATION_STAGES {
        let example: Value = parse_strict_json_response(spec.output_schema, spec.title)?;
        let schema = json_shape_schema_from_example(&example);
        let stage_properties = schema
            .get("properties")
            .and_then(Value::as_object)
            .ok_or_else(|| {
                AppError::ConfigError(format!("{} 输出合同缺少 properties", spec.title))
            })?;
        for (key, value) in stage_properties {
            properties.insert(key.clone(), value.clone());
            required.push(Value::String(key.clone()));
        }
    }
    properties.insert("total_chapters".to_string(), json!({"type": "integer"}));
    properties.insert(
        "needs_part_outlines".to_string(),
        json!({"type": "boolean"}),
    );
    Ok(json!({
        "type": "object",
        "additionalProperties": false,
        "required": required,
        "properties": properties,
    }))
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::*;

    #[test]
    fn registered_import_reverse_schemas_are_strict_objects() {
        for prompt_name in [
            "reverse_outline",
            "reverse_part_outline",
            "reverse_blueprint",
        ] {
            let schema = reverse_engineer_output_schema(prompt_name).unwrap();

            assert_eq!(schema.get("type"), Some(&json!("object")), "{prompt_name}");
            assert_eq!(
                schema.get("additionalProperties"),
                Some(&json!(false)),
                "{prompt_name}"
            );
        }
    }

    #[test]
    fn reverse_blueprint_optional_fields_are_not_required() {
        let schema = reverse_engineer_output_schema("reverse_blueprint").unwrap();
        let required = schema
            .get("required")
            .and_then(Value::as_array)
            .unwrap()
            .iter()
            .filter_map(Value::as_str)
            .collect::<BTreeSet<_>>();
        let properties = schema.get("properties").and_then(Value::as_object).unwrap();

        assert!(properties.contains_key("total_chapters"));
        assert!(properties.contains_key("needs_part_outlines"));
        assert!(!required.contains("total_chapters"));
        assert!(!required.contains("needs_part_outlines"));
    }

    #[test]
    fn rejects_unregistered_import_reverse_schema() {
        let error = reverse_engineer_output_schema("unknown_reverse").unwrap_err();

        match error {
            AppError::InvalidArgument(message) => {
                assert_eq!(message, "未注册的导入反推结构化输出合同: unknown_reverse")
            }
            other => panic!("unexpected error: {other}"),
        }
    }
}
