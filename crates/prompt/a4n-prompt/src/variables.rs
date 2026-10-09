use std::collections::HashMap;

use a4n_error::AppError;
use serde_json::{Map, Value};

pub use crate::tera_engine::variables::{
    required_template_variables, template_variable_access_paths,
};

const DEFAULT_INVALID_MESSAGE: &str = "variables 必须是对象或 null";

pub fn parse_template_variables(value: Option<&Value>) -> Result<HashMap<String, Value>, String> {
    match value {
        None | Some(Value::Null) => Ok(HashMap::new()),
        Some(Value::Object(map)) => Ok(map
            .iter()
            .map(|(key, value)| (key.clone(), value.clone()))
            .collect()),
        Some(_) => Err(DEFAULT_INVALID_MESSAGE.to_string()),
    }
}

pub fn parse_template_variables_app_error(
    value: &Option<Value>,
) -> Result<HashMap<String, Value>, AppError> {
    parse_template_variables(value.as_ref()).map_err(AppError::InvalidArgument)
}

pub fn placeholder_template_variables(
    variables: &[String],
    access_paths: &[Vec<String>],
) -> HashMap<String, Value> {
    let mut result = variables
        .iter()
        .map(|name| (name.clone(), Value::String(format!("⟦{name}⟧"))))
        .collect::<HashMap<_, _>>();
    for path in access_paths {
        let Some((root, fields)) = path.split_first() else {
            continue;
        };
        if fields.is_empty() {
            continue;
        }
        let Some(value) = result.get_mut(root) else {
            continue;
        };
        insert_nested_placeholder(value, fields, &path.join("."));
    }
    result
}

fn insert_nested_placeholder(current: &mut Value, path: &[String], label: &str) {
    if !current.is_object() {
        *current = Value::Object(Map::new());
    }
    let Some(object) = current.as_object_mut() else {
        return;
    };
    let Some((field, remaining)) = path.split_first() else {
        return;
    };
    if remaining.is_empty() {
        object.insert(field.clone(), Value::String(format!("⟦{label}⟧")));
    } else {
        insert_nested_placeholder(
            object.entry(field.clone()).or_insert(Value::Null),
            remaining,
            label,
        );
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn missing_and_null_variables_produce_empty_maps() {
        assert!(parse_template_variables(None).unwrap().is_empty());
        assert!(
            parse_template_variables(Some(&Value::Null))
                .unwrap()
                .is_empty()
        );
    }

    #[test]
    fn object_variables_preserve_values() {
        let variables = parse_template_variables(Some(&json!({
            "tone": "warm",
            "limits": {"max": 3},
        })))
        .expect("object variables should parse");

        assert_eq!(variables["tone"], "warm");
        assert_eq!(variables["limits"]["max"], 3);
    }

    #[test]
    fn invalid_values_preserve_default_error_contract() {
        let value = json!(["tone"]);
        assert_eq!(
            parse_template_variables(Some(&value)).unwrap_err(),
            DEFAULT_INVALID_MESSAGE
        );
    }

    #[test]
    fn preview_variables_preserve_nested_template_access() {
        let variables = vec!["scene".to_string(), "line".to_string(), "tone".to_string()];
        let paths = vec![
            vec!["scene".to_string(), "title".to_string()],
            vec!["line".to_string(), "text".to_string()],
        ];

        let preview = placeholder_template_variables(&variables, &paths);

        assert_eq!(preview["scene"]["title"], "⟦scene.title⟧");
        assert_eq!(preview["line"]["text"], "⟦line.text⟧");
        assert_eq!(preview["tone"], "⟦tone⟧");
    }
}
