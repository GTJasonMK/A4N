use std::collections::BTreeSet;

use a4n_core::models::ai::image_gen_config;
use a4n_engine::image::ImageGenerationRequest;
use a4n_services::image::image_backend_supports_reference_images;
use serde_json::{Value, json};

use super::video_frame_context::{
    video_frame_generation_label, video_frame_image_generation_context,
};
use super::{
    merge_image_metadata, resolve_image_config,
    source_binding::{
        ImageChapterBindingAccess, collect_generated_reference_images_by_ids,
        resolve_image_chapter_binding,
    },
};
use crate::AppState;

const BATCH_IMAGE_ITEM_FIELDS: &[&str] = &[
    "prompt",
    "image_type",
    "chapter_id",
    "panel_id",
    "metadata_json",
    "config_id",
    "aspect_ratio",
    "reference_image_ids",
];

pub(super) struct BatchImageJob {
    pub(super) index: usize,
    pub(super) request: ImageGenerationRequest,
    pub(super) image_type: String,
    pub(super) preview_notes: Vec<String>,
    pub(super) chapter_id: Option<i64>,
    pub(super) panel_id: Option<String>,
    pub(super) metadata_json: Option<Value>,
    pub(super) config: image_gen_config::Model,
}

struct OwnedVideoFrameImageGenerationContext {
    source: String,
    prompt_id: i64,
    request_preview_note: Option<String>,
}

pub(super) async fn build_batch_image_jobs(
    state: &AppState,
    project_id: i64,
    arr: &[Value],
    enforce_generation_locks: bool,
) -> Result<Vec<BatchImageJob>, String> {
    let mut jobs = Vec::with_capacity(arr.len());
    let mut video_frame_requests = BTreeSet::<(String, i64)>::new();
    for (index, item) in arr.iter().enumerate() {
        ensure_batch_image_item_shape(index, item)?;
        let prompt = item
            .get("prompt")
            .and_then(Value::as_str)
            .ok_or_else(|| "每项必须含 prompt 字段".to_string())?
            .trim();
        if prompt.is_empty() {
            return Err(format!("第 {} 项 prompt 不能为空", index + 1));
        }
        let image_type = item
            .get("image_type")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|v| !v.is_empty())
            .ok_or_else(|| "每项必须含非空 image_type 字段".to_string())?
            .to_string();
        let chapter_id = optional_i64_field(item, "chapter_id")?;
        let panel_id = optional_string_field(item, "panel_id")?
            .map(str::trim)
            .filter(|v| !v.is_empty())
            .map(ToString::to_string);
        let metadata_json = item
            .get("metadata_json")
            .filter(|value| !value.is_null())
            .cloned();
        let config_id = optional_i64_field(item, "config_id")?;
        let mut reference_image_ids = batch_reference_image_ids(item)?;
        let binding_access = if enforce_generation_locks {
            ImageChapterBindingAccess::Writable
        } else {
            ImageChapterBindingAccess::ReadOnly
        };
        let (chapter_id, source_metadata) =
            resolve_image_chapter_binding(state, project_id, chapter_id, binding_access).await?;
        let mut metadata_json = merge_image_metadata(metadata_json, source_metadata);
        let mut preview_notes = Vec::new();
        let video_frame_context = video_frame_image_generation_context(metadata_json.as_ref())?
            .map(|context| OwnedVideoFrameImageGenerationContext {
                source: context.source.to_string(),
                prompt_id: context.prompt_id,
                request_preview_note: context.request_preview_note.map(ToString::to_string),
            });
        match video_frame_context.as_ref() {
            Some(context) if enforce_generation_locks => {
                let key = (context.source.to_string(), context.prompt_id);
                if !video_frame_requests.insert(key) {
                    return Err(format!(
                        "批量成图包含重复的{}请求：视频帧提示词 #{}",
                        video_frame_generation_label(&context.source),
                        context.prompt_id
                    ));
                }
                state
                    .video
                    .ensure_frame_prompt_image_generation_allowed(
                        context.prompt_id,
                        &context.source,
                    )
                    .await
                    .map_err(|e| e.to_string())?;
                reference_image_ids = state
                    .video
                    .effective_frame_prompt_image_reference_ids(
                        context.prompt_id,
                        &context.source,
                        &reference_image_ids,
                    )
                    .await
                    .map_err(|e| e.to_string())?;
            }
            Some(context) => {
                let key = (context.source.to_string(), context.prompt_id);
                if !video_frame_requests.insert(key) {
                    return Err(format!(
                        "批量成图包含重复的{}请求：视频帧提示词 #{}",
                        video_frame_generation_label(&context.source),
                        context.prompt_id
                    ));
                }
                preview_notes.push(format!(
                    "{}绑定视频帧提示词 #{}；预览按当前有效参考图构造图片 provider 请求。",
                    video_frame_generation_label(&context.source),
                    context.prompt_id
                ));
                if let Some(note) = context.request_preview_note.as_ref() {
                    preview_notes.push(note.clone());
                }
                let effective_ids = state
                    .video
                    .effective_frame_prompt_image_reference_ids(
                        context.prompt_id,
                        &context.source,
                        &reference_image_ids,
                    )
                    .await
                    .map_err(|e| e.to_string())?;
                if effective_ids != reference_image_ids {
                    preview_notes
                        .push(format!("按边界帧连续性规则使用参考图：{:?}", effective_ids));
                }
                reference_image_ids = effective_ids;
            }
            None => {}
        }
        if video_frame_context.is_some() {
            metadata_json =
                attach_video_frame_reference_image_ids(metadata_json, &reference_image_ids)?;
        }

        let config = resolve_image_config(state, config_id).await?;
        let reference_images = collect_generated_reference_images_by_ids(
            state,
            project_id,
            chapter_id,
            video_frame_context.is_some(),
            &reference_image_ids,
        )
        .await?;
        if !reference_images.is_empty() {
            ensure_batch_reference_image_backend(&config.backend)?;
            preview_notes.push(format!("参考图数量：{}", reference_images.len()));
        }
        let (width, height) = batch_image_dimensions(&config, item, metadata_json.as_ref())?;
        jobs.push(BatchImageJob {
            index,
            request: ImageGenerationRequest {
                prompt: prompt.to_string(),
                width,
                height,
                model: config.model.clone(),
                reference_images,
            },
            image_type,
            preview_notes,
            chapter_id,
            panel_id,
            metadata_json,
            config,
        });
    }
    Ok(jobs)
}

