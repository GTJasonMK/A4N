use a4n_core::models::canvas::canvas_edge;
use a4n_orchestrator::canvas::CanvasGraphRecordDocument as CanvasGraphDocument;
use serde_json::Value;

pub(super) fn incoming_edges(
    document: &CanvasGraphDocument,
    node_id: i64,
) -> Vec<&canvas_edge::Model> {
    document
        .edges
        .iter()
        .filter(|edge| edge.target_node_id == node_id)
        .collect()
}

pub(super) fn canvas_edge_order_index(edge: &canvas_edge::Model) -> Result<i64, String> {
    let order_index = edge
        .config_json
        .get("order_index")
        .and_then(Value::as_i64)
        .ok_or_else(|| "AssembleVideoNode 片段连线必须设置 order_index".to_string())?;
    if order_index <= 0 {
        return Err("AssembleVideoNode 片段连线 order_index 必须是正整数".to_string());
    }
    Ok(order_index)
}

pub(super) fn validate_canvas_clip_order(clips: &[(i64, i64, i64)]) -> Result<(), String> {
    for (index, (order_index, _, _)) in clips.iter().enumerate() {
        let expected = i64::try_from(index + 1)
            .map_err(|_| "AssembleVideoNode 片段数量超出 i64 范围".to_string())?;
        if *order_index != expected {
            return Err(format!(
                "AssembleVideoNode 片段顺序必须从 1 连续递增，缺少 order_index={expected}"
            ));
        }
    }
    Ok(())
}

pub(super) fn canvas_config_string(config: &Value, keys: &[&str]) -> Option<String> {
    keys.iter().find_map(|key| {
        config
            .get(*key)
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_string)
    })
}

pub(super) fn canvas_config_i64(config: &Value, key: &str) -> Option<i64> {
    config
        .get(key)
        .and_then(Value::as_i64)
        .filter(|value| *value > 0)
}

pub(super) fn canvas_node_is_operation(kind: &str) -> bool {
    matches!(
        kind,
        "RenderTextNode"
            | "LlmTextNode"
            | "GenerateImageNode"
            | "GenerateVideoNode"
            | "ExtractFrameNode"
            | "TrimVideoNode"
            | "AssembleVideoNode"
            | "TextProbeNode"
            | "ImageProbeNode"
            | "VideoProbeNode"
            | "AudioProbeNode"
    )
}

pub(super) fn canvas_config_f32(config: &Value, key: &str) -> Option<f32> {
    config
        .get(key)
        .and_then(Value::as_f64)
        .filter(|value| value.is_finite() && *value >= 0.0)
        .map(|value| value as f32)
}

pub(super) fn i64_to_i32(value: i64) -> Option<i32> {
    i32::try_from(value).ok().filter(|value| *value > 0)
}
