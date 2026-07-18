use std::collections::BTreeSet;

use a4n_image::ImageGenerationRequest;
use serde_json::Value;

use super::contracts::{BatchImageJob, OwnedVideoFrameImageGenerationContext};
use super::dimensions::batch_image_dimensions;
use super::references::{
    attach_video_frame_reference_image_ids, batch_reference_image_ids,
    ensure_batch_reference_image_backend,
};
use super::validation::{ensure_batch_image_item_shape, optional_i64_field, optional_string_field};
use crate::AppState;
use crate::actions::image_generation::video_frame_context::{
    video_frame_generation_label, video_frame_image_generation_context,
};
use crate::actions::image_generation::{
    merge_image_metadata, resolve_image_config,
    source_binding::{
        ImageChapterBindingAccess, collect_generated_reference_images_by_ids,
        resolve_image_chapter_binding,
    },
};

pub(in crate::actions::image_generation) async fn build_batch_image_jobs(
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
                    .orchestrator
                    .ensure_frame_prompt_image_generation_allowed(
                        context.prompt_id,
                        &context.source,
                    )
                    .await
                    .map_err(crate::actions::app_error_message)?;
                reference_image_ids = state
                    .orchestrator
                    .effective_frame_prompt_image_reference_ids(
                        context.prompt_id,
                        &context.source,
                        &reference_image_ids,
                    )
                    .await
                    .map_err(crate::actions::app_error_message)?;
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
                    .orchestrator
                    .effective_frame_prompt_image_reference_ids(
                        context.prompt_id,
                        &context.source,
                        &reference_image_ids,
                    )
                    .await
                    .map_err(crate::actions::app_error_message)?;
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
