use serde_json::Value;

pub(in crate::video_prompt::dependencies) fn image_generation_prompt_id(
    metadata: Option<&Value>,
) -> Option<i64> {
    let value = metadata
        .and_then(|metadata| metadata.get("generation_context"))
        .and_then(|context| context.get("prompt_id"))?;
    value.as_i64()
}

pub(in crate::video_prompt::dependencies) fn image_generation_source(
    metadata: Option<&Value>,
) -> Option<&str> {
    metadata
        .and_then(|metadata| metadata.get("generation_context"))
        .and_then(|context| context.get("source"))
        .and_then(Value::as_str)
}

pub(in crate::video_prompt::dependencies) fn image_generation_shot_number(
    metadata: Option<&Value>,
) -> Option<i32> {
    metadata
        .and_then(|metadata| metadata.get("generation_context"))
        .and_then(|context| context.get("shot_number"))
        .and_then(Value::as_i64)
        .and_then(|value| i32::try_from(value).ok())
        .filter(|value| *value > 0)
}

pub(in crate::video_prompt::dependencies) fn image_generation_source_hash(
    metadata: Option<&Value>,
) -> Option<&str> {
    metadata
        .and_then(|metadata| metadata.get("generation_context"))
        .and_then(|context| context.get("source_content_sha256"))
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
}
