//! 批量图片生成项 JSON 形状纯规则。

use serde_json::{Value, json};

/// 批量图片项允许的字段白名单。
pub const BATCH_IMAGE_ITEM_FIELDS: &[&str] = &[
    "prompt",
    "image_type",
    "chapter_id",
    "panel_id",
    "metadata_json",
    "config_id",
    "aspect_ratio",
    "reference_image_ids",
];

/// 校验 batch 图片项字段白名单与可选字段类型。
pub fn ensure_batch_image_item_shape(index: usize, item: &Value) -> Result<(), String> {
    let object = item
        .as_object()
        .ok_or_else(|| format!("第 {} 项必须是 JSON 对象", index + 1))?;
    for key in object.keys() {
        if !BATCH_IMAGE_ITEM_FIELDS.contains(&key.as_str()) {
            return Err(format!("第 {} 项包含未知字段：{key}", index + 1));
        }
    }
    optional_json_i64_field(item, "chapter_id")?;
    optional_json_i64_field(item, "config_id")?;
    optional_json_string_field(item, "panel_id")?;
    optional_json_string_field(item, "aspect_ratio")?;
    Ok(())
}

/// 可选整数 JSON 字段（null/缺省 → None）。
pub fn optional_json_i64_field(item: &Value, field: &str) -> Result<Option<i64>, String> {
    let Some(value) = item.get(field).filter(|value| !value.is_null()) else {
        return Ok(None);
    };
    value
        .as_i64()
        .map(Some)
        .ok_or_else(|| format!("{field} 必须是整数"))
}

/// 可选字符串 JSON 字段（null/缺省 → None）。
pub fn optional_json_string_field<'a>(
    item: &'a Value,
    field: &str,
) -> Result<Option<&'a str>, String> {
    let Some(value) = item.get(field).filter(|value| !value.is_null()) else {
        return Ok(None);
    };
    value
        .as_str()
        .map(Some)
        .ok_or_else(|| format!("{field} 必须是字符串"))
}

/// 从 batch 项与 metadata 解析 aspect_ratio（顶层优先，再 metadata / snapshot / context）。
pub fn batch_image_aspect_ratio(
    item: &Value,
    metadata_json: Option<&Value>,
) -> Result<Option<String>, String> {
    let top_level = optional_json_string_field(item, "aspect_ratio")?;
    let metadata = metadata_json
        .map(|metadata| {
            optional_aspect_ratio_field(metadata, "aspect_ratio", "metadata_json.aspect_ratio")
        })
        .transpose()?
        .flatten();
    let prompt_snapshot = metadata_json
        .and_then(|metadata| metadata.get("prompt_snapshot"))
        .map(|snapshot| {
            optional_aspect_ratio_field(
                snapshot,
                "aspect_ratio",
                "metadata_json.prompt_snapshot.aspect_ratio",
            )
        })
        .transpose()?
        .flatten();
    let generation_context = metadata_json
        .and_then(|metadata| metadata.get("generation_context"))
        .map(|context| {
            optional_aspect_ratio_field(
                context,
                "aspect_ratio",
                "metadata_json.generation_context.aspect_ratio",
            )
        })
        .transpose()?
        .flatten();
    Ok([top_level, metadata, prompt_snapshot, generation_context]
        .into_iter()
        .flatten()
        .map(|value| value.trim())
        .find(|value| !value.is_empty())
        .map(ToString::to_string))
}

fn optional_aspect_ratio_field<'a>(
    container: &'a Value,
    field: &str,
    label: &str,
) -> Result<Option<&'a str>, String> {
    let Some(value) = container.get(field).filter(|value| !value.is_null()) else {
        return Ok(None);
    };
    value
        .as_str()
        .map(Some)
        .ok_or_else(|| format!("{label} 必须是字符串"))
}

/// 按 aspect_ratio 与默认长边计算宽高。
pub fn image_dimensions_for_aspect_ratio(
    default_width: i32,
    default_height: i32,
    aspect_ratio: &str,
) -> Result<(i32, i32), String> {
    let (ratio_width, ratio_height) = parse_image_aspect_ratio(aspect_ratio)?;
    let long_edge = i64::from(default_width.max(default_height).max(1));
    let (width, height) = if ratio_width >= ratio_height {
        (
            long_edge,
            rounded_ratio_dimension(long_edge, ratio_height, ratio_width),
        )
    } else {
        (
            rounded_ratio_dimension(long_edge, ratio_width, ratio_height),
            long_edge,
        )
    };
    let width = i32::try_from(width.max(1))
        .map_err(|_| format!("图片宽度超过 i32 范围：{}", width.max(1)))?;
    let height = i32::try_from(height.max(1))
        .map_err(|_| format!("图片高度超过 i32 范围：{}", height.max(1)))?;
    Ok((width, height))
}

