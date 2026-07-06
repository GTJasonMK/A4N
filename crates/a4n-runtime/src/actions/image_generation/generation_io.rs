use super::{ImageGenerationProgress, normalize_image_metadata};
use crate::AppState;
use crate::file_utils::data_dir_relative_file_path;
use a4n_core::files::paths::ProjectPaths;
use a4n_core::models::ai::{generated_image, image_gen_config};
use a4n_engine::image::{ImageGenerationRequest, ImageGenerationResult};
use a4n_engine::queue::priority::{TaskPriority, TaskQueue, TaskType as QueueTaskType};
use a4n_services::generation::{
    GenerationRunCheckpointContext, RecordCompletedGenerationRunInput,
    RecordCompletedGenerationRunWithCheckpointInput, generation_checkpoint_fingerprint_sha256,
};
use a4n_services::image::{CreateImageRecord, configured_image_provider};
use a4n_services::queue::TaskWorkflowContext;
use a4n_services::source::source_content_sha256;
use serde_json::{Value, json};
use std::future::Future;
use std::path::{Path, PathBuf};
use tokio_util::sync::CancellationToken;

pub(super) struct GeneratedImageSaveInput<'a> {
    pub(super) project_id: i64,
    pub(super) chapter_id: Option<i64>,
    pub(super) image_type: String,
    pub(super) command: &'a str,
    pub(super) config: &'a image_gen_config::Model,
    pub(super) file_extra: Option<String>,
    pub(super) file_context: &'a str,
    pub(super) prompt: String,
    pub(super) request_json: Value,
    pub(super) width: i32,
    pub(super) height: i32,
    pub(super) panel_id: Option<String>,
    pub(super) metadata_json: Option<Value>,
    pub(super) created_at: chrono::NaiveDateTime,
    pub(super) workflow_run_id: Option<i64>,
}

pub(super) async fn run_image_generation_provider_request_with_workflow(
    state: &AppState,
    project_id: i64,
    config: &image_gen_config::Model,
    request: ImageGenerationRequest,
    workflow_run_id: Option<i64>,
    progress: &ImageGenerationProgress<'_>,
) -> Result<ImageGenerationResult, String> {
    let provider = configured_image_provider(config).map_err(|e| e.to_string())?;
    run_queued_request_with_cancel_and_workflow(
        state,
        QueueTaskType::ImageGeneration,
        TaskPriority::Medium,
        Some(project_id),
        workflow_run_id,
        progress.cancel_token(),
        || async move {
            provider
                .generate(request)
                .await
                .map_err(|e| image_provider_error_with_config(config, e.to_string()))
        },
    )
    .await
}

fn image_provider_error_with_config(config: &image_gen_config::Model, error: String) -> String {
    format!(
        "{error}；实际使用图片配置：id={}，name={}，backend={}，base_url={}，model={}，has_api_key={}",
        config.id,
        config.name,
        config.backend,
        config.base_url.as_deref().unwrap_or("-"),
        config.model,
        config
            .api_key_encrypted
            .as_ref()
            .is_some_and(|value| !value.trim().is_empty())
    )
}

pub(super) fn image_generation_request_trace_json(
    config: &image_gen_config::Model,
    request: &ImageGenerationRequest,
) -> Value {
    let reference_images = request
        .reference_images
        .iter()
        .enumerate()
        .map(|(index, reference)| {
            json!({
                "index": index,
                "label": reference.label,
                "data_uri_sha256": source_content_sha256(&reference.data_uri),
                "data_uri_length": reference.data_uri.len(),
                "media_type": reference
                    .data_uri
                    .strip_prefix("data:")
                    .and_then(|value| value.split_once(';').map(|(media_type, _)| media_type)),
            })
        })
        .collect::<Vec<_>>();

    json!({
        "provider": {
            "config_id": config.id,
            "config_name": config.name,
            "backend": config.backend,
            "base_url": config.base_url,
            "configured_model": config.model,
        },
        "request": {
            "prompt": request.prompt,
            "width": request.width,
            "height": request.height,
            "aspect_ratio": image_request_aspect_ratio(request.width, request.height),
            "model": request.model,
            "reference_image_count": request.reference_images.len(),
            "reference_images": reference_images,
        },
    })
}

