use a4n_core::error::AppError;
use serde_json::{Value, json};

use crate::structured_llm::json_shape_schema_from_example;

const REFINE_OUTPUT_FIELDS: [&str; 5] = [
    "style",
    "full_synopsis",
    "world_setting",
    "characters",
    "relationships",
];

pub(super) fn blueprint_refine_output_schema() -> Result<Value, AppError> {
    let mut properties = serde_json::Map::new();
    for spec in crate::blueprint::BLUEPRINT_GENERATION_STAGES {
        let example: Value = serde_json::from_str(spec.output_schema).map_err(|error| {
            AppError::ConfigError(format!("{} 输出合同不是合法 JSON：{error}", spec.title))
        })?;
        let schema = json_shape_schema_from_example(&example);
        let stage_properties = schema
            .get("properties")
            .and_then(Value::as_object)
            .ok_or_else(|| {
                AppError::ConfigError(format!("{} 输出合同缺少 properties", spec.title))
            })?;
        for (key, value) in stage_properties {
            if REFINE_OUTPUT_FIELDS.contains(&key.as_str()) {
                properties.insert(key.clone(), value.clone());
            }
        }
    }
    Ok(json!({
        "type": "object",
        "additionalProperties": false,
        "properties": properties,
    }))
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::*;

    #[test]
    fn refine_output_schema_only_exposes_patchable_blueprint_fields() {
        let schema = blueprint_refine_output_schema().unwrap();
        let properties = schema.get("properties").and_then(Value::as_object).unwrap();
        let fields = properties
            .keys()
            .map(String::as_str)
            .collect::<BTreeSet<_>>();
        let expected = REFINE_OUTPUT_FIELDS.into_iter().collect::<BTreeSet<_>>();

        assert_eq!(schema.get("type"), Some(&json!("object")));
        assert_eq!(schema.get("additionalProperties"), Some(&json!(false)));
        assert_eq!(fields, expected);
    }
}
