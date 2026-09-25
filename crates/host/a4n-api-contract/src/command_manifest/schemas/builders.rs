use schemars::{JsonSchema, Schema, transform::transform_subschemas};
use serde_json::{Value, json};
pub(crate) fn typed_input_schema<T: JsonSchema>() -> Value {
    json!({
        "type": "object",
        "properties": { "input": property_schema::<T>("input") },
        "required": ["input"],
        "additionalProperties": true,
        "x-a4n-schema-quality": "typed-input",
    })
}

pub(crate) fn strict_typed_input_schema<T: JsonSchema>() -> Value {
    let mut schema = typed_input_schema::<T>();
    schema["additionalProperties"] = Value::Bool(false);
    schema
}

pub(crate) fn config_typed_input_schema<T: JsonSchema>() -> Value {
    json!({
        "type": "object",
        "properties": {
            "configId": { "type": "integer", "minimum": 1 },
            "input": property_schema::<T>("input"),
        },
        "required": ["configId", "input"],
        "additionalProperties": false,
        "x-a4n-schema-quality": "typed-input",
    })
}

pub(crate) fn strict_runtime_typed_input_schema<T: JsonSchema>() -> Value {
    json!({
        "type": "object",
        "properties": {
            "input": property_schema::<T>("input"),
            "runtime_config_snapshot": { "type": ["object", "null"] },
        },
        "required": ["input"],
        "additionalProperties": false,
        "x-a4n-schema-quality": "typed-input",
    })
}

pub(crate) fn optional_task_runtime_typed_input_schema<T: JsonSchema>() -> Value {
    let mut schema = strict_runtime_typed_input_schema::<T>();
    schema["properties"]
        .as_object_mut()
        .expect("runtime input properties must be an object")
        .insert(
            "taskId".into(),
            json!({
                "type": ["string", "null"],
                "minLength": 1,
                "maxLength": 160,
                "pattern": "^[A-Za-z0-9_.:-]+$",
            }),
        );
    schema
}

pub(crate) fn required_task_typed_input_schema<T: JsonSchema>() -> Value {
    json!({
        "type": "object",
        "properties": {
            "input": property_schema::<T>("input"),
            "taskId": {
                "type": "string",
                "minLength": 1,
                "maxLength": 160,
                "pattern": "^[A-Za-z0-9_.:-]+$",
            },
        },
        "required": ["input", "taskId"],
        "additionalProperties": false,
        "x-a4n-schema-quality": "typed-input",
    })
}

pub(crate) fn required_task_runtime_typed_input_schema<T: JsonSchema>() -> Value {
    let mut schema = required_task_typed_input_schema::<T>();
    schema["properties"]
        .as_object_mut()
        .expect("task input properties must be an object")
        .insert(
            "runtime_config_snapshot".into(),
            json!({ "type": ["object", "null"] }),
        );
    schema
}

pub(crate) fn project_typed_input_schema<T: JsonSchema>() -> Value {
    json!({
        "type": "object",
        "properties": {
            "projectId": { "type": "integer", "minimum": 1 },
            "input": property_schema::<T>("input"),
        },
        "required": ["projectId", "input"],
        "additionalProperties": false,
        "x-a4n-schema-quality": "typed-input",
    })
}

pub(crate) fn project_runtime_typed_input_schema<T: JsonSchema>() -> Value {
    json!({
        "type": "object",
        "properties": {
            "projectId": { "type": "integer", "minimum": 1 },
            "input": property_schema::<T>("input"),
            "runtime_config_snapshot": { "type": ["object", "null"] },
        },
        "required": ["projectId", "input"],
        "additionalProperties": false,
        "x-a4n-schema-quality": "typed-input",
    })
}

pub(crate) fn project_optional_task_runtime_typed_input_schema<T: JsonSchema>() -> Value {
    let mut schema = project_runtime_typed_input_schema::<T>();
    schema["properties"]
        .as_object_mut()
        .expect("project runtime input properties must be an object")
        .insert(
            "taskId".into(),
            json!({
                "type": ["string", "null"],
                "minLength": 1,
                "maxLength": 160,
                "pattern": "^[A-Za-z0-9_.:-]+$",
            }),
        );
    schema
}

