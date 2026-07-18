use super::super::super::inputs::CanvasLlmTransformInput;
use super::super::checkpoint::load_canvas_llm_text_checkpoint;
use super::super::request::build_canvas_llm_transform_request;
use super::contracts::{CanvasLlmCallResult, CanvasLlmCheckpointWrite};
use crate::AppState;
use crate::actions::workflow_checkpoint::llm_call_checkpoint_fingerprint_sha256;
use a4n_core::error::AppError;
use a4n_core::models::canvas::canvas_node;
use a4n_llm::trace::context_aware_chat_request_trace_json;
use a4n_queue::priority::{TaskPriority, TaskType as QueueTaskType};
use a4n_queue::queue::TaskWorkflowContext;
use serde_json::json;

pub(super) async fn execute_canvas_llm_text_call(
    state: &AppState,
    project_id: i64,
    graph_id: i64,
    node: &canvas_node::Model,
    llm_input: &CanvasLlmTransformInput,
    workflow_run_id: i64,
    step_key: &str,
    mode: &str,
) -> Result<CanvasLlmCallResult, String> {
    let mut request = build_canvas_llm_transform_request(
        project_id,
        graph_id,
        node,
        llm_input,
        workflow_run_id,
        step_key,
        mode,
    )?;
    let call_key = format!("{step_key}:llm_transform");
    let call_order = i32::try_from(node.id)
        .map_err(|_| format!("画布节点 ID 超出调用级 checkpoint 顺序范围: {}", node.id))?;
    let request_json = context_aware_chat_request_trace_json(&request)
        .map_err(|err| format!("LlmTextNode 请求序列化失败: {err}"))?;
    let input_fingerprint_json = json!({
        "workflow": super::super::super::WORKFLOW_KIND_VIDEO_CANVAS_GRAPH,
        "call_kind": "canvas_llm_transform",
        "project_id": project_id,
        "graph_id": graph_id,
        "node_id": node.id,
        "mode": mode,
        "request": request_json});
    let input_fingerprint_sha256 = llm_call_checkpoint_fingerprint_sha256(&input_fingerprint_json)
        .map_err(|err| format!("LlmTextNode 调用级 fingerprint 生成失败: {err}"))?;
    if let Some(trace_context) = request.trace_context.as_mut() {
        trace_context.call_key = Some(call_key.clone());
        trace_context.input_fingerprint_sha256 = Some(input_fingerprint_sha256);
    }
    if let Some(restored) =
        load_canvas_llm_text_checkpoint(state, workflow_run_id, &call_key, &input_fingerprint_json)
            .await?
    {
        return Ok(CanvasLlmCallResult {
            call_key,
            response_content: restored.response_content,
            usage: restored.usage,
            finish_reason: restored.finish_reason,
            restored_parsed_text: Some(restored.parsed_text),
            checkpoint_write: None,
        });
    }

    let workflow_context =
        TaskWorkflowContext::new(workflow_run_id).map_err(|err| err.to_string())?;
    let response = state
        .task_queue
        .run_tracked_with_workflow_context(
            QueueTaskType::LlmCall,
            TaskPriority::Medium,
            Some(project_id),
            workflow_context,
            || async {
                let provider = state.dynamic_llm_provider_for_profile()?;
                provider
                    .chat_with_config_id(llm_input.config_id, request)
                    .await
                    .map_err(|err| AppError::LlmError(err.to_string()))
            },
        )
        .await
        .map_err(|err| format!("LlmTextNode 调用 LLM 失败: {err}"))?;
    Ok(CanvasLlmCallResult {
        call_key,
        response_content: response.message.content,
        usage: response.usage,
        finish_reason: response.finish_reason,
        restored_parsed_text: None,
        checkpoint_write: Some(CanvasLlmCheckpointWrite {
            call_order,
            input_fingerprint_json,
        }),
    })
}
