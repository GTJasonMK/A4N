use serde_json::{Map, Value, json};

pub fn typed_output_schema<T: schemars::JsonSchema>() -> Value {
    schemars::schema_for!(T).to_value()
}

pub fn nonempty_json_schema(_: &mut schemars::SchemaGenerator) -> schemars::Schema {
    json!({"anyOf":[
        {"type":"string","minLength":1,"pattern":"\\S"},
        {"type":"array","minItems":1},
        {"type":"object","minProperties":1},
        {"type":["boolean","number"]}
    ]})
    .try_into()
    .expect("static nonempty JSON schema")
}

/// Projects machine-readable field constraints from the same contract sent in
/// the task payload. Unspecified business fields remain the domain validator's responsibility.
pub fn array_output_contract_schema(contract: &Value) -> Result<Value, String> {
    let root = contract["required_root_field"]
        .as_str()
        .filter(|root| !root.is_empty())
        .ok_or("数组输出合同缺少 required_root_field")?;
    let properties = contract["field_types"]
        .as_object()
        .ok_or("输出合同 field_types 必须为对象")?;
    let required = contract["item_required_fields"]
        .as_array()
        .ok_or("输出合同 item_required_fields 必须为数组")?;
    let mut seen = std::collections::HashSet::new();
    for field in required {
        let field = field
            .as_str()
            .filter(|field| !field.trim().is_empty())
            .ok_or("item_required_fields 必须只包含非空字段名称")?;
        if !seen.insert(field) {
            return Err(format!("item_required_fields 包含重复字段：{field}"));
        }
        if !properties.contains_key(field) {
            return Err(format!("{field} 缺少 field_types 合同"));
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
            "required_root_field":"characters", "item_required_fields":["name","evidence_anchor_id"],
            "field_types":{"name":{"type":"string"},"evidence_anchor_id":super::super::positive_i64_schema()}
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

    #[test]
    fn array_schema_rejects_incomplete_or_ambiguous_field_declarations() {
        let contract = json!({
            "required_root_field":"items", "item_required_fields":["kind"],
            "field_types":{"kind":{"type":"string","enum":["primary","minor"]}}
        });
        let schema = array_output_contract_schema(&contract).unwrap();
        assert_eq!(
            schema["properties"]["items"]["items"]["properties"],
            contract["field_types"]
        );
        for fields in [
            json!(null),
            json!(["missing"]),
            json!(["kind", "kind"]),
            json!([""]),
            json!([1]),
        ] {
            let mut invalid = contract.clone();
            invalid["item_required_fields"] = fields;
            assert!(array_output_contract_schema(&invalid).is_err());
        }
        let mut missing_types = contract;
        missing_types.as_object_mut().unwrap().remove("field_types");
        assert!(array_output_contract_schema(&missing_types).is_err());
    }

    #[test]
    fn nullable_optional_and_required_empty_objects_are_distinct() {
        let schema = array_output_contract_schema(&json!({
            "required_root_field":"items", "item_required_fields":["state"],
            "field_types":{"state":{"type":"object"},"note":{"type":["string","null"]}}
        }))
        .unwrap();
        assert_eq!(
            schema["properties"]["items"]["items"]["required"],
            json!(["state"])
        );
        assert_eq!(
            schema["properties"]["items"]["items"]["properties"]["note"]["type"],
            json!(["string", "null"])
        );
    }
}