pub(super) async fn save_generated_image_result(
    state: &AppState,
    result: ImageGenerationResult,
    input: GeneratedImageSaveInput<'_>,
) -> Result<generated_image::Model, String> {
    let command = input.command.to_string();
    let provider = input.config.backend.clone();
    let request_json = input.request_json;
    let output_format = result.format.clone();
    let metadata_json = attach_image_generation_trace(
        input.metadata_json,
        ImageGenerationTrace {
            command: input.command,
            config: input.config,
            image_type: &input.image_type,
            prompt: &input.prompt,
            provider_model: &result.model,
            chapter_id: input.chapter_id,
            panel_id: input.panel_id.as_deref(),
            width: input.width,
            height: input.height,
            generated_at: input.created_at,
        },
    )?;
    let path = save_image_bytes(
        state,
        input.project_id,
        input.chapter_id,
        &input.image_type,
        input.file_extra.as_deref(),
        &result.bytes,
        &result.format,
    )?;
    let file_path = data_dir_relative_file_path(&state.config.data_dir, &path, input.file_context)?;
    let file_size = result.bytes.len() as i64;
    let media_refs_json = metadata_json.as_ref().and_then(image_generation_media_refs);
    let image_type = input.image_type;
    let prompt = input.prompt;
    let model = result.model;
    let chapter_id = input.chapter_id;
    let panel_id = input.panel_id.clone();
    let call_key = format!(
        "{}:{}:chapter:{}:panel:{}",
        command,
        image_type,
        chapter_id
            .map(|value| value.to_string())
            .unwrap_or_else(|| "none".to_string()),
        panel_id.as_deref().unwrap_or("none")
    );
    let input_fingerprint_json = json!({
        "command": command.clone(),
        "project_id": input.project_id,
        "chapter_id": chapter_id,
        "panel_id": panel_id.clone(),
        "image_type": image_type.clone(),
        "prompt": prompt.clone(),
        "width": input.width,
        "height": input.height,
        "provider": provider.clone(),
        "model": model.clone(),
        "request": request_json.clone(),
        "media_refs": media_refs_json.clone(),
    });
    let image_record = CreateImageRecord {
        project_id: input.project_id,
        image_type: image_type.clone(),
        file_path,
        prompt: prompt.clone(),
        model: model.clone(),
        width: input.width,
        height: input.height,
        file_size: Some(file_size),
        chapter_id,
        panel_id: input.panel_id,
        metadata_json,
        created_at: Some(input.created_at),
    };
    let image = state
        .image
        .create_record_with_generation_run_checkpoint(
            image_record,
            RecordCompletedGenerationRunWithCheckpointInput {
                run: RecordCompletedGenerationRunInput {
                    project_id: input.project_id,
                    workflow: "image_generation".to_string(),
                    step_key: command,
                    title: format!("{image_type} 图片 provider 生成"),
                    model_kind: "image".to_string(),
                    provider: Some(provider),
                    model: Some(model.clone()),
                    prompt_template_names_json: None,
                    prompt_template_hashes_json: None,
                    rendered_messages_json: None,
                    rendered_prompts_json: Some(json!({
                        "prompt": prompt,
                    })),
                    input_variables_json: None,
                    source_refs_json: None,
                    media_refs_json: media_refs_json.clone(),
                    request_json: Some(request_json),
                    response_summary_json: Some(json!({
                        "format": output_format.clone(),
                        "file_size": file_size,
                    })),
                    targets: Vec::new(),
                },
                checkpoint: GenerationRunCheckpointContext {
                    workflow_run_id: input.workflow_run_id,
                    call_key: Some(call_key),
                    input_fingerprint_sha256: Some(
                        generation_checkpoint_fingerprint_sha256(&input_fingerprint_json)
                            .map_err(|err| err.to_string())?,
                    ),
                    input_fingerprint_json: Some(input_fingerprint_json),
                    raw_response_json: Some(json!({
                        "format": output_format,
                        "model": model,
                        "file_size": file_size,
                    })),
                    parsed_output_json: None,
                    validation_json: None,
                    artifact_refs_json: None,
                    attempt: None,
                },
            },
        )
        .await
        .map_err(|err| err.to_string());
    let image = match image {
        Ok(image) => image,
        Err(err) => {
            let remove_err = std::fs::remove_file(&path)
                .err()
                .map(|remove_err| format!("同时删除已写出的图片文件失败：{remove_err}"));
            return Err(match remove_err {
                Some(remove_err) => format!("{err}；{remove_err}"),
                None => err,
            });
        }
    };
    Ok(image)
}

