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
            .map_err(|error| format!("输出不符合合同 {}：{error}", error.instance_path()))
    }
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
