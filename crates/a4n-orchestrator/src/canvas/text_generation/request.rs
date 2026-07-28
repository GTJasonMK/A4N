use a4n_core::source_spans::source_text_hash as source_content_sha256;
use a4n_llm::{
    context::{
        COMPRESSION_FOCUSED_RAW, COMPRESSION_FULL_RAW, COMPRESSION_MINIMAL, LlmContextLedger,
        LlmContextLedgerPackDraft, context_aware_system_user_request,
    },
    provider::{ChatRequest, LlmTraceContext},
};
use serde_json::json;

use crate::canvas::WORKFLOW_KIND_VIDEO_CANVAS_GRAPH;

use super::contracts::{CanvasLlmTextInput, CanvasTextNodeExecutionInput};

pub(super) fn build_canvas_llm_text_request(
    input: &CanvasTextNodeExecutionInput<'_>,
    llm_input: &CanvasLlmTextInput,
    workflow_run_id: i64,
    step_key: &str,
) -> Result<ChatRequest, String> {
    let node_config_hash = source_content_sha256(
        &serde_json::to_string(&input.node.config_json)
            .map_err(|error| format!("LlmTextNode 节点配置序列化失败: {error}"))?,
    );
    let source_context_hash = source_content_sha256(&llm_input.source_context);
    let node_config_pack = json!({
        "node_kind": input.node.node_kind,
        "node_id": input.node.id,
        "output_kind": llm_input.output_kind,
        "instruction": llm_input.instruction,
        "config_id": llm_input.config_id,
        "node_config_sha256": node_config_hash,
    });
    let mut ledger = LlmContextLedger::new();
    ledger
        .add_pack(a4n_llm::context::LlmContextLedgerPackInput {
            scope_kind: WORKFLOW_KIND_VIDEO_CANVAS_GRAPH.into(),
            pack_kind: "canvas_node_config:LlmTextNode".into(),
            source_refs_json: Some(json!({
                "graph_id": input.graph_id,
                "node_id": input.node.id,
                "node_config_sha256": node_config_hash,
            })),
            config_identity_json: json!({
                "ledger_version": "canvas_llm_transform_context_v1",
                "node_kind": "LlmTextNode",
            }),
            required: true,
            stable_prefix: true,
            variants: vec![
                LlmContextLedgerPackDraft::new(COMPRESSION_FULL_RAW, node_config_pack.clone()),
                LlmContextLedgerPackDraft::new(COMPRESSION_FOCUSED_RAW, node_config_pack.clone()),
                LlmContextLedgerPackDraft::new(COMPRESSION_MINIMAL, node_config_pack),
            ],
        })
        .map_err(|error| error.to_string())?;
    let dynamic_payload = json!({
        "context_refs": {
            "node_config": "稳定上下文包 canvas_node_config:LlmTextNode",
        },
        "run": {
            "mode": input.mode,
            "graph_id": input.graph_id,
            "node_id": input.node.id,
            "source_node_ids": llm_input.source_node_ids,
        },
        "upstream_artifacts": {
            "source_context_sha256": source_context_hash,
            "text": llm_input.source_context,
        },
        "output_contract": {
            "format": "plain_text",
            "output_kind": llm_input.output_kind,
            "rules": [
                "只返回转换后的正文。",
                "不要输出解释、Markdown 包裹、字段名或处理过程。",
                "输出必须能够直接传给下游画布节点。"
            ],
        },
    });
    let dynamic_user_content = format!(
        "请执行当前画布 LLM 文本节点。只返回 output_contract 要求的纯文本。\n\n输入：\n{}",
        serde_json::to_string_pretty(&dynamic_payload)
            .map_err(|error| format!("LlmTextNode 动态输入序列化失败: {error}"))?
    );
    let manifest = ledger.into_manifest(
        dynamic_user_content,
        Some(
            "画布 LLM 节点必须只处理当前节点职责：按稳定节点配置转换上游文本，不存储产物、不访问未连接节点。"
                .to_string(),
        ),
    );
    let fingerprint = source_content_sha256(
        &serde_json::to_string(&json!({
            "graph_id": input.graph_id,
            "node_id": input.node.id,
            "mode": input.mode,
            "node_config_sha256": node_config_hash,
            "source_context_sha256": source_context_hash,
            "output_kind": llm_input.output_kind,
        }))
        .map_err(|error| format!("LlmTextNode fingerprint 序列化失败: {error}"))?,
    );
    Ok(context_aware_system_user_request(
        llm_input.system_prompt.clone(),
        manifest,
        llm_input.temperature,
        llm_input.max_tokens,
        None,
    )
    .map_err(|error| error.to_string())?
    .with_trace_context(LlmTraceContext {
        project_id: Some(input.project_id),
        root_project_id: Some(input.project_id),
        workflow: WORKFLOW_KIND_VIDEO_CANVAS_GRAPH.to_string(),
        step_key: step_key.to_string(),
        generation_run_id: None,
        workflow_run_id: Some(workflow_run_id),
        call_key: Some(step_key.to_string()),
        input_fingerprint_sha256: Some(fingerprint),
        context_plan_json: Some(json!({
            "canvas_graph_id": input.graph_id,
            "canvas_node_id": input.node.id,
            "output_kind": llm_input.output_kind,
            "config_id": llm_input.config_id,
            "node_config_sha256": node_config_hash,
            "source_context_sha256": source_context_hash,
        })),
        context_pack_refs_json: None,
    }))
}
