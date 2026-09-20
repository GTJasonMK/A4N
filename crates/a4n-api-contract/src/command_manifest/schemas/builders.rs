use schemars::JsonSchema;
use serde_json::{Value, json};
pub(crate) fn typed_input_schema<T: JsonSchema>() -> Value {
    json!({
        "type": "object",
        "properties": { "input": raw_schema::<T>() },
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
            "input": raw_schema::<T>(),
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
            "input": raw_schema::<T>(),
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
            "input": raw_schema::<T>(),
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
            "input": raw_schema::<T>(),
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
            "input": raw_schema::<T>(),
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
