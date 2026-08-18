//! 批量图片生成项 JSON 形状纯规则。

use a4n_domain::VideoAspectRatio;
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

/// 校验 batch 图片项对象形状与字段白名单。
pub fn ensure_batch_image_item_shape(index: usize, item: &Value) -> Result<(), String> {
    let object = item
        .as_object()
        .ok_or_else(|| format!("第 {} 项必须是 JSON 对象", index + 1))?;
    for key in object.keys() {
        if !BATCH_IMAGE_ITEM_FIELDS.contains(&key.as_str()) {
            return Err(format!("第 {} 项包含未知字段：{key}", index + 1));
        }
    }
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

/// 读取 batch 项唯一的 aspect_ratio 合同。
pub fn batch_image_aspect_ratio(item: &Value) -> Result<VideoAspectRatio, String> {
    let aspect_ratio = optional_json_string_field(item, "aspect_ratio")?
        .filter(|value| !value.is_empty())
        .ok_or_else(|| "aspect_ratio 必须是非空字符串".to_string())?;
    aspect_ratio.parse::<VideoAspectRatio>().map_err(|_| {
        format!(
            "图片 aspect_ratio 无效：{aspect_ratio}，必须是 {}",
            VideoAspectRatio::supported_values()
        )
    })
}

/// 按 aspect_ratio 与默认长边计算宽高。
pub fn image_dimensions_for_aspect_ratio(
    default_width: i32,
    default_height: i32,
    aspect_ratio: VideoAspectRatio,
) -> Result<(i32, i32), String> {
    let (ratio_width, ratio_height) = aspect_ratio.components();
    let ratio_width = i64::from(ratio_width);
    let ratio_height = i64::from(ratio_height);
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
    use a4n_domain::image::{
        IMAGE_GENERATION_SOURCE_VIDEO_BOUNDARY_FRAME_PROMPT,
        IMAGE_GENERATION_SOURCE_VIDEO_FIRST_FRAME_PROMPT, IMAGE_TYPE_VIDEO_KEY_FRAME,
    };
    use serde_json::json;

    #[test]
    fn shape_rejects_unknown_fields_and_field_readers_reject_wrong_types() {
        let err = ensure_batch_image_item_shape(
            0,
            &json!({
                "prompt": "draw",
                "image_type": IMAGE_TYPE_VIDEO_KEY_FRAME,
                "legacy_config": 1
            }),
        )
        .unwrap_err();
        assert_eq!(err, "第 1 项包含未知字段：legacy_config");

        let err = optional_json_i64_field(&json!({"chapter_id": "12"}), "chapter_id")
            .unwrap_err();
        assert_eq!(err, "chapter_id 必须是整数");

        let err = batch_image_aspect_ratio(&json!({"aspect_ratio": 169})).unwrap_err();
        assert_eq!(err, "aspect_ratio 必须是字符串");
    }

    #[test]
    fn shape_accepts_documented_fields() {
        ensure_batch_image_item_shape(
            0,
            &json!({
                "prompt": "draw",
                "image_type": IMAGE_TYPE_VIDEO_KEY_FRAME,
                "chapter_id": 12,
                "panel_id": "shot-1",
                "metadata_json": { "generation_context": { "source": IMAGE_GENERATION_SOURCE_VIDEO_FIRST_FRAME_PROMPT, "prompt_id": 9 } },
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
            Some(json!({"generation_context": {"source": IMAGE_GENERATION_SOURCE_VIDEO_BOUNDARY_FRAME_PROMPT}})),
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
            json!(IMAGE_GENERATION_SOURCE_VIDEO_BOUNDARY_FRAME_PROMPT)
        );

        let err =
            attach_video_frame_reference_image_ids(Some(json!({"generation_context": []})), &[1])
                .unwrap_err();
        assert!(err.contains("generation_context 必须是对象"));
    }

    #[test]
    fn aspect_ratio_dimensions_and_backend() {
        assert_eq!(
            image_dimensions_for_aspect_ratio(
                1024,
                1024,
                VideoAspectRatio::Ratio16x9,
            )
            .unwrap(),
            (1024, 576)
        );
        assert_eq!(
            image_dimensions_for_aspect_ratio(
                1024,
                1024,
                VideoAspectRatio::Ratio9x16,
            )
            .unwrap(),
            (576, 1024)
        );
        assert_eq!(
            image_dimensions_for_aspect_ratio(768, 1024, VideoAspectRatio::Ratio1x1).unwrap(),
            (1024, 1024)
        );

        assert_eq!(
            batch_image_aspect_ratio(&json!({"aspect_ratio": "16:9"})).unwrap(),
            VideoAspectRatio::Ratio16x9
        );
        assert!(batch_image_aspect_ratio(&json!({})).is_err());
        for invalid in [" 16:9 ", "16/9", "16：9", "2:1"] {
            assert!(batch_image_aspect_ratio(&json!({"aspect_ratio": invalid})).is_err());
        }

        ensure_batch_reference_image_backend("openai-chat-image").unwrap();
        assert!(ensure_batch_reference_image_backend("stable-diffusion").is_err());
    }
}
