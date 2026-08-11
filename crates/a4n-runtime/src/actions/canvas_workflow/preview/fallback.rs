use super::contracts::{CanvasNodeRequestPreview, PreviewCanvasNodeRequestInput};
use a4n_core::models::canvas::canvas_node;
use a4n_orchestrator::canvas::CanvasGraphRecordDocument as CanvasGraphDocument;
use serde_json::json;

pub(super) fn generic_canvas_node_request_preview(
    input: &PreviewCanvasNodeRequestInput,
    node: &canvas_node::Model,
) -> CanvasNodeRequestPreview {
    CanvasNodeRequestPreview {
        project_id: input.project_id,
        graph_id: input.graph_id,
        node_id: input.node_id,
        node_kind: node.node_kind.clone(),
        note: "预览只展示画布节点配置和输入覆盖，不调用外部 provider。真实 provider 组装将在画布执行器接入节点类型后启用。"
            .to_string(),
        request: json!({
            "node_kind": node.node_kind,
            "ref_type": node.ref_type,
            "ref_id": node.ref_id,
            "config": node.config_json,
            "input_overrides": input.input_overrides.clone().unwrap_or_else(|| json!({}))
        }),
    }
}

pub(super) fn blocked_canvas_node_request_preview(
    input: &PreviewCanvasNodeRequestInput,
    node: &canvas_node::Model,
    document: &CanvasGraphDocument,
    reason: String,
) -> CanvasNodeRequestPreview {
    let incoming_edges = document
        .edges
        .iter()
        .filter(|edge| edge.target_node_id == node.id)
        .map(|edge| {
            let source = document
                .nodes
                .iter()
                .find(|candidate| candidate.id == edge.source_node_id);
            json!({
                "edge_id": edge.id,
                "source_node_id": edge.source_node_id,
                "source_node_kind": source.map(|source| source.node_kind.as_str()),
                "source_port": edge.source_port,
                "target_port": edge.target_port,
                "role": edge.role,
            })
        })
        .collect::<Vec<_>>();
    CanvasNodeRequestPreview {
        project_id: input.project_id,
        graph_id: input.graph_id,
        node_id: input.node_id,
        node_kind: node.node_kind.clone(),
        note: format!("当前节点暂时无法构造真实 provider 请求：{reason}"),
        request: json!({
            "status": "blocked",
            "reason": reason,
            "node": {
                "id": node.id,
                "node_kind": node.node_kind,
                "ref_type": node.ref_type,
                "ref_id": node.ref_id,
                "config": node.config_json,
                "state": node.state,
            },
            "incoming_edges": incoming_edges,
            "input_overrides": input.input_overrides.clone().unwrap_or_else(|| json!({})),
            "next_action": "先运行上游链路，或选择输出节点使用“运行到此节点”；上游产物存在后再预览该节点可看到真实 provider 请求。"
        }),
    }
}
