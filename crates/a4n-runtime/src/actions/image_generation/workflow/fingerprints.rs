use super::super::batch::BatchImageJob;
use a4n_core::source_spans::source_text_hash as source_content_sha256;
use serde_json::{Value, json};

pub(super) fn batch_image_workflow_business_key(
    project_id: i64,
    requests_json: &Value,
) -> Result<String, String> {
    let serialized = serde_json::to_string(requests_json)
        .map_err(|err| format!("批量图片请求序列化失败: {err}"))?;
    Ok(format!(
        "image_batch:{project_id}:{}",
        source_content_sha256(&serialized)
    ))
}

pub(super) fn batch_image_workflow_fingerprint(project_id: i64, requests_json: &Value) -> Value {
    json!({
        "workflow": "image_batch",
        "project_id": project_id,
        "requests": requests_json,
    })
}

pub(crate) fn batch_image_item_step_key(index: usize) -> String {
    format!("image_batch.item.{}", index + 1)
}

pub(crate) fn batch_image_item_fingerprint(
    project_id: i64,
    job: &BatchImageJob,
    request_json: &Value,
) -> Value {
    json!({
        "workflow": "image_batch",
        "project_id": project_id,
        "index": job.index,
        "image_type": job.image_type,
        "chapter_id": job.chapter_id,
        "panel_id": job.panel_id,
        "metadata_json": job.metadata_json,
        "request": request_json,
    })
}

pub(super) fn image_generation_workflow_fingerprint(
    project_id: i64,
    chapter_id: Option<i64>,
    command: &str,
    image_type: &str,
    panel_id: Option<&str>,
    request_json: &Value,
    metadata_json: Option<&Value>,
) -> Value {
    json!({
        "workflow": "image_generation",
        "project_id": project_id,
        "chapter_id": chapter_id,
        "command": command,
        "image_type": image_type,
        "panel_id": panel_id,
        "request": request_json,
        "metadata": metadata_json,
    })
}

pub(super) fn image_generation_step_key(command: &str) -> String {
    format!("image_generation.{command}")
}