fn batch_image_dimensions(
    config: &image_gen_config::Model,
    item: &Value,
    metadata_json: Option<&Value>,
) -> Result<(i32, i32), String> {
    let Some(aspect_ratio) = batch_image_aspect_ratio(item, metadata_json) else {
        return Ok((config.default_width, config.default_height));
    };
    image_dimensions_for_aspect_ratio(config.default_width, config.default_height, &aspect_ratio)
}

fn batch_image_aspect_ratio(item: &Value, metadata_json: Option<&Value>) -> Option<String> {
    let top_level = optional_string_field(item, "aspect_ratio").ok().flatten();
    [
        top_level,
        metadata_json.and_then(|metadata| metadata.get("aspect_ratio").and_then(Value::as_str)),
        metadata_json.and_then(|metadata| {
            metadata
                .get("prompt_snapshot")
                .and_then(|snapshot| snapshot.get("aspect_ratio"))
                .and_then(Value::as_str)
        }),
        metadata_json.and_then(|metadata| {
            metadata
                .get("generation_context")
                .and_then(|context| context.get("aspect_ratio"))
                .and_then(Value::as_str)
        }),
    ]
    .into_iter()
    .flatten()
    .map(|value| value.trim())
    .find(|value| !value.is_empty())
    .map(ToString::to_string)
}

fn ensure_batch_image_item_shape(index: usize, item: &Value) -> Result<(), String> {
    let object = item
        .as_object()
        .ok_or_else(|| format!("第 {} 项必须是 JSON 对象", index + 1))?;
    for key in object.keys() {
        if !BATCH_IMAGE_ITEM_FIELDS.contains(&key.as_str()) {
            return Err(format!("第 {} 项包含未知字段：{key}", index + 1));
        }
    }
    optional_i64_field(item, "chapter_id")?;
    optional_i64_field(item, "config_id")?;
    optional_string_field(item, "panel_id")?;
    optional_string_field(item, "aspect_ratio")?;
    Ok(())
}

fn optional_i64_field(item: &Value, field: &str) -> Result<Option<i64>, String> {
    let Some(value) = item.get(field).filter(|value| !value.is_null()) else {
        return Ok(None);
    };
    value
        .as_i64()
        .map(Some)
        .ok_or_else(|| format!("{field} 必须是整数"))
}

fn optional_string_field<'a>(item: &'a Value, field: &str) -> Result<Option<&'a str>, String> {
    let Some(value) = item.get(field).filter(|value| !value.is_null()) else {
        return Ok(None);
    };
    value
        .as_str()
        .map(Some)
        .ok_or_else(|| format!("{field} 必须是字符串"))
}

