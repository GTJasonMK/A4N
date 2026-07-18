use std::collections::BTreeSet;

use a4n_orchestrator::config::image_backend_supports_reference_images;
use serde_json::{Value, json};

pub(super) fn batch_reference_image_ids(item: &Value) -> Result<Vec<i64>, String> {
    let Some(value) = item
        .get("reference_image_ids")
        .filter(|value| !value.is_null())
    else {
        return Ok(Vec::new());
    };
    let arr = value
        .as_array()
        .ok_or_else(|| "reference_image_ids 必须是数组".to_string())?;
    let mut seen = BTreeSet::new();
    let mut ids = Vec::with_capacity(arr.len());
    for (index, value) in arr.iter().enumerate() {
        let id = value
            .as_i64()
            .filter(|id| *id > 0)
            .ok_or_else(|| format!("reference_image_ids[{index}] 必须是正整数"))?;
        if !seen.insert(id) {
            return Err(format!("reference_image_ids 包含重复 ID：{id}"));
        }
        ids.push(id);
    }
    Ok(ids)
}

pub(super) fn attach_video_frame_reference_image_ids(
    metadata_json: Option<Value>,
    reference_image_ids: &[i64],
) -> Result<Option<Value>, String> {
    if reference_image_ids.is_empty() {
        return Ok(metadata_json);
    }
    match metadata_json {
        Some(Value::Object(mut metadata)) => {
            match metadata
                .entry("generation_context".to_string())
                .or_insert_with(|| json!({}))
            {
                Value::Object(context) => {
                    context.insert(
                        "reference_image_ids".to_string(),
                        json!(reference_image_ids),
                    );
                }
                _ => {
                    return Err("图片生成 metadata_json.generation_context 必须是对象".to_string());
                }
            }
            Ok(Some(Value::Object(metadata)))
        }
        Some(_) => Err("图片生成 metadata_json 必须是对象".to_string()),
        None => Ok(Some(json!({
            "generation_context": {
                "reference_image_ids": reference_image_ids,
            },
        }))),
    }
}

pub(super) fn ensure_batch_reference_image_backend(backend: &str) -> Result<(), String> {
    if image_backend_supports_reference_images(backend) {
        return Ok(());
    }
    Err(format!(
        "当前图片后端 {backend} 不支持真实参考图输入，请切换到 openai-compatible-image-edit、openai-chat-image 或 cliproxy-image，或移除 reference_image_ids"
    ))
}