pub(crate) async fn run_queued_request_with_cancel<F, Fut, T>(
    state: &AppState,
    task_type: QueueTaskType,
    priority: TaskPriority,
    project_id: Option<i64>,
    cancel_token: CancellationToken,
    operation: F,
) -> Result<T, String>
where
    F: FnOnce() -> Fut,
    Fut: Future<Output = Result<T, String>>,
{
    run_queued_request_with_cancel_and_workflow(
        state,
        task_type,
        priority,
        project_id,
        None,
        cancel_token,
        operation,
    )
    .await
}

pub(crate) async fn run_queued_request_with_cancel_and_workflow<F, Fut, T>(
    state: &AppState,
    task_type: QueueTaskType,
    priority: TaskPriority,
    project_id: Option<i64>,
    workflow_run_id: Option<i64>,
    cancel_token: CancellationToken,
    operation: F,
) -> Result<T, String>
where
    F: FnOnce() -> Fut,
    Fut: Future<Output = Result<T, String>>,
{
    let submitted_id = if let Some(workflow_run_id) = workflow_run_id {
        state
            .task_queue
            .submit_with_workflow_context(
                task_type,
                priority,
                project_id,
                TaskWorkflowContext::new(workflow_run_id).map_err(|err| err.to_string())?,
            )
            .await
            .map_err(|err| err.to_string())?
    } else {
        state
            .task_queue
            .submit(task_type, priority, project_id)
            .await
            .map_err(|err| err.to_string())?
    };
    let Some(permit) = state
        .task_queue
        .acquire_task_with_cancel(&submitted_id, cancel_token)
        .await
        .map_err(|err| err.to_string())?
    else {
        if let Err(err) = state.task_queue.cancel(&submitted_id).await {
            tracing::warn!(
                task_id = %submitted_id,
                error = %err,
                "同步取消队列任务失败"
            );
        }
        return Err("任务已取消".to_string());
    };
    let running_id = permit.task_id().to_string();
    if let Some(workflow_run_id) = workflow_run_id
        && let Err(err) = state
            .workflow_run
            .mark_active_task(workflow_run_id, &running_id)
            .await
    {
        let message = err.to_string();
        permit.fail(message.clone());
        if let Err(status_err) = state.task_queue.mark_failed(&running_id, &message).await {
            tracing::warn!(
                task_id = %running_id,
                error = %status_err,
                "同步队列任务失败状态失败"
            );
        }
        return Err(message);
    }

    match operation().await {
        Ok(value) => {
            permit.complete();
            if let Err(err) = state.task_queue.mark_done(&running_id).await {
                tracing::warn!(
                    task_id = %running_id,
                    error = %err,
                    "同步队列任务完成状态失败"
                );
            }
            if let Some(workflow_run_id) = workflow_run_id
                && let Err(err) = state
                    .workflow_run
                    .clear_active_task_if_matches(workflow_run_id, &running_id)
                    .await
            {
                tracing::warn!(
                    workflow_run_id,
                    task_id = %running_id,
                    error = %err,
                    "清理 workflow active_task_id 失败"
                );
            }
            Ok(value)
        }
        Err(err) => {
            permit.fail(err.clone());
            if let Err(status_err) = state.task_queue.mark_failed(&running_id, &err).await {
                tracing::warn!(
                    task_id = %running_id,
                    error = %status_err,
                    "同步队列任务失败状态失败"
                );
            }
            if let Some(workflow_run_id) = workflow_run_id
                && let Err(clear_err) = state
                    .workflow_run
                    .clear_active_task_if_matches(workflow_run_id, &running_id)
                    .await
            {
                tracing::warn!(
                    workflow_run_id,
                    task_id = %running_id,
                    error = %clear_err,
                    "清理 workflow active_task_id 失败"
                );
            }
            Err(err)
        }
    }
}

