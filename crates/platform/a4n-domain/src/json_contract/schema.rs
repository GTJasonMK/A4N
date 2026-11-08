use serde_json::{Map, Value, json};

/// Projects machine-readable field constraints from the same contract sent in
/// the task payload. Unspecified business fields remain the domain validator's responsibility.
pub fn array_output_contract_schema(contract: &Value) -> Result<Value, String> {
    let root = contract["required_root_field"]
        .as_str()
        .filter(|root| !root.is_empty())
        .ok_or("数组输出合同缺少 required_root_field")?;
    let mut properties = match contract.get("field_types") {
        None => Map::new(),
        Some(Value::Object(fields)) => fields.clone(),
        Some(_) => return Err("输出合同 field_types 必须为对象".into()),
    };
    let mut required = Vec::new();
    for key in [
        "item_required_fields",
        "character_required_fields",
        "object_required_fields",
        "object_fields",
    ] {
        let Some(fields) = contract.get(key) else {
            continue;
        };
        for field in fields
            .as_array()
            .ok_or_else(|| format!("{key} 必须为数组"))?
        {
            let field = field
                .as_str()
                .ok_or_else(|| format!("{key} 必须只包含字段名称"))?;
            if !required
                .iter()
                .any(|item: &Value| item.as_str() == Some(field))
            {
                required.push(json!(field));
            }
            properties.entry(field.to_string()).or_insert(json!({}));
        }
    }
    for field in properties.keys() {
        if !required.iter().any(|item| item.as_str() == Some(field)) {
            required.push(json!(field));
        }
    }
    Ok(json!({
        "type": "object", "required": [root], "additionalProperties": false,
        "properties": { root: {
            "type": "array", "items": {
                "type": "object", "properties": properties, "required": required, "additionalProperties": false,
            }
        }}
    }))
}

/// Existing output-shape contracts use type names, enum alternatives and one
/// representative array element. This projection keeps their schema in sync.
pub fn json_schema_from_output_shape(shape: &Value) -> Result<Value, String> {
    match shape {
        Value::Object(fields) => {
            let properties = fields
                .iter()
                .map(|(key, value)| Ok((key.clone(), json_schema_from_output_shape(value)?)))
                .collect::<Result<Map<_, _>, String>>()?;
            Ok(
                json!({"type":"object","properties":properties,"required":fields.keys().collect::<Vec<_>>(),"additionalProperties":false}),
            )
        }
        Value::Array(items) if items.len() == 1 => {
            Ok(json!({"type":"array","items":json_schema_from_output_shape(&items[0])?}))
        }
        Value::Array(_) => Err("输出形状数组必须包含一个元素的类型合同".into()),
        Value::String(kind) if kind == "string" => Ok(json!({"type":"string"})),
        Value::String(choices) if choices.contains('|') => {
            Ok(json!({"type":"string","enum":choices.split('|').collect::<Vec<_>>()}))
        }
        Value::String(value) => Ok(json!({"type":"string","const":value})),
        Value::Number(number) if number.is_i64() || number.is_u64() => {
            Ok(json!({"type":"integer"}))
        }
        Value::Number(_) => Ok(json!({"type":"number"})),
        Value::Bool(_) => Ok(json!({"type":"boolean"})),
        Value::Null => Ok(json!({"type":"null"})),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn output_shape_retains_nested_types_and_enum_choices() {
        let schema = json_schema_from_output_shape(
            &json!({"items":[{"name":"string","role":"primary|minor","score":1}]}),
        )
        .unwrap();
        let item = &schema["properties"]["items"]["items"];
        assert_eq!(
            item["properties"]["role"]["enum"],
            json!(["primary", "minor"])
        );
        assert_eq!(item["properties"]["score"]["type"], "integer");
        assert_eq!(item["additionalProperties"], false);
        assert!(json_schema_from_output_shape(&json!({"items":[]})).is_err());
    }

    #[test]
    fn array_schema_preserves_shared_id_constraints_and_required_fields() {
        let schema = array_output_contract_schema(&json!({
            "required_root_field":"characters", "character_required_fields":["name","evidence_anchor_id"],
            "field_types":{"evidence_anchor_id":super::super::positive_i64_schema()}
        })).unwrap();
        assert_eq!(
            schema["properties"]["characters"]["items"]["properties"]["evidence_anchor_id"],
            super::super::positive_i64_schema()
        );
        assert_eq!(
            schema["properties"]["characters"]["items"]["required"],
            json!(["name", "evidence_anchor_id"])
        );
        assert_eq!(schema["additionalProperties"], false);
        assert!(array_output_contract_schema(&json!({})).is_err());
        assert!(
            array_output_contract_schema(&json!({"required_root_field":"events","field_types":[]}))
                .is_err()
        );
    }
}
