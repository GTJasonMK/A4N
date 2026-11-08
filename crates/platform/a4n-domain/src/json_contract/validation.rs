use serde_json::Value;

/// The exact schema sent to a model also validates the adopted response.
/// Source references and cross-record invariants remain with the business validator.
pub struct OutputSchemaValidator(jsonschema::Validator);

impl OutputSchemaValidator {
    pub fn new(schema: &Value) -> Result<Self, String> {
        reject_external_refs(schema)?;
        jsonschema::validator_for(schema)
            .map(Self)
            .map_err(|error| format!("输出 Schema 无效：{error}"))
    }

    pub fn validate(&self, value: &Value) -> Result<(), String> {
        self.0
            .validate(value)
            .map_err(|error| validation_message(&error, value))
    }
}

fn property_path(path: &mut String, property: &str) {
    let mut chars = property.chars();
    let identifier = chars
        .next()
        .is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
        && chars.all(|c| c.is_ascii_alphanumeric() || c == '_');
    if identifier {
        if !path.is_empty() {
            path.push('.');
        }
        path.push_str(property);
    } else {
        path.push_str(&format!(
            "[{}]",
            serde_json::to_string(property).expect("property string")
        ));
    }
}

fn validation_message(error: &jsonschema::ValidationError<'_>, value: &Value) -> String {
    use jsonschema::{
        JsonType,
        error::{TypeKind, ValidationErrorKind},
    };
    let mut path = String::new();
    let mut current = Some(value);
    for token in error.instance_path().as_str().split('/').skip(1) {
        let property = token.replace("~1", "/").replace("~0", "~");
        if let Some(Value::Array(items)) = current {
            let index = property.parse::<usize>().expect("validated array index");
            path.push_str(&format!("[{index}]"));
            current = items.get(index);
        } else {
            property_path(&mut path, &property);
            current = current.and_then(|value| value.get(&property));
        }
    }
    if let ValidationErrorKind::Required {
        property: Value::String(property),
    } = error.kind()
    {
        property_path(&mut path, property);
        return format!("输出不符合合同：{path} 缺少字段");
    }
    let path = if path.is_empty() { "输出" } else { &path };
    if let ValidationErrorKind::Type {
        kind: TypeKind::Single(JsonType::Integer),
    } = error.kind()
    {
        return format!("输出不符合合同：{path} 必须是 JSON 整数数字");
    }
    format!("输出不符合合同 {path}：{}", error.masked())
}

fn reject_external_refs(value: &Value) -> Result<(), String> {
    match value {
        Value::Object(fields) => {
            for key in ["$ref", "$dynamicRef", "$recursiveRef"] {
                if let Some(reference) = fields.get(key).and_then(Value::as_str)
                    && !reference.starts_with('#')
                {
                    return Err("输出 Schema 只能引用文档内部定义".into());
                }
            }
            for child in fields.values() {
                reject_external_refs(child)?;
            }
        }
        Value::Array(items) => {
            for child in items {
                reject_external_refs(child)?;
            }
        }
        _ => {}
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn reports_stable_required_and_integer_paths_without_exposing_model_values() {
        let schema = json!({"type":"object","properties":{"characters":{"type":"array","items":{"type":"object","required":["evidence_anchor_id","exit_state"],"properties":{"evidence_anchor_id":{"type":"integer"},"exit_state":{"type":"object"}}}}}});
        let validator = OutputSchemaValidator::new(&schema).unwrap();
        let error = validator
            .validate(
                &json!({"characters":[{"evidence_anchor_id":"secret-source","exit_state":{}}]}),
            )
            .unwrap_err();
        assert!(
            error.contains("characters[0].evidence_anchor_id 必须是 JSON 整数数字"),
            "{error}"
        );
        assert!(!error.contains("secret-source"));
        let error = validator
            .validate(&json!({"characters":[{"evidence_anchor_id":1}]}))
            .unwrap_err();
        assert!(
            error.contains("characters[0].exit_state 缺少字段"),
            "{error}"
        );
        let validator = OutputSchemaValidator::new(
            &json!({"properties":{"a/b":{"properties":{"0":{"type":"integer"}}}}}),
        )
        .unwrap();
        let error = validator
            .validate(&json!({"a/b":{"0":"secret"}}))
            .unwrap_err();
        assert!(
            error.contains("[\"a/b\"][\"0\"] 必须是 JSON 整数数字"),
            "{error}"
        );
    }

    #[test]
    fn enforces_nested_required_optional_nullable_enum_bounds_and_unique_arrays() {
        let validator = OutputSchemaValidator::new(&json!({
            "type":"object", "required":["state","ids"], "additionalProperties":false,
            "properties":{
                "state":{"type":"object"},
                "note":{"type":["string","null"],"minLength":1},
                "ids":{"type":"array","minItems":1,"uniqueItems":true,"items":{"type":"integer","minimum":1}},
                "kind":{"enum":["event","object"]}
            }
        })).unwrap();
        assert!(validator.validate(&json!({"state":{},"ids":[1]})).is_ok());
        assert!(
            validator
                .validate(&json!({"state":{},"ids":[1],"note":null}))
                .is_ok()
        );
        for invalid in [
            json!({"ids":[1]}),
            json!({"state":null,"ids":[1]}),
            json!({"state":{},"ids":["1"]}),
            json!({"state":{},"ids":[0]}),
            json!({"state":{},"ids":[1,1]}),
            json!({"state":{},"ids":[]}),
            json!({"state":{},"ids":[1],"note":""}),
            json!({"state":{},"ids":[1],"kind":"unknown"}),
            json!({"state":{},"ids":[1],"extra":true}),
        ] {
            assert!(validator.validate(&invalid).is_err(), "{invalid}");
        }
        assert!(OutputSchemaValidator::new(&json!({"$ref":"https://example.org/schema"})).is_err());
        assert!(OutputSchemaValidator::new(&json!({"$ref":"file:///etc/passwd"})).is_err());
    }
}
