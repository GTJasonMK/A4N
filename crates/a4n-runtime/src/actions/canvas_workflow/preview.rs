use super::*;
use crate::actions::image_generation::preview_canvas_image_request;
use crate::actions::video_generation::preview_canvas_video_request;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PreviewCanvasNodeRequestInput {
    pub project_id: i64,
    pub graph_id: i64,
    pub node_id: i64,
    pub input_overrides: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CanvasNodeRequestPreview {
    pub project_id: i64,
    pub graph_id: i64,
    pub node_id: i64,
    pub node_kind: String,
    pub status: String,
    pub note: String,
    pub request: Value,
}

pub async fn preview_canvas_node_request(
    state: &AppState,
    user_id: i64,
    input: PreviewCanvasNodeRequestInput,
) -> Result<CanvasNodeRequestPreview, String> {
    access::verify_project_access_string(state, user_id, input.project_id).await?;
    let document = state
        .canvas
        .get_or_create_graph(input.project_id, Some(input.graph_id))
        .await
        .map_err(|e| e.to_string())?;
    validate_canvas_document_edge_contract(&document).map_err(|e| e.to_string())?;
    let node = document
        .nodes
        .iter()
        .find(|node| node.id == input.node_id)
        .ok_or_else(|| format!("画布节点不存在: {}", input.node_id))?;
    if node.node_kind == "GenerateVideoNode" {
        let video_input = match build_canvas_video_input_for_run(
            Some(state),
            None,
            user_id,
            input.project_id,
            input.graph_id,
            node,
            &document,
        )
        .await
        {
            Ok(video_input) => video_input,
            Err(reason) => {
                return Ok(blocked_canvas_node_request_preview(
                    &input, node, &document, reason,
                ));
            }
        };
        let preview = match preview_canvas_video_request(state, video_input).await {
            Ok(preview) => preview,
            Err(reason) => {
                return Ok(blocked_canvas_node_request_preview(
                    &input, node, &document, reason,
                ));
            }
        };
        return Ok(CanvasNodeRequestPreview {
            project_id: input.project_id,
            graph_id: input.graph_id,
            node_id: input.node_id,
            node_kind: node.node_kind.clone(),
            status: "preview".to_string(),
            note: "已构造 GenerateVideoNode 的视频 provider 请求预览；预览不会调用外部模型。"
                .to_string(),
            request: serde_json::to_value(preview).map_err(|err| err.to_string())?,
        });
    }
    if node.node_kind == "GenerateImageNode" {
        let image_input = match build_canvas_image_input_for_run(
            Some(state),
            None,
            user_id,
            input.project_id,
            input.graph_id,
            node,
            &document,
        )
        .await
        {
            Ok(image_input) => image_input,
            Err(reason) => {
                return Ok(blocked_canvas_node_request_preview(
                    &input, node, &document, reason,
                ));
            }
        };
        let preview = match preview_canvas_image_request(state, user_id, image_input).await {
            Ok(preview) => preview,
            Err(reason) => {
                return Ok(blocked_canvas_node_request_preview(
                    &input, node, &document, reason,
                ));
            }
        };
        return Ok(CanvasNodeRequestPreview {
            project_id: input.project_id,
            graph_id: input.graph_id,
            node_id: input.node_id,
            node_kind: node.node_kind.clone(),
            status: "preview".to_string(),
            note: "已构造 GenerateImageNode 的图片 provider 请求预览；预览不会调用外部模型。"
                .to_string(),
            request: serde_json::to_value(preview).map_err(|err| err.to_string())?,
        });
    }
    if node.node_kind == "LlmTextNode" {
        let llm_input = match build_canvas_llm_transform_input_for_run(
            Some(state),
            None,
            node,
            &document,
        )
        .await
        {
            Ok(llm_input) => llm_input,
            Err(reason) => {
                return Ok(blocked_canvas_node_request_preview(
                    &input, node, &document, reason,
                ));
            }
        };
        return Ok(CanvasNodeRequestPreview {
            project_id: input.project_id,
            graph_id: input.graph_id,
            node_id: input.node_id,
            node_kind: node.node_kind.clone(),
            status: "preview".to_string(),
            note: "已构造 LlmTextNode 的 LLM 请求预览；预览不会调用外部模型。".to_string(),
            request: json!({
                "config_id": llm_input.config_id,
                "output_kind": llm_input.output_kind,
                "temperature": llm_input.temperature,
                "max_tokens": llm_input.max_tokens,
                "messages": [
                    { "role": "system", "content": llm_input.system_prompt },
                    { "role": "user", "content": llm_input.user_prompt }
                ],
                "source_node_ids": llm_input.source_node_ids,
                "canvas_text_asset_id": llm_input.canvas_text_asset_id
            }),
        });
    }

    Ok(CanvasNodeRequestPreview {
        project_id: input.project_id,
        graph_id: input.graph_id,
        node_id: input.node_id,
        node_kind: node.node_kind.clone(),
        status: "preview".to_string(),
        note: "预览只展示画布节点配置和输入覆盖，不调用外部 provider。真实 provider 组装将在画布执行器接入节点类型后启用。"
            .to_string(),
        request: json!({
            "node_kind": node.node_kind,
            "ref_type": node.ref_type,
            "ref_id": node.ref_id,
            "config": node.config_json,
            "input_overrides": input.input_overrides.unwrap_or_else(|| json!({}))
        }),
    })
}

fn blocked_canvas_node_request_preview(
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
        status: "blocked".to_string(),
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