fn save_image_bytes(
    state: &AppState,
    project_id: i64,
    chapter_id: Option<i64>,
    kind: &str,
    extra: Option<&str>,
    bytes: &[u8],
    format: &str,
) -> Result<PathBuf, String> {
    let kind = required_filename_segment(kind, "kind")?;
    let format = required_filename_segment(format, "format")?;
    let suffix = match extra {
        Some(value) => format!("_{}", required_filename_segment(value, "extra")?),
        None => String::new(),
    };
    let dir = image_output_dir(&state.config.data_dir, project_id, chapter_id)?;
    std::fs::create_dir_all(&dir).map_err(|e| format!("无法创建图片目录 {dir:?}：{e}"))?;
    let ts = chrono::Utc::now().timestamp_millis();
    let filename = format!("{kind}{suffix}_{ts}.{format}");
    let path = dir.join(filename);
    std::fs::write(&path, bytes).map_err(|e| format!("无法写入图片：{e}"))?;
    Ok(path)
}

fn image_output_dir(
    data_dir: &Path,
    project_id: i64,
    chapter_id: Option<i64>,
) -> Result<PathBuf, String> {
    let project_paths = ProjectPaths::new(data_dir, project_id);
    match chapter_id {
        Some(chapter_id) if chapter_id > 0 => Ok(project_paths.chapter_images_dir(chapter_id)),
        Some(chapter_id) => Err(format!(
            "章节图片目录需要有效 chapter_id，收到 {chapter_id}"
        )),
        None => Ok(project_paths.images_dir()),
    }
}

fn required_filename_segment<'a>(value: &'a str, field: &str) -> Result<&'a str, String> {
    let value = value.trim();
    if value.is_empty() {
        return Err(format!("图片文件名片段 {field} 不能为空"));
    }
    if !value
        .bytes()
        .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-'))
    {
        return Err(format!(
            "图片文件名片段 {field} 只能包含 ASCII 字母、数字、下划线或短横线"
        ));
    }
    Ok(value)
}

struct ImageGenerationTrace<'a> {
    command: &'a str,
    config: &'a image_gen_config::Model,
    image_type: &'a str,
    prompt: &'a str,
    provider_model: &'a str,
    chapter_id: Option<i64>,
    panel_id: Option<&'a str>,
    width: i32,
    height: i32,
    generated_at: chrono::NaiveDateTime,
}

fn attach_image_generation_trace(
    metadata_json: Option<Value>,
    trace: ImageGenerationTrace<'_>,
) -> Result<Option<Value>, String> {
    let generation = json!({
        "command": trace.command,
        "config_id": trace.config.id,
        "config_name": &trace.config.name,
        "backend": &trace.config.backend,
        "base_url": &trace.config.base_url,
        "configured_model": &trace.config.model,
        "provider_model": trace.provider_model,
        "image_type": trace.image_type,
        "chapter_id": trace.chapter_id,
        "panel_id": trace.panel_id,
        "width": trace.width,
        "height": trace.height,
        "prompt": trace.prompt,
        "generated_at": trace.generated_at.to_string(),
        "runtime_config_snapshot": crate::runtime_config::redacted_current_snapshot_value(),
    });
    let metadata = match metadata_json {
        Some(Value::Object(mut map)) => {
            map.insert("generation".into(), generation);
            Some(Value::Object(map))
        }
        Some(_) => return Err("图片生成 metadata_json 必须是对象".to_string()),
        None => Some(json!({
            "generation": generation,
        })),
    };
    Ok(normalize_image_metadata(metadata))
}

