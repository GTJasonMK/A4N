use serde_json::{Value, json};

pub(super) fn probe_node_data_type(node_kind: &str) -> Result<&'static str, String> {
    match node_kind {
        "TextProbeNode" => Ok("text"),
        "ImageProbeNode" => Ok("image"),
        "VideoProbeNode" => Ok("video"),
        "AudioProbeNode" => Ok("audio"),
        other => Err(format!("探针节点类型无效: {other}")),
    }
}

pub(super) fn canvas_probe_observation_patch(
    artifact_json: &Value,
    source_node_ids: &[i64],
) -> Value {
    json!({
        "observed_data_type": artifact_json.get("data_type").and_then(Value::as_str),
        "observed_text": artifact_json.get("text").and_then(Value::as_str),
        "generated_image_id": artifact_json.get("generated_image_id").and_then(Value::as_i64),
        "generated_video_id": artifact_json.get("generated_video_id").and_then(Value::as_i64),
        "media_asset_id": artifact_json.get("media_asset_id").and_then(Value::as_i64),
        "file_path": artifact_json.get("file_path").and_then(Value::as_str),
        "thumbnail_path": artifact_json.get("thumbnail_path").and_then(Value::as_str),
        "duration_secs": artifact_json.get("duration_secs").and_then(Value::as_f64),
        "source_node_ids": source_node_ids})
}