pub(crate) fn empty_args_schema() -> Value {
    json!({
        "type": "object",
        "additionalProperties": false,
        "x-a4n-schema-quality": "typed",
    })
}

pub(crate) fn typed_schema<T: JsonSchema>() -> Value {
    let mut schema = raw_schema::<T>();
    schema
        .as_object_mut()
        .expect("schemars root schema must be an object")
        .insert("x-a4n-schema-quality".into(), Value::String("typed".into()));
    schema
}

pub(crate) fn raw_schema<T: JsonSchema>() -> Value {
    serde_json::to_value(schemars::schema_for!(T)).expect("JSON schema must serialize")
}

pub(crate) fn property_schema<T: JsonSchema>(property: &str) -> Value {
    let mut schema = schemars::schema_for!(T);
    let property = property.replace('~', "~0").replace('/', "~1");
    rebase_schema_refs(&mut schema, &format!("/properties/{property}"));
    serde_json::to_value(schema).expect("JSON schema must serialize")
}

fn rebase_schema_refs(schema: &mut Schema, prefix: &str) {
    if let Some(object) = schema.as_object_mut() {
        if object.contains_key("$id") {
            return;
        }
        if let Some(Value::String(reference)) = object.get_mut("$ref")
            && (reference == "#" || reference.starts_with("#/"))
        {
            *reference = format!("#{prefix}{}", &reference[1..]);
        }
    }
    transform_subschemas(
        &mut |child: &mut Schema| rebase_schema_refs(child, prefix),
        schema,
    );
}

pub(crate) fn with_context_fields(mut schema: Value, fields: &[&str]) -> Value {
    schema
        .as_object_mut()
        .expect("command args schema must be an object")
        .insert(
            "x-a4n-context-fields".into(),
            serde_json::to_value(fields).expect("context fields must serialize"),
        );
    schema
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_references(schema: &mut Schema, root: &Value, command: &str) {
        let own_resource = schema
            .as_object()
            .is_some_and(|object| object.contains_key("$id"))
            .then(|| serde_json::to_value(&*schema).unwrap());
        let root = own_resource.as_ref().unwrap_or(root);
        if let Some(reference) = schema
            .as_object()
            .and_then(|object| object.get("$ref"))
            .and_then(Value::as_str)
            && let Some(pointer) = reference.strip_prefix('#')
            && (pointer.is_empty() || pointer.starts_with('/'))
        {
            assert!(
                root.pointer(pointer).is_some(),
                "{command}: unresolved {reference}"
            );
        }
        transform_subschemas(
            &mut |child: &mut Schema| assert_references(child, root, command),
            schema,
        );
    }

    #[test]
    fn command_schemas_resolve_all_embedded_local_references() {
        for command in crate::command_manifest::API_COMMANDS {
            let descriptor = crate::command_manifest::command_descriptor(command).unwrap();
            for value in [descriptor.args_schema, descriptor.result_schema] {
                let mut schema = Schema::try_from(value.clone()).unwrap();
                assert_references(&mut schema, &value, command);
            }
        }
    }

    #[test]
    fn embedding_preserves_example_data_and_independent_schema_resources() {
        let mut schema = schemars::json_schema!({
            "$defs": { "Value": { "type": "string" } },
            "properties": {
                "value": { "$ref": "#/$defs/Value" },
                "resource": { "$id": "urn:a4n:test:resource", "$defs": { "Own": { "type": "integer" } }, "$ref": "#/$defs/Own" }
            },
            "default": { "$ref": "#/$defs/literal-data" }
        });
        rebase_schema_refs(&mut schema, "/properties/input");
        let value = serde_json::to_value(schema).unwrap();
        assert_eq!(
            value["properties"]["value"]["$ref"],
            "#/properties/input/$defs/Value"
        );
        assert_eq!(value["properties"]["resource"]["$ref"], "#/$defs/Own");
        assert_eq!(value["default"]["$ref"], "#/$defs/literal-data");
    }
}