fn image_dimensions_for_aspect_ratio(
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
    Ok((width.max(1) as i32, height.max(1) as i32))
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

fn batch_reference_image_ids(item: &Value) -> Result<Vec<i64>, String> {
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

fn attach_video_frame_reference_image_ids(
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

fn ensure_batch_reference_image_backend(backend: &str) -> Result<(), String> {
    if image_backend_supports_reference_images(backend) {
        return Ok(());
    }
    Err(format!(
        "当前图片后端 {backend} 不支持真实参考图输入，请切换到 openai-compatible-image-edit、openai-chat-image 或 cliproxy-image，或移除 reference_image_ids"
    ))
}

pub(super) fn batch_image_step_note(image_type: &str, preview_notes: &[String]) -> String {
    let mut notes = Vec::with_capacity(preview_notes.len() + 1);
    notes.push(format!("image_type: {image_type}"));
    notes.extend(preview_notes.iter().cloned());
    notes.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn image_dimensions_follow_explicit_aspect_ratio() {
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
    }

    #[test]
    fn image_dimensions_reject_invalid_aspect_ratio() {
        let err = image_dimensions_for_aspect_ratio(1024, 1024, "wide")
            .expect_err("invalid aspect ratio should fail");
        assert!(err.contains("aspect_ratio 无效"));

        let err = image_dimensions_for_aspect_ratio(1024, 1024, "16:0")
            .expect_err("zero aspect ratio should fail");
        assert!(err.contains("必须大于 0"));
    }

    #[test]
    fn batch_image_aspect_ratio_reads_prompt_snapshot_and_top_level_override() {
        let metadata = json!({
            "prompt_snapshot": {
                "aspect_ratio": "16:9"
            }
        });
        assert_eq!(
            batch_image_aspect_ratio(&json!({}), Some(&metadata)),
            Some("16:9".to_string())
        );
        assert_eq!(
            batch_image_aspect_ratio(&json!({ "aspect_ratio": "9:16" }), Some(&metadata)),
            Some("9:16".to_string())
        );
    }

    #[test]
    fn batch_image_item_shape_rejects_unknown_fields() {
        let err = ensure_batch_image_item_shape(
            0,
            &json!({
                "prompt": "draw",
                "image_type": "video_key_frame",
                "legacy_config": 1
            }),
        )
        .expect_err("unknown batch item field must fail");

        assert_eq!(err, "第 1 项包含未知字段：legacy_config");
    }

    #[test]
    fn batch_image_item_shape_rejects_wrong_optional_field_types() {
        let err = ensure_batch_image_item_shape(
            1,
            &json!({
                "prompt": "draw",
                "image_type": "video_key_frame",
                "chapter_id": "12"
            }),
        )
        .expect_err("chapter_id string must not become project-level image");
        assert_eq!(err, "chapter_id 必须是整数");

        let err = ensure_batch_image_item_shape(
            1,
            &json!({
                "prompt": "draw",
                "image_type": "video_key_frame",
                "config_id": "2"
            }),
        )
        .expect_err("config_id string must not fall back to default config");
        assert_eq!(err, "config_id 必须是整数");

        let err = ensure_batch_image_item_shape(
            1,
            &json!({
                "prompt": "draw",
                "image_type": "video_key_frame",
                "aspect_ratio": 169
            }),
        )
        .expect_err("aspect_ratio number must not fall back to default dimensions");
        assert_eq!(err, "aspect_ratio 必须是字符串");
    }

    #[test]
    fn batch_image_item_shape_accepts_documented_and_video_frame_fields() {
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
        .expect("known batch item fields should pass");
    }

    #[test]
    fn video_frame_reference_ids_are_attached_to_generation_context() {
        let metadata = attach_video_frame_reference_image_ids(
            Some(json!({
                "generation_context": {
                    "source": "video_boundary_frame_prompt",
                    "prompt_id": 9
                }
            })),
            &[3, 5],
        )
        .expect("metadata should accept reference ids")
        .expect("metadata should be present");

        assert_eq!(
            metadata["generation_context"]["reference_image_ids"],
            json!([3, 5])
        );
    }

    #[test]
    fn video_frame_reference_ids_reject_non_object_generation_context() {
        let err = attach_video_frame_reference_image_ids(
            Some(json!({
                "generation_context": "invalid"
            })),
            &[3],
        )
        .expect_err("generation_context must stay structured");

        assert!(err.contains("metadata_json.generation_context 必须是对象"));
    }

    #[test]
    fn batch_reference_image_backend_accepts_only_real_image_input_backends() {
        for backend in [
            "openai-compatible-image-edit",
            "openai-chat-image",
            "cliproxy-image",
        ] {
            ensure_batch_reference_image_backend(backend)
                .expect("image input backend should accept reference_image_ids");
        }

        for backend in ["openai-compatible-image", "stable-diffusion", "comfyui"] {
            let err = ensure_batch_reference_image_backend(backend)
                .expect_err("text-only backend must reject reference_image_ids");
            assert!(err.contains("不支持真实参考图输入"));
            assert!(err.contains("移除 reference_image_ids"));
        }
    }
}
