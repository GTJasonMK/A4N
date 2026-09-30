//! 画布节点输入：边序、config 读取、节点种类纯规则（无 DB、无 Host）。

use a4n_db::canvas::CanvasGraphRecordDocument;
use a4n_domain::canvas::{CanvasEdgeRecord, CanvasNodeRecord};
use serde_json::Value;

/// 指向 `node_id` 的入边。
pub fn incoming_edges(
    document: &CanvasGraphRecordDocument,
    node_id: i64,
) -> Vec<&CanvasEdgeRecord> {
    document
        .edges
        .iter()
        .filter(|edge| edge.target_node_id == node_id)
        .collect()
}

pub(crate) fn source_node(
    document: &CanvasGraphRecordDocument,
    source_node_id: i64,
) -> Result<&CanvasNodeRecord, String> {
    document
        .nodes
        .iter()
        .find(|candidate| candidate.id == source_node_id)
        .ok_or_else(|| format!("画布连线引用不存在的源节点: {source_node_id}"))
}

/// Assemble 片段连线 `config_json.order_index`（正整数）。
pub fn canvas_edge_order_index(edge: &CanvasEdgeRecord) -> Result<i64, String> {
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
pub fn canvas_config_i64(config: &Value, key: &str) -> Result<Option<i64>, String> {
    let Some(value) = config.get(key).filter(|value| !value.is_null()) else {
        return Ok(None);
    };
    value
        .as_i64()
        .filter(|value| *value > 0)
        .map(Some)
        .ok_or_else(|| format!("{key} 必须是正整数"))
}

pub fn canvas_config_i32(config: &Value, key: &str) -> Result<Option<i32>, String> {
    canvas_config_i64(config, key)?
        .map(|value| i32::try_from(value).map_err(|_| format!("{key} 超出 i32 范围")))
        .transpose()
}

/// 非负有限 f32 config 字段。
pub fn canvas_config_f32(config: &Value, key: &str) -> Result<Option<f32>, String> {
    let Some(value) = config.get(key).filter(|value| !value.is_null()) else {
        return Ok(None);
    };
    let value = value
        .as_f64()
        .filter(|value| value.is_finite() && *value >= 0.0 && *value <= f64::from(f32::MAX))
        .ok_or_else(|| format!("{key} 必须是非负有限 f32 数字"))?;
    let converted = value as f32;
    if value > 0.0 && converted == 0.0 {
        return Err(format!("{key} 小于 f32 可表示的范围"));
    }
    Ok(Some(converted))
}

pub fn ensure_canvas_provider_slot_ready(config: &Value, capability: &str) -> Result<(), String> {
    if config
        .get("a4n_provider_slot_unresolved")
        .and_then(Value::as_bool)
        == Some(true)
    {
        return Err(format!(
            "画布节点缺少可用的 {capability} Provider 配置；请先绑定本机配置，系统不会静默回退默认模型"
        ));
    }
    Ok(())
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

/// 节点是否已经保存了可供后续画布运行复用的最近产物。
pub fn canvas_node_has_persisted_runtime_result(node: &CanvasNodeRecord) -> bool {
    node.config_json
        .get("canvas_runtime_result")
        .and_then(Value::as_bool)
        == Some(true)
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
        assert_eq!(canvas_config_i64(&cfg, "llm_config_id").unwrap(), Some(3));
        assert!(canvas_node_is_operation("GenerateVideoNode"));
        assert!(!canvas_node_is_operation("TextNode"));
    }

    #[test]
    fn integer_options_distinguish_absence_from_invalid_values() {
        assert_eq!(
            canvas_config_i32(&json!({}), "duration_secs").unwrap(),
            None
        );
        assert_eq!(
            canvas_config_i32(&json!({"duration_secs": null}), "duration_secs").unwrap(),
            None
        );
        assert_eq!(
            canvas_config_i32(&json!({"duration_secs": 2}), "duration_secs").unwrap(),
            Some(2)
        );
        for value in [
            json!(1.5),
            json!(0),
            json!(-1),
            json!("2"),
            json!(true),
            json!(i64::from(i32::MAX) + 1),
        ] {
            assert!(canvas_config_i32(&json!({"duration_secs": value}), "duration_secs").is_err());
        }
    }

    #[test]
    fn floating_options_reject_overflow_underflow_and_invalid_values() {
        assert_eq!(canvas_config_f32(&json!({}), "temperature").unwrap(), None);
        assert_eq!(
            canvas_config_f32(&json!({"temperature": null}), "temperature").unwrap(),
            None
        );
        assert_eq!(
            canvas_config_f32(&json!({"temperature": 0.0}), "temperature").unwrap(),
            Some(0.0)
        );
        assert_eq!(
            canvas_config_f32(&json!({"temperature": 0.5}), "temperature").unwrap(),
            Some(0.5)
        );
        for value in [
            json!(1e100),
            json!(1e-100),
            json!(-1),
            json!("0.5"),
            json!(true),
        ] {
            assert!(canvas_config_f32(&json!({"temperature": value}), "temperature").is_err());
        }
    }
}
