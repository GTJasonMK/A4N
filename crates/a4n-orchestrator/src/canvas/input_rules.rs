//! 画布节点输入：边序、config 读取、节点种类纯规则（无 DB、无 Host）。

use a4n_db::canvas::CanvasGraphRecordDocument;
use a4n_db_models::models::canvas::{canvas_edge, canvas_node};
use serde_json::Value;

/// 指向 `node_id` 的入边。
pub fn incoming_edges(
    document: &CanvasGraphRecordDocument,
    node_id: i64,
) -> Vec<&canvas_edge::Model> {
    document
        .edges
        .iter()
        .filter(|edge| edge.target_node_id == node_id)
        .collect()
}

pub(crate) fn source_node(
    document: &CanvasGraphRecordDocument,
    source_node_id: i64,
) -> Result<&canvas_node::Model, String> {
    document
        .nodes
        .iter()
        .find(|candidate| candidate.id == source_node_id)
        .ok_or_else(|| format!("画布连线引用不存在的源节点: {source_node_id}"))
}

/// Assemble 片段连线 `config_json.order_index`（正整数）。
pub fn canvas_edge_order_index(edge: &canvas_edge::Model) -> Result<i64, String> {
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

/// 片段顺序必须从 1 连续递增。
pub fn validate_canvas_clip_order(clips: &[(i64, i64, i64)]) -> Result<(), String> {
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

/// 从 config 取非空字符串。
pub fn canvas_config_string(config: &Value, key: &str) -> Option<String> {
    config
        .get(key)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}

/// 正整数 config 字段。
pub fn canvas_config_i64(config: &Value, key: &str) -> Option<i64> {
    config
        .get(key)
        .and_then(Value::as_i64)
        .filter(|value| *value > 0)
}

/// 非负有限 f32 config 字段。
pub fn canvas_config_f32(config: &Value, key: &str) -> Option<f32> {
    config
        .get(key)
        .and_then(Value::as_f64)
        .filter(|value| value.is_finite() && *value >= 0.0)
        .map(|value| value as f32)
}

/// 是否为可执行操作节点 kind。
pub fn canvas_node_is_operation(kind: &str) -> bool {
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

/// 正 i64 → i32。
pub fn i64_to_i32(value: i64) -> Option<i32> {
    i32::try_from(value).ok().filter(|value| *value > 0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn clip_order_and_config() {
        assert!(validate_canvas_clip_order(&[(1, 1, 1), (2, 2, 2)]).is_ok());
        assert!(validate_canvas_clip_order(&[(1, 1, 1), (3, 2, 2)]).is_err());
        let cfg = json!({"template": " hi ", "llm_config_id": 3, "temp": 0.5});
        assert_eq!(
            canvas_config_string(&cfg, "template").as_deref(),
            Some("hi")
        );
        assert_eq!(canvas_config_i64(&cfg, "llm_config_id"), Some(3));
        assert!(canvas_node_is_operation("GenerateVideoNode"));
        assert!(!canvas_node_is_operation("TextNode"));
    }
}