fn parse_image_aspect_ratio(aspect_ratio: &str) -> Result<(i64, i64), String> {
    let normalized = aspect_ratio.trim().replace('：', ":").replace('／', "/");
    let Some((left, right)) = normalized
        .split_once(':')
        .or_else(|| normalized.split_once('/'))
    else {
        return Err(format!(
            "图片 aspect_ratio 无效：{aspect_ratio}，格式应为 16:9、9:16 或 1:1"
        ));
    };
    let ratio_width = left
        .trim()
        .parse::<i64>()
        .map_err(|_| format!("图片 aspect_ratio 无效：{aspect_ratio}，宽高比必须是正整数"))?;
    let ratio_height = right
        .trim()
        .parse::<i64>()
        .map_err(|_| format!("图片 aspect_ratio 无效：{aspect_ratio}，宽高比必须是正整数"))?;
    if ratio_width <= 0 || ratio_height <= 0 {
        return Err(format!(
            "图片 aspect_ratio 无效：{aspect_ratio}，宽高比必须大于 0"
        ));
    }
    Ok((ratio_width, ratio_height))
}

fn rounded_ratio_dimension(long_edge: i64, numerator: i64, denominator: i64) -> i64 {
    ((long_edge * numerator) + (denominator / 2)) / denominator
}

/// 批量生成使用参考图时，后端必须支持真实参考图输入。
pub fn ensure_batch_reference_image_backend(backend: &str) -> Result<(), String> {
    if crate::config::image_backend_supports_reference_images(backend) {
        return Ok(());
    }
    Err(format!(
        "当前图片后端 {backend} 不支持真实参考图输入，请切换到 openai-compatible-image-edit、openai-chat-image 或 cliproxy-image，或移除 reference_image_ids"
    ))
}

/// 将参考图 ID 写入 video frame 生成 metadata 的 generation_context。
pub fn attach_video_frame_reference_image_ids(
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

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn shape_rejects_unknown_and_wrong_types() {
        let err = ensure_batch_image_item_shape(
            0,
            &json!({
                "prompt": "draw",
                "image_type": "video_key_frame",
                "legacy_config": 1
            }),
        )
        .unwrap_err();
        assert_eq!(err, "第 1 项包含未知字段：legacy_config");

        let err = ensure_batch_image_item_shape(
            1,
            &json!({
                "prompt": "draw",
                "image_type": "video_key_frame",
                "chapter_id": "12"
            }),
        )
        .unwrap_err();
        assert_eq!(err, "chapter_id 必须是整数");

        let err = ensure_batch_image_item_shape(
            1,
            &json!({
                "prompt": "draw",
                "image_type": "video_key_frame",
                "aspect_ratio": 169
            }),
        )
        .unwrap_err();
        assert_eq!(err, "aspect_ratio 必须是字符串");
    }

    #[test]
    fn shape_accepts_documented_fields() {
        ensure_batch_image_item_shape(
            0,
            &json!({
                "prompt": "draw",
                "image_type": "video_key_frame",
                "chapter_id": 12,
                "panel_id": "shot-1",
                "metadata_json": { "generation_context": { "source": "video_first_frame_prompt", "prompt_id": 9 } },
                "config_id": 3,
                "aspect_ratio": "16:9",
                "reference_image_ids": [1, 2]
            }),
        )
        .unwrap();
    }

    #[test]
    fn attach_reference_ids_to_generation_context() {
        let metadata = attach_video_frame_reference_image_ids(
            Some(json!({"generation_context": {"source": "video_boundary_frame_prompt"}})),
            &[11, 12],
        )
        .unwrap()
        .unwrap();
        assert_eq!(
            metadata["generation_context"]["reference_image_ids"],
            json!([11, 12])
        );
        assert_eq!(
            metadata["generation_context"]["source"],
            json!("video_boundary_frame_prompt")
        );

        let err =
            attach_video_frame_reference_image_ids(Some(json!({"generation_context": []})), &[1])
                .unwrap_err();
        assert!(err.contains("generation_context 必须是对象"));
    }

    #[test]
    fn aspect_ratio_dimensions_and_backend() {
        assert_eq!(
            image_dimensions_for_aspect_ratio(1024, 1024, "16:9").unwrap(),
            (1024, 576)
        );
        assert_eq!(
            image_dimensions_for_aspect_ratio(1024, 1024, "9:16").unwrap(),
            (576, 1024)
        );
        assert_eq!(
            image_dimensions_for_aspect_ratio(768, 1024, "1：1").unwrap(),
            (1024, 1024)
        );
        assert!(image_dimensions_for_aspect_ratio(1024, 1024, "bad").is_err());

        let ratio = batch_image_aspect_ratio(
            &json!({"aspect_ratio": "16:9"}),
            Some(&json!({
                "aspect_ratio": "1:1",
                "prompt_snapshot": {"aspect_ratio": "4:3"},
                "generation_context": {"aspect_ratio": "9:16"},
            })),
        )
        .unwrap();
        assert_eq!(ratio.as_deref(), Some("16:9"));

        ensure_batch_reference_image_backend("openai-chat-image").unwrap();
        assert!(ensure_batch_reference_image_backend("stable-diffusion").is_err());
    }
}