fn image_generation_media_refs(metadata_json: &Value) -> Option<Value> {
    let references = metadata_json
        .get("generation_context")
        .and_then(|context| context.get("reference_image_ids"))
        .or_else(|| metadata_json.get("reference_image_ids"))
        .cloned();
    let bindings = metadata_json
        .get("reference_bindings")
        .cloned()
        .or_else(|| {
            metadata_json
                .get("generation_context")
                .and_then(|context| context.get("reference_bindings").cloned())
        });
    match (references, bindings) {
        (Some(references), Some(bindings)) => Some(json!({
            "reference_image_ids": references,
            "reference_bindings": bindings,
        })),
        (Some(references), None) => Some(json!({
            "reference_image_ids": references,
        })),
        (None, Some(bindings)) => Some(json!({
            "reference_bindings": bindings,
        })),
        (None, None) => None,
    }
}

fn image_request_aspect_ratio(width: i32, height: i32) -> String {
    let gcd = gcd_i32(width.abs().max(1), height.abs().max(1));
    format!("{}:{}", width.abs().max(1) / gcd, height.abs().max(1) / gcd)
}

fn gcd_i32(mut left: i32, mut right: i32) -> i32 {
    while right != 0 {
        let next = left % right;
        left = right;
        right = next;
    }
    left.max(1)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn image_config() -> image_gen_config::Model {
        let now = chrono::Utc::now().naive_utc();
        image_gen_config::Model {
            id: 1,
            name: "primary".to_string(),
            backend: "openai-compatible-image".to_string(),
            base_url: Some("http://localhost:3000".to_string()),
            api_key_encrypted: Some("encrypted".to_string()),
            model: "image-model".to_string(),
            max_concurrent: 1,
            default_width: 1024,
            default_height: 1024,
            provider_options_json: None,
            is_default: true,
            created_at: now,
            updated_at: now,
        }
    }

    fn trace<'a>(config: &'a image_gen_config::Model) -> ImageGenerationTrace<'a> {
        ImageGenerationTrace {
            command: "generate_image",
            config,
            image_type: "scene",
            prompt: "test prompt",
            provider_model: "provider-model",
            chapter_id: Some(7),
            panel_id: Some("panel-1"),
            width: 1024,
            height: 576,
            generated_at: chrono::Utc::now().naive_utc(),
        }
    }

    #[test]
    fn filename_segment_rejects_empty_or_path_like_values() {
        let empty = required_filename_segment("  ", "kind")
            .expect_err("empty filename segment should fail");
        assert!(empty.contains("不能为空"));

        let slash = required_filename_segment("../scene", "kind")
            .expect_err("path-like filename segment should fail");
        assert!(slash.contains("只能包含"));
    }

    #[test]
    fn filename_segment_accepts_safe_ascii_values() {
        let segment = required_filename_segment("manga_page-01", "kind")
            .expect("safe filename segment should pass");

        assert_eq!(segment, "manga_page-01");
    }

    #[test]
    fn image_request_aspect_ratio_reduces_dimensions() {
        assert_eq!(image_request_aspect_ratio(1024, 576), "16:9");
        assert_eq!(image_request_aspect_ratio(576, 1024), "9:16");
    }

    #[test]
    fn attach_image_generation_trace_requires_object_metadata() {
        let config = image_config();
        let err = attach_image_generation_trace(Some(json!(["legacy"])), trace(&config))
            .expect_err("non-object image metadata should fail explicitly");

        assert!(err.contains("metadata_json 必须是对象"));
    }

    #[test]
    fn attach_image_generation_trace_preserves_object_metadata() {
        let config = image_config();
        let metadata = attach_image_generation_trace(
            Some(json!({
                "reference_image_ids": [1, 2],
            })),
            trace(&config),
        )
        .expect("object image metadata should accept")
        .expect("image metadata should be present");

        assert_eq!(metadata["metadata_schema_version"], 1);
        assert_eq!(metadata["reference_image_ids"], json!([1, 2]));
        assert_eq!(metadata["generation"]["command"], "generate_image");
    }
}
