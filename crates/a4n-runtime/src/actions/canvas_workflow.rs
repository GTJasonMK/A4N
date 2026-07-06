use crate::AppState;
use crate::actions::access;
use crate::actions::image_generation::{GeneratedImageSummary, generate_canvas_image};
use crate::actions::video_generation::{GeneratedVideoSummary, generate_canvas_video};
use crate::actions::video_records::{
    assemble_canvas_videos, extract_canvas_video_frame, trim_video,
};
use a4n_core::error::AppError;
use a4n_core::models::{
    ai::{generated_image, generated_video},
    canvas::{canvas_node, canvas_text_asset},
    media::media_asset,
};
use a4n_engine::llm::provider::{
    ChatRequest, LlmTraceContext, UsageInfo, is_truncated_finish_reason,
};
use a4n_engine::queue::priority::{TaskPriority, TaskType as QueueTaskType};
use a4n_services::canvas::{
    CanvasGraphDocument, CanvasGraphInput, UpsertCanvasTextAssetInput,
    validate_canvas_document_edge_contract,
};
use a4n_services::generation::context_aware_chat_request_trace_json;
use a4n_services::llm_checkpoint::{
    CompletedLlmCallCheckpointInput, llm_call_checkpoint_fingerprint_sha256,
    load_completed_llm_call_checkpoint, required_llm_call_checkpoint_json,
    save_completed_llm_call_checkpoint,
};
use a4n_services::llm_context::{
    COMPRESSION_FOCUSED_RAW, COMPRESSION_FULL_RAW, COMPRESSION_MINIMAL, LlmContextLedger,
    LlmContextLedgerPackDraft, context_aware_system_user_request,
};
use a4n_services::queue::TaskWorkflowContext;
use a4n_services::source::source_content_sha256;
use a4n_services::workflow::{
    UpsertWorkflowRunInput, WORKFLOW_STATUS_CANCELLED, WORKFLOW_STATUS_COMPLETED,
    WORKFLOW_STATUS_FAILED, WORKFLOW_STATUS_RUNNING,
};
use sea_orm::EntityTrait;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::future::Future;
use tokio_util::sync::CancellationToken;

mod contracts;
mod execution;
mod inputs;
mod media_import;
mod persistence;
mod preview;

pub use contracts::{CanvasNodeContract, CanvasPortContract, GetCanvasNodeContractsInput};
pub use execution::{resume_canvas_graph_run, run_canvas_graph, run_canvas_node};
pub use media_import::import_canvas_media;
pub use preview::{
    CanvasNodeRequestPreview, PreviewCanvasNodeRequestInput, preview_canvas_node_request,
};

use contracts::canvas_node_contracts;
use execution::{finish_canvas_run_failed, record_canvas_node_checkpoint};
use inputs::{
    CanvasLlmTransformInput, CanvasOutputValue, build_canvas_assemble_input_for_run,
    build_canvas_export_video_input_for_run, build_canvas_extract_frame_input_for_run,
    build_canvas_image_input_for_run, build_canvas_llm_transform_input_for_run,
    build_canvas_output_value, build_canvas_prompt_artifact_input_for_run,
    build_canvas_trim_input_for_run, build_canvas_video_input_for_run, first_canvas_text_asset_id,
};
use persistence::{
    FreeformCanvasSpecInput, ensure_canvas_image_media_asset, ensure_canvas_video_media_asset,
    mark_canvas_generation_spec_status, sync_canvas_assembly_contract, update_canvas_node_artifact,
    update_canvas_node_observation, upsert_canvas_prompt_artifact,
    upsert_freeform_canvas_generation_spec,
};

const WORKFLOW_KIND_VIDEO_CANVAS_GRAPH: &str = "video_canvas_graph";
const FREEFORM_GENERATION_CONTEXT: &str = "freeform";
const CANVAS_RENDERER_VERSION: &str = "canvas_workflow_v1";

struct CanvasLlmTextCheckpoint {
    response_content: String,
    parsed_text: String,
    usage: Option<UsageInfo>,
    finish_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GetCanvasGraphInput {
    pub project_id: i64,
    pub graph_id: Option<i64>,
}

fn build_canvas_llm_transform_request(
    project_id: i64,
    graph_id: i64,
    node: &canvas_node::Model,
    llm_input: &CanvasLlmTransformInput,
    workflow_run_id: i64,
    step_key: &str,
    mode: &str,
) -> Result<ChatRequest, String> {
    let node_config_hash = source_content_sha256(
        &serde_json::to_string(&node.config_json)
            .map_err(|err| format!("LlmTextNode 节点配置序列化失败: {err}"))?,
    );
    let source_context_hash = source_content_sha256(&llm_input.source_context);
    let node_config_pack = json!({
        "node_kind": node.node_kind,
        "node_id": node.id,
        "output_kind": llm_input.output_kind,
        "instruction": llm_input.instruction,
        "config_id": llm_input.config_id,
        "node_config_sha256": node_config_hash,
    });
    let mut ledger = LlmContextLedger::new();
    ledger
        .add_pack(a4n_services::llm_context::LlmContextLedgerPackInput {
            scope_kind: (WORKFLOW_KIND_VIDEO_CANVAS_GRAPH).into(),
            pack_kind: ("canvas_node_config:LlmTextNode").into(),
            source_refs_json: Some(json!({
                "graph_id": graph_id,
                "node_id": node.id,
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
        .map_err(|err| err.to_string())?;
    let dynamic_payload = json!({
        "context_refs": {
            "node_config": "稳定上下文包 canvas_node_config:LlmTextNode",
        },
        "run": {
            "mode": mode,
            "graph_id": graph_id,
            "node_id": node.id,
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
            ]
        }
    });
    let dynamic_user_content = format!(
        "请执行当前画布 LLM 文本节点。只返回 output_contract 要求的纯文本。\n\n输入：\n{}",
        serde_json::to_string_pretty(&dynamic_payload)
            .map_err(|err| format!("LlmTextNode 动态输入序列化失败: {err}"))?
    );
    let manifest = ledger.into_manifest(
        dynamic_user_content,
        Some("画布 LLM 节点必须只处理当前节点职责：按稳定节点配置转换上游文本，不存储产物、不访问未连接节点。".to_string()),
    );
    let fingerprint = source_content_sha256(
        &serde_json::to_string(&json!({
            "graph_id": graph_id,
            "node_id": node.id,
            "mode": mode,
            "node_config_sha256": node_config_hash,
            "source_context_sha256": source_context_hash,
            "output_kind": llm_input.output_kind,
        }))
        .map_err(|err| format!("LlmTextNode fingerprint 序列化失败: {err}"))?,
    );
    Ok(context_aware_system_user_request(
        llm_input.system_prompt.clone(),
        manifest,
        llm_input.temperature,
        llm_input.max_tokens,
        None,
    )
    .map_err(|err| err.to_string())?
    .with_trace_context(LlmTraceContext {
        project_id: Some(project_id),
        root_project_id: Some(project_id),
        workflow: WORKFLOW_KIND_VIDEO_CANVAS_GRAPH.to_string(),
        step_key: step_key.to_string(),
        generation_run_id: None,
        workflow_run_id: Some(workflow_run_id),
        call_key: Some(step_key.to_string()),
        input_fingerprint_sha256: Some(fingerprint),
        context_plan_json: Some(json!({
            "canvas_graph_id": graph_id,
            "canvas_node_id": node.id,
            "output_kind": llm_input.output_kind,
            "config_id": llm_input.config_id,
            "node_config_sha256": node_config_hash,
            "source_context_sha256": source_context_hash,
        })),
        context_pack_refs_json: None,
    }))
}

async fn load_canvas_llm_text_checkpoint(
    state: &AppState,
    workflow_run_id: i64,
    call_key: &str,
    input_fingerprint_json: &Value,
) -> Result<Option<CanvasLlmTextCheckpoint>, String> {
    let Some(checkpoint) = load_completed_llm_call_checkpoint(
        state.db.clone(),
        workflow_run_id,
        call_key,
        input_fingerprint_json,
    )
    .await
    .map_err(|err| err.to_string())?
    else {
        return Ok(None);
    };
    let checkpoint_json =
        required_llm_call_checkpoint_json(checkpoint, call_key).map_err(|err| err.to_string())?;
    let response_content = checkpoint_json
        .get("response_content")
        .and_then(Value::as_str)
        .map(ToString::to_string)
        .ok_or_else(|| format!("{call_key}.response_content 必须是字符串"))?;
    let parsed_output = checkpoint_json
        .get("parsed_output")
        .ok_or_else(|| format!("{call_key}.parsed_output 缺失"))?;
    let parsed_text = parsed_output
        .get("text")
        .and_then(Value::as_str)
        .map(ToString::to_string)
        .ok_or_else(|| format!("{call_key}.parsed_output.text 必须是字符串"))?;
    let usage = checkpoint_json
        .get("usage")
        .filter(|value| !value.is_null())
        .cloned()
        .map(serde_json::from_value)
        .transpose()
        .map_err(|err| format!("{call_key}.usage 反序列化失败: {err}"))?;
    let finish_reason = match checkpoint_json.get("finish_reason") {
        Some(Value::Null) | None => None,
        Some(value) => Some(
            value
                .as_str()
                .ok_or_else(|| format!("{call_key}.finish_reason 必须是字符串或 null"))?
                .to_string(),
        ),
    };
    if is_truncated_finish_reason(finish_reason.as_deref()) {
        return Err(format!(
            "{call_key} checkpoint 标记为截断响应，不能用于恢复"
        ));
    }
    Ok(Some(CanvasLlmTextCheckpoint {
        response_content,
        parsed_text,
        usage,
        finish_reason,
    }))
}

struct SaveCanvasLlmTextCheckpointInput<'a> {
    workflow_run_id: i64,
    call_key: String,
    call_order: i32,
    input_fingerprint_json: Value,
    response_content: &'a str,
    transformed_text: &'a str,
    usage: Option<&'a UsageInfo>,
    finish_reason: Option<&'a str>,
}

async fn save_canvas_llm_text_checkpoint(
    state: &AppState,
    input: SaveCanvasLlmTextCheckpointInput<'_>,
) -> Result<(), String> {
    let usage_json = input
        .usage
        .map(serde_json::to_value)
        .transpose()
        .map_err(|err| format!("LlmTextNode 用量序列化失败: {err}"))?;
    save_completed_llm_call_checkpoint(
        state.db.clone(),
        CompletedLlmCallCheckpointInput {
            workflow_run_id: input.workflow_run_id,
            call_key: input.call_key,
            call_order: input.call_order,
            input_fingerprint_json: input.input_fingerprint_json,
            checkpoint_json: json!({
                "output": {
                    "text": input.transformed_text,
                },
                "parsed_output": {
                    "text": input.transformed_text,
                },
                "response_content": input.response_content,
                "usage": usage_json,
                "finish_reason": input.finish_reason,
            }),
            artifacts_json: None,
        },
    )
    .await
    .map_err(|err| err.to_string())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SaveCanvasGraphInput {
    pub project_id: i64,
    pub graph: CanvasGraphInput,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SaveCanvasTextAssetInput {
    pub project_id: i64,
    pub text_asset: UpsertCanvasTextAssetInput,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RunCanvasGraphInput {
    pub project_id: i64,
    pub graph_id: i64,
    #[serde(default)]
    pub target_node_id: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RunCanvasNodeInput {
    pub project_id: i64,
    pub graph_id: i64,
    pub node_id: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ImportCanvasMediaInput {
    pub project_id: i64,
    pub media_kind: String,
    #[serde(default)]
    pub file_path: Option<String>,
    #[serde(default)]
    pub file_name: Option<String>,
    #[serde(default)]
    pub mime_type: Option<String>,
    #[serde(default)]
    pub data_base64: Option<String>,
    #[serde(default)]
    pub prompt: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "media_kind", rename_all = "snake_case")]
pub enum ImportedCanvasMedia {
    Image {
        image: GeneratedImageSummary,
        asset: media_asset::Model,
    },
    Video {
        video: GeneratedVideoSummary,
        asset: media_asset::Model,
    },
    Audio {
        asset: media_asset::Model,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CanvasRunSummary {
    pub workflow_run_id: i64,
    pub project_id: i64,
    pub graph_id: i64,
    pub mode: String,
    pub status: String,
    pub error_message: Option<String>,
}

#[derive(Debug, Clone, Copy)]
struct CanvasRunHandle {
    workflow_run_id: i64,
    finalize_on_success: bool,
}

pub async fn get_canvas_graph(
    state: &AppState,
    user_id: i64,
    input: GetCanvasGraphInput,
) -> Result<CanvasGraphDocument, String> {
    access::verify_project_access_string(state, user_id, input.project_id).await?;
    state
        .canvas
        .get_or_create_graph(input.project_id, input.graph_id)
        .await
        .map_err(|e| e.to_string())
}

pub async fn get_canvas_node_contracts(
    _state: &AppState,
    _user_id: i64,
    _input: GetCanvasNodeContractsInput,
) -> Result<Vec<CanvasNodeContract>, String> {
    Ok(canvas_node_contracts())
}

pub async fn save_canvas_graph(
    state: &AppState,
    user_id: i64,
    input: SaveCanvasGraphInput,
) -> Result<CanvasGraphDocument, String> {
    access::verify_project_editor_string(state, user_id, input.project_id).await?;
    state
        .canvas
        .save_graph(input.project_id, input.graph)
        .await
        .map_err(|e| e.to_string())
}

pub async fn save_canvas_text_asset(
    state: &AppState,
    user_id: i64,
    input: SaveCanvasTextAssetInput,
) -> Result<canvas_text_asset::Model, String> {
    access::verify_project_editor_string(state, user_id, input.project_id).await?;
    state
        .canvas
        .upsert_text_asset(input.project_id, input.text_asset)
        .await
        .map_err(|e| e.to_string())
}

async fn start_canvas_node_run(
    state: &AppState,
    project_id: i64,
    graph_id: i64,
    mode: &str,
    step_key: &str,
    input_json: Value,
    shared_run: Option<CanvasRunHandle>,
) -> Result<CanvasRunHandle, String> {
    if let Some(run) = shared_run {
        record_canvas_node_checkpoint(
            state,
            run.workflow_run_id,
            step_key.to_string(),
            WORKFLOW_STATUS_RUNNING,
            input_json,
            None,
            None,
            false,
        )
        .await?;
        return Ok(run);
    }
    let run = state
        .workflow_run
        .upsert_run(UpsertWorkflowRunInput {
            business_key: format!("video_canvas_graph:{graph_id}:{mode}:{step_key}"),
            workflow_kind: WORKFLOW_KIND_VIDEO_CANVAS_GRAPH.to_string(),
            project_id: Some(project_id),
            root_project_id: Some(project_id),
            source_snapshot_id: None,
            scope_table: Some("canvas_graph".to_string()),
            scope_id: Some(graph_id),
            chapter_id: None,
            mode: Some(mode.to_string()),
            input_fingerprint_json: Some(input_json),
            status: WORKFLOW_STATUS_RUNNING.to_string(),
            current_step_key: Some(step_key.to_string()),
            active_task_id: None,
            can_resume: true,
            resume_block_reason: None,
            error_message: None,
        })
        .await
        .map_err(|e| e.to_string())?;
    Ok(CanvasRunHandle {
        workflow_run_id: run.id,
        finalize_on_success: true,
    })
}

async fn finish_canvas_node_success(
    state: &AppState,
    run: CanvasRunHandle,
    project_id: i64,
    graph_id: i64,
    mode: &str,
) -> Result<CanvasRunSummary, String> {
    if run.finalize_on_success {
        let completed = state
            .workflow_run
            .mark_status(run.workflow_run_id, WORKFLOW_STATUS_COMPLETED, false, None)
            .await
            .map_err(|e| e.to_string())?;
        return Ok(CanvasRunSummary {
            workflow_run_id: completed.id,
            project_id,
            graph_id,
            mode: mode.to_string(),
            status: completed.status,
            error_message: None,
        });
    }
    Ok(CanvasRunSummary {
        workflow_run_id: run.workflow_run_id,
        project_id,
        graph_id,
        mode: mode.to_string(),
        status: WORKFLOW_STATUS_COMPLETED.to_string(),
        error_message: None,
    })
}

async fn finish_canvas_node_cancelled(
    state: &AppState,
    run: CanvasRunHandle,
    project_id: i64,
    graph_id: i64,
    mode: &str,
    step_key: String,
    input_json: Value,
    generation_spec_id: Option<i64>,
) -> Result<CanvasRunSummary, String> {
    if let Some(spec_id) = generation_spec_id {
        mark_canvas_generation_spec_status(state, spec_id, WORKFLOW_STATUS_CANCELLED).await?;
    }
    record_canvas_node_checkpoint(
        state,
        run.workflow_run_id,
        step_key,
        WORKFLOW_STATUS_CANCELLED,
        input_json,
        None,
        Some("用户取消画布运行".to_string()),
        true,
    )
    .await?;
    Ok(CanvasRunSummary {
        workflow_run_id: run.workflow_run_id,
        project_id,
        graph_id,
        mode: mode.to_string(),
        status: WORKFLOW_STATUS_CANCELLED.to_string(),
        error_message: Some("用户取消画布运行".to_string()),
    })
}

async fn run_canvas_assembly_task<F, Fut, T>(
    state: &AppState,
    project_id: i64,
    workflow_run_id: i64,
    operation: F,
) -> Result<T, String>
where
    F: FnOnce() -> Fut,
    Fut: Future<Output = Result<T, String>>,
{
    let workflow_context =
        TaskWorkflowContext::new(workflow_run_id).map_err(|err| err.to_string())?;
    state
        .task_queue
        .run_tracked_with_workflow_context(
            QueueTaskType::Assembly,
            TaskPriority::Medium,
            Some(project_id),
            workflow_context,
            || async {
                operation()
                    .await
                    .map_err(|err| AppError::Internal(anyhow::anyhow!(err)))
            },
        )
        .await
        .map_err(|err| err.to_string())
}

async fn run_canvas_render_text_node(
    state: &AppState,
    project_id: i64,
    graph_id: i64,
    node: &canvas_node::Model,
    document: &CanvasGraphDocument,
    run_handle: Option<CanvasRunHandle>,
    mode: &str,
) -> Result<CanvasRunSummary, String> {
    let upstream_run_id = run_handle.map(|run| run.workflow_run_id);
    let rendered =
        build_canvas_prompt_artifact_input_for_run(Some(state), upstream_run_id, node, document)
            .await?;
    let step_key = format!("canvas_node:{}", node.id);
    let input_json = json!({
        "project_id": project_id,
        "graph_id": graph_id,
        "node_id": node.id,
        "prompt_kind": rendered.prompt_kind,
        "prompt_text": rendered.prompt_text,
        "canvas_text_asset_id": rendered.canvas_text_asset_id,
        "source_node_ids": rendered.source_node_ids,
    });
    let run = start_canvas_node_run(
        state,
        project_id,
        graph_id,
        mode,
        &step_key,
        input_json.clone(),
        run_handle,
    )
    .await?;

    let spec = upsert_freeform_canvas_generation_spec(
        state,
        FreeformCanvasSpecInput {
            project_id,
            graph_id,
            node_id: node.id,
            canvas_text_asset_id: rendered.canvas_text_asset_id,
            target_kind: "canvas_prompt",
            target_entity_kind: "canvas_node",
            status: WORKFLOW_STATUS_COMPLETED,
            source_material_json: json!({
                "prompt_text": rendered.prompt_text,
                "source_node_ids": rendered.source_node_ids,
            }),
            params_json: json!({
                "workflow": "render_prompt",
                "prompt_kind": rendered.prompt_kind,
                "node_config": node.config_json,
            }),
        },
    )
    .await?;
    let spec = spec.ok_or_else(|| "RenderTextNode 当前只支持 freeform 视频项目".to_string())?;
    let artifact =
        upsert_canvas_prompt_artifact(state, spec.id, &rendered.prompt_kind, &rendered.prompt_text)
            .await?;
    record_canvas_node_checkpoint(
        state,
        run.workflow_run_id,
        step_key,
        WORKFLOW_STATUS_COMPLETED,
        input_json,
        Some(json!({
            "data_type": "text",
            "text": artifact.prompt_text,
            "generation_spec_id": spec.id,
            "prompt_artifact_id": artifact.id,
            "prompt_kind": artifact.prompt_kind,
        })),
        None,
        true,
    )
    .await?;
    finish_canvas_node_success(state, run, project_id, graph_id, mode).await
}

async fn run_canvas_llm_text_node(
    state: &AppState,
    project_id: i64,
    graph_id: i64,
    node: &canvas_node::Model,
    document: &CanvasGraphDocument,
    run_handle: Option<CanvasRunHandle>,
    mode: &str,
) -> Result<CanvasRunSummary, String> {
    let upstream_run_id = run_handle.map(|run| run.workflow_run_id);
    let llm_input =
        build_canvas_llm_transform_input_for_run(Some(state), upstream_run_id, node, document)
            .await?;
    let step_key = format!("canvas_node:{}", node.id);
    let input_json = json!({
        "project_id": project_id,
        "graph_id": graph_id,
        "node_id": node.id,
        "output_kind": llm_input.output_kind,
        "config_id": llm_input.config_id,
        "temperature": llm_input.temperature,
        "max_tokens": llm_input.max_tokens,
        "canvas_text_asset_id": llm_input.canvas_text_asset_id,
        "source_node_ids": llm_input.source_node_ids,
        "messages": [
            { "role": "system", "content": llm_input.system_prompt },
            { "role": "user", "content": llm_input.user_prompt }
        ],
    });
    let run = start_canvas_node_run(
        state,
        project_id,
        graph_id,
        mode,
        &step_key,
        input_json.clone(),
        run_handle,
    )
    .await?;

    let spec = upsert_freeform_canvas_generation_spec(
        state,
        FreeformCanvasSpecInput {
            project_id,
            graph_id,
            node_id: node.id,
            canvas_text_asset_id: llm_input.canvas_text_asset_id,
            target_kind: "canvas_prompt",
            target_entity_kind: "canvas_node",
            status: WORKFLOW_STATUS_RUNNING,
            source_material_json: json!({
                "system_prompt": llm_input.system_prompt,
                "user_prompt": llm_input.user_prompt,
                "source_node_ids": llm_input.source_node_ids,
            }),
            params_json: json!({
                "workflow": "llm_transform",
                "output_kind": llm_input.output_kind,
                "config_id": llm_input.config_id,
                "node_config": node.config_json,
            }),
        },
    )
    .await?;
    let Some(spec) = spec else {
        return finish_canvas_run_failed(
            state,
            run.workflow_run_id,
            project_id,
            graph_id,
            mode,
            step_key,
            input_json,
            "LlmTextNode 当前只支持 freeform 视频项目".to_string(),
        )
        .await;
    };

    let mut request = build_canvas_llm_transform_request(
        project_id,
        graph_id,
        node,
        &llm_input,
        run.workflow_run_id,
        &step_key,
        mode,
    )?;
    let call_key = format!("{step_key}:llm_transform");
    let call_order = i32::try_from(node.id)
        .map_err(|_| format!("画布节点 ID 超出调用级 checkpoint 顺序范围: {}", node.id))?;
    let request_json = context_aware_chat_request_trace_json(&request)
        .map_err(|err| format!("LlmTextNode 请求序列化失败: {err}"))?;
    let input_fingerprint_json = json!({
        "workflow": WORKFLOW_KIND_VIDEO_CANVAS_GRAPH,
        "call_kind": "canvas_llm_transform",
        "project_id": project_id,
        "graph_id": graph_id,
        "node_id": node.id,
        "mode": mode,
        "request": request_json,
    });
    let input_fingerprint_sha256 = llm_call_checkpoint_fingerprint_sha256(&input_fingerprint_json)
        .map_err(|err| format!("LlmTextNode 调用级 fingerprint 生成失败: {err}"))?;
    if let Some(trace_context) = request.trace_context.as_mut() {
        trace_context.call_key = Some(call_key.clone());
        trace_context.input_fingerprint_sha256 = Some(input_fingerprint_sha256);
    }
    let restored_checkpoint = load_canvas_llm_text_checkpoint(
        state,
        run.workflow_run_id,
        &call_key,
        &input_fingerprint_json,
    )
    .await?;
    let (response_content, llm_usage, llm_finish_reason, restored_parsed_text) =
        if let Some(restored) = restored_checkpoint {
            (
                restored.response_content,
                restored.usage,
                restored.finish_reason,
                Some(restored.parsed_text),
            )
        } else {
            let workflow_context =
                TaskWorkflowContext::new(run.workflow_run_id).map_err(|err| err.to_string())?;
            match state
                .task_queue
                .run_tracked_with_workflow_context(
                    QueueTaskType::LlmCall,
                    TaskPriority::Medium,
                    Some(project_id),
                    workflow_context,
                    || async {
                        crate::dynamic_llm::DynamicLlmProvider::new(
                            state.db.clone(),
                            state.task_queue.clone(),
                        )
                        .chat_with_config_id(llm_input.config_id, request)
                        .await
                        .map_err(|err| AppError::LlmError(err.to_string()))
                    },
                )
                .await
            {
                Ok(response) => (
                    response.message.content,
                    response.usage,
                    response.finish_reason,
                    None,
                ),
                Err(err) => {
                    mark_canvas_generation_spec_status(state, spec.id, WORKFLOW_STATUS_FAILED)
                        .await?;
                    return finish_canvas_run_failed(
                        state,
                        run.workflow_run_id,
                        project_id,
                        graph_id,
                        mode,
                        step_key,
                        input_json,
                        format!("LlmTextNode 调用 LLM 失败: {err}"),
                    )
                    .await;
                }
            }
        };
    if is_truncated_finish_reason(llm_finish_reason.as_deref()) {
        mark_canvas_generation_spec_status(state, spec.id, WORKFLOW_STATUS_FAILED).await?;
        return finish_canvas_run_failed(
            state,
            run.workflow_run_id,
            project_id,
            graph_id,
            mode,
            step_key,
            input_json,
            "LlmTextNode 输出达到当前 max_tokens 上限，文本可能被截断；请调大输出档位或减少上游文本后重试"
                .to_string(),
        )
        .await;
    }
    let transformed_text = response_content.trim().to_string();
    if let Some(restored_text) = restored_parsed_text.as_ref()
        && restored_text != &transformed_text
    {
        mark_canvas_generation_spec_status(state, spec.id, WORKFLOW_STATUS_FAILED).await?;
        return finish_canvas_run_failed(
            state,
            run.workflow_run_id,
            project_id,
            graph_id,
            mode,
            step_key,
            input_json,
            format!(
                "{call_key} checkpoint 的 parsed_output 与 response_content 重新解析结果不一致"
            ),
        )
        .await;
    }
    if transformed_text.is_empty() {
        mark_canvas_generation_spec_status(state, spec.id, WORKFLOW_STATUS_FAILED).await?;
        return finish_canvas_run_failed(
            state,
            run.workflow_run_id,
            project_id,
            graph_id,
            mode,
            step_key,
            input_json,
            "LlmTextNode 返回空文本".to_string(),
        )
        .await;
    }
    if restored_parsed_text.is_none()
        && let Err(err) = save_canvas_llm_text_checkpoint(
            state,
            SaveCanvasLlmTextCheckpointInput {
                workflow_run_id: run.workflow_run_id,
                call_key,
                call_order,
                input_fingerprint_json,
                response_content: &response_content,
                transformed_text: &transformed_text,
                usage: llm_usage.as_ref(),
                finish_reason: llm_finish_reason.as_deref(),
            },
        )
        .await
    {
        mark_canvas_generation_spec_status(state, spec.id, WORKFLOW_STATUS_FAILED).await?;
        return finish_canvas_run_failed(
            state,
            run.workflow_run_id,
            project_id,
            graph_id,
            mode,
            step_key,
            input_json,
            format!("LlmTextNode 保存调用级 checkpoint 失败: {err}"),
        )
        .await;
    };
    mark_canvas_generation_spec_status(state, spec.id, WORKFLOW_STATUS_COMPLETED).await?;
    let artifact =
        upsert_canvas_prompt_artifact(state, spec.id, &llm_input.output_kind, &transformed_text)
            .await?;
    record_canvas_node_checkpoint(
        state,
        run.workflow_run_id,
        step_key,
        WORKFLOW_STATUS_COMPLETED,
        input_json,
        Some(json!({
            "data_type": "text",
            "text": artifact.prompt_text,
            "generation_spec_id": spec.id,
            "prompt_artifact_id": artifact.id,
            "prompt_kind": artifact.prompt_kind,
            "llm_finish_reason": llm_finish_reason,
            "llm_usage": llm_usage,
        })),
        None,
        true,
    )
    .await?;
    finish_canvas_node_success(state, run, project_id, graph_id, mode).await
}

async fn run_canvas_generate_image_node(
    state: &AppState,
    user_id: i64,
    project_id: i64,
    graph_id: i64,
    node: &canvas_node::Model,
    document: &CanvasGraphDocument,
    run_handle: Option<CanvasRunHandle>,
    mode: &str,
) -> Result<CanvasRunSummary, String> {
    let upstream_run_id = run_handle.map(|run| run.workflow_run_id);
    let mut image_input = build_canvas_image_input_for_run(
        Some(state),
        upstream_run_id,
        user_id,
        project_id,
        graph_id,
        node,
        document,
    )
    .await?;
    let step_key = format!("canvas_node:{}", node.id);
    let input_json = serde_json::to_value(&image_input).map_err(|err| err.to_string())?;
    let generation_spec = upsert_freeform_canvas_generation_spec(
        state,
        FreeformCanvasSpecInput {
            project_id,
            graph_id,
            node_id: node.id,
            canvas_text_asset_id: first_canvas_text_asset_id(node, document),
            target_kind: "canvas_image",
            target_entity_kind: "canvas_node",
            status: WORKFLOW_STATUS_RUNNING,
            source_material_json: json!({
                "prompt": image_input.prompt,
                "source_node_ids": image_input.source_node_ids,
            }),
            params_json: json!({
                "workflow": "generate_image",
                "config_id": image_input.config_id,
                "node_config": node.config_json,
            }),
        },
    )
    .await?;
    let run = start_canvas_node_run(
        state,
        project_id,
        graph_id,
        mode,
        &step_key,
        input_json.clone(),
        run_handle,
    )
    .await?;

    image_input.workflow_run_id = Some(run.workflow_run_id);

    match generate_canvas_image(state, user_id, image_input, None).await {
        Ok(image) => {
            if let Some(spec) = &generation_spec {
                mark_canvas_generation_spec_status(state, spec.id, WORKFLOW_STATUS_COMPLETED)
                    .await?;
            }
            let media_asset_id = sync_completed_canvas_image_asset(
                state,
                project_id,
                image.id,
                generation_spec.as_ref().map(|spec| spec.id),
                "generated_canvas_image",
            )
            .await?;
            record_canvas_node_checkpoint(
                state,
                run.workflow_run_id,
                step_key,
                WORKFLOW_STATUS_COMPLETED,
                input_json,
                Some(json!({
                    "data_type": "image",
                    "generated_image_id": image.id,
                    "file_path": image.file_path,
                    "generation_spec_id": generation_spec.as_ref().map(|spec| spec.id),
                    "media_asset_id": media_asset_id,
                })),
                None,
                true,
            )
            .await?;
            finish_canvas_node_success(state, run, project_id, graph_id, mode).await
        }
        Err(error) => {
            if canvas_workflow_run_is_cancelled(state, run.workflow_run_id).await? {
                return finish_canvas_node_cancelled(
                    state,
                    run,
                    project_id,
                    graph_id,
                    mode,
                    step_key,
                    input_json,
                    generation_spec.as_ref().map(|spec| spec.id),
                )
                .await;
            }
            if let Some(spec) = &generation_spec {
                mark_canvas_generation_spec_status(state, spec.id, WORKFLOW_STATUS_FAILED).await?;
            }
            finish_canvas_run_failed(
                state,
                run.workflow_run_id,
                project_id,
                graph_id,
                mode,
                step_key,
                input_json,
                error,
            )
            .await
        }
    }
}

async fn run_canvas_trim_video_node(
    state: &AppState,
    user_id: i64,
    project_id: i64,
    graph_id: i64,
    node: &canvas_node::Model,
    document: &CanvasGraphDocument,
    run_handle: Option<CanvasRunHandle>,
    mode: &str,
) -> Result<CanvasRunSummary, String> {
    let upstream_run_id = run_handle.map(|run| run.workflow_run_id);
    let (video_id, start_secs, end_secs, source_node_ids) =
        build_canvas_trim_input_for_run(Some(state), upstream_run_id, node, document).await?;
    let step_key = format!("canvas_node:{}", node.id);
    let input_json = json!({
        "project_id": project_id,
        "graph_id": graph_id,
        "node_id": node.id,
        "video_id": video_id,
        "start_secs": start_secs,
        "end_secs": end_secs,
        "source_node_ids": source_node_ids,
    });
    let generation_spec = upsert_freeform_canvas_generation_spec(
        state,
        FreeformCanvasSpecInput {
            project_id,
            graph_id,
            node_id: node.id,
            canvas_text_asset_id: first_canvas_text_asset_id(node, document),
            target_kind: "canvas_video",
            target_entity_kind: "canvas_node",
            status: WORKFLOW_STATUS_RUNNING,
            source_material_json: json!({
                "video_id": video_id,
                "start_secs": start_secs,
                "end_secs": end_secs,
                "source_node_ids": source_node_ids,
            }),
            params_json: json!({
                "workflow": "trim_video",
                "node_config": node.config_json,
            }),
        },
    )
    .await?;
    let run = start_canvas_node_run(
        state,
        project_id,
        graph_id,
        mode,
        &step_key,
        input_json.clone(),
        run_handle,
    )
    .await?;

    match run_canvas_assembly_task(state, project_id, run.workflow_run_id, || async {
        trim_video(state, user_id, video_id, start_secs, end_secs).await
    })
    .await
    {
        Ok(video) => {
            if let Some(spec) = &generation_spec {
                mark_canvas_generation_spec_status(state, spec.id, WORKFLOW_STATUS_COMPLETED)
                    .await?;
            }
            let media_asset_id = sync_completed_canvas_video_asset(
                state,
                project_id,
                video.id,
                generation_spec.as_ref().map(|spec| spec.id),
                "canvas_trim",
            )
            .await?;
            record_canvas_node_checkpoint(
                state,
                run.workflow_run_id,
                step_key,
                WORKFLOW_STATUS_COMPLETED,
                input_json,
                Some(json!({
                    "data_type": "video",
                    "generated_video_id": video.id,
                    "file_path": video.file_path,
                    "thumbnail_path": video.thumbnail_path,
                    "generation_spec_id": generation_spec.as_ref().map(|spec| spec.id),
                    "media_asset_id": media_asset_id,
                })),
                None,
                true,
            )
            .await?;
            finish_canvas_node_success(state, run, project_id, graph_id, mode).await
        }
        Err(error) => {
            if canvas_workflow_run_is_cancelled(state, run.workflow_run_id).await? {
                return finish_canvas_node_cancelled(
                    state,
                    run,
                    project_id,
                    graph_id,
                    mode,
                    step_key,
                    input_json,
                    generation_spec.as_ref().map(|spec| spec.id),
                )
                .await;
            }
            if let Some(spec) = &generation_spec {
                mark_canvas_generation_spec_status(state, spec.id, WORKFLOW_STATUS_FAILED).await?;
            }
            finish_canvas_run_failed(
                state,
                run.workflow_run_id,
                project_id,
                graph_id,
                mode,
                step_key,
                input_json,
                error,
            )
            .await
        }
    }
}

async fn run_canvas_generate_video_node(
    state: &AppState,
    user_id: i64,
    project_id: i64,
    graph_id: i64,
    node: &canvas_node::Model,
    document: &CanvasGraphDocument,
    run_handle: Option<CanvasRunHandle>,
    mode: &str,
) -> Result<CanvasRunSummary, String> {
    let upstream_run_id = run_handle.map(|run| run.workflow_run_id);
    let mut video_input = build_canvas_video_input_for_run(
        Some(state),
        upstream_run_id,
        user_id,
        project_id,
        graph_id,
        node,
        document,
    )
    .await?;
    let step_key = format!("canvas_node:{}", node.id);
    let run_input_json = serde_json::to_value(&video_input).map_err(|err| err.to_string())?;
    let generation_spec = upsert_freeform_canvas_generation_spec(
        state,
        FreeformCanvasSpecInput {
            project_id,
            graph_id,
            node_id: node.id,
            canvas_text_asset_id: first_canvas_text_asset_id(node, document),
            target_kind: "canvas_video",
            target_entity_kind: "canvas_node",
            status: WORKFLOW_STATUS_RUNNING,
            source_material_json: json!({
                "prompt": video_input.prompt,
                "first_frame_image_id": video_input.first_frame_image_id,
                "reference_image_ids": video_input.reference_image_ids,
                "source_node_ids": video_input.source_node_ids,
            }),
            params_json: json!({
                "workflow": "generate_video",
                "config_id": video_input.config_id,
                "width": video_input.width,
                "height": video_input.height,
                "duration_secs": video_input.duration_secs,
                "fps": video_input.fps,
                "node_config": node.config_json,
            }),
        },
    )
    .await?;
    let run = start_canvas_node_run(
        state,
        project_id,
        graph_id,
        mode,
        &step_key,
        run_input_json,
        run_handle,
    )
    .await?;
    video_input.workflow_run_id = Some(run.workflow_run_id);
    let input_json = serde_json::to_value(&video_input).map_err(|err| err.to_string())?;

    match generate_canvas_video(state, video_input, None, CancellationToken::new()).await {
        Ok(video) if video.status == WORKFLOW_STATUS_FAILED => {
            if let Some(spec) = &generation_spec {
                mark_canvas_generation_spec_status(state, spec.id, WORKFLOW_STATUS_FAILED).await?;
            }
            let error = video
                .error_message
                .clone()
                .unwrap_or_else(|| "画布视频生成失败".to_string());
            record_canvas_node_checkpoint(
                state,
                run.workflow_run_id,
                step_key,
                WORKFLOW_STATUS_FAILED,
                input_json,
                Some(json!({
                    "generated_video_id": video.id,
                    "status": video.status,
                    "generation_spec_id": generation_spec.as_ref().map(|spec| spec.id),
                })),
                Some(error.clone()),
                true,
            )
            .await?;
            let failed = state
                .workflow_run
                .mark_status(
                    run.workflow_run_id,
                    WORKFLOW_STATUS_FAILED,
                    true,
                    Some(error.as_str()),
                )
                .await
                .map_err(|e| e.to_string())?;
            Ok(CanvasRunSummary {
                workflow_run_id: failed.id,
                project_id,
                graph_id,
                mode: mode.to_string(),
                status: failed.status,
                error_message: Some(error),
            })
        }
        Ok(video) if video.status == WORKFLOW_STATUS_RUNNING => {
            record_canvas_node_checkpoint(
                state,
                run.workflow_run_id,
                step_key,
                WORKFLOW_STATUS_RUNNING,
                input_json,
                Some(json!({
                    "generated_video_id": video.id,
                    "status": video.status,
                    "provider_job_id": video.provider_job_id,
                    "generation_spec_id": generation_spec.as_ref().map(|spec| spec.id),
                })),
                None,
                false,
            )
            .await?;
            Ok(CanvasRunSummary {
                workflow_run_id: run.workflow_run_id,
                project_id,
                graph_id,
                mode: mode.to_string(),
                status: WORKFLOW_STATUS_RUNNING.to_string(),
                error_message: None,
            })
        }
        Ok(video) => {
            if let Some(spec) = &generation_spec {
                mark_canvas_generation_spec_status(state, spec.id, WORKFLOW_STATUS_COMPLETED)
                    .await?;
            }
            let media_asset_id = sync_completed_canvas_video_asset(
                state,
                project_id,
                video.id,
                generation_spec.as_ref().map(|spec| spec.id),
                "generated_canvas_video",
            )
            .await?;
            record_canvas_node_checkpoint(
                state,
                run.workflow_run_id,
                step_key,
                WORKFLOW_STATUS_COMPLETED,
                input_json,
                Some(json!({
                    "data_type": "video",
                    "generated_video_id": video.id,
                    "status": video.status,
                    "file_path": video.file_path,
                    "thumbnail_path": video.thumbnail_path,
                    "generation_spec_id": generation_spec.as_ref().map(|spec| spec.id),
                    "media_asset_id": media_asset_id,
                })),
                None,
                true,
            )
            .await?;
            finish_canvas_node_success(state, run, project_id, graph_id, mode).await
        }
        Err(error) => {
            if canvas_workflow_run_is_cancelled(state, run.workflow_run_id).await? {
                return finish_canvas_node_cancelled(
                    state,
                    run,
                    project_id,
                    graph_id,
                    mode,
                    step_key,
                    input_json,
                    generation_spec.as_ref().map(|spec| spec.id),
                )
                .await;
            }
            if let Some(spec) = &generation_spec {
                mark_canvas_generation_spec_status(state, spec.id, WORKFLOW_STATUS_FAILED).await?;
            }
            record_canvas_node_checkpoint(
                state,
                run.workflow_run_id,
                step_key,
                WORKFLOW_STATUS_FAILED,
                input_json,
                None,
                Some(error.clone()),
                true,
            )
            .await?;
            let failed = state
                .workflow_run
                .mark_status(
                    run.workflow_run_id,
                    WORKFLOW_STATUS_FAILED,
                    true,
                    Some(error.as_str()),
                )
                .await
                .map_err(|e| e.to_string())?;
            Ok(CanvasRunSummary {
                workflow_run_id: failed.id,
                project_id,
                graph_id,
                mode: mode.to_string(),
                status: failed.status,
                error_message: Some(error),
            })
        }
    }
}

async fn canvas_workflow_run_is_cancelled(
    state: &AppState,
    workflow_run_id: i64,
) -> Result<bool, String> {
    let run = state
        .workflow_run
        .get_run(workflow_run_id)
        .await
        .map_err(|err| err.to_string())?;
    Ok(run.status == WORKFLOW_STATUS_CANCELLED)
}

async fn run_canvas_extract_frame_node(
    state: &AppState,
    user_id: i64,
    project_id: i64,
    graph_id: i64,
    node: &canvas_node::Model,
    document: &CanvasGraphDocument,
    run_handle: Option<CanvasRunHandle>,
    mode: &str,
) -> Result<CanvasRunSummary, String> {
    let upstream_run_id = run_handle.map(|run| run.workflow_run_id);
    let (video_id, frame_kind, at_secs, source_node_ids) =
        build_canvas_extract_frame_input_for_run(Some(state), upstream_run_id, node, document)
            .await?;
    let step_key = format!("canvas_node:{}", node.id);
    let input_json = json!({
        "project_id": project_id,
        "graph_id": graph_id,
        "node_id": node.id,
        "video_id": video_id,
        "frame_kind": frame_kind,
        "at_secs": at_secs,
        "source_node_ids": source_node_ids,
    });
    let generation_spec = upsert_freeform_canvas_generation_spec(
        state,
        FreeformCanvasSpecInput {
            project_id,
            graph_id,
            node_id: node.id,
            canvas_text_asset_id: first_canvas_text_asset_id(node, document),
            target_kind: "canvas_image",
            target_entity_kind: "canvas_node",
            status: WORKFLOW_STATUS_RUNNING,
            source_material_json: json!({
                "video_id": video_id,
                "frame_kind": frame_kind,
                "at_secs": at_secs,
                "source_node_ids": source_node_ids,
            }),
            params_json: json!({
                "workflow": "extract_frame",
                "node_config": node.config_json,
            }),
        },
    )
    .await?;
    let run = start_canvas_node_run(
        state,
        project_id,
        graph_id,
        mode,
        &step_key,
        input_json.clone(),
        run_handle,
    )
    .await?;

    match run_canvas_assembly_task(state, project_id, run.workflow_run_id, || async {
        extract_canvas_video_frame(state, user_id, video_id, frame_kind, at_secs).await
    })
    .await
    {
        Ok(image) => {
            if let Some(spec) = &generation_spec {
                mark_canvas_generation_spec_status(state, spec.id, WORKFLOW_STATUS_COMPLETED)
                    .await?;
            }
            let media_asset_id = sync_completed_canvas_image_asset(
                state,
                project_id,
                image.id,
                generation_spec.as_ref().map(|spec| spec.id),
                "canvas_extracted_frame",
            )
            .await?;
            record_canvas_node_checkpoint(
                state,
                run.workflow_run_id,
                step_key,
                WORKFLOW_STATUS_COMPLETED,
                input_json,
                Some(json!({
                    "data_type": "image",
                    "generated_image_id": image.id,
                    "file_path": image.file_path,
                    "generation_spec_id": generation_spec.as_ref().map(|spec| spec.id),
                    "media_asset_id": media_asset_id,
                })),
                None,
                true,
            )
            .await?;
            finish_canvas_node_success(state, run, project_id, graph_id, mode).await
        }
        Err(error) => {
            if let Some(spec) = &generation_spec {
                mark_canvas_generation_spec_status(state, spec.id, WORKFLOW_STATUS_FAILED).await?;
            }
            finish_canvas_run_failed(
                state,
                run.workflow_run_id,
                project_id,
                graph_id,
                mode,
                step_key,
                input_json,
                error,
            )
            .await
        }
    }
}

async fn run_canvas_assemble_node(
    state: &AppState,
    user_id: i64,
    project_id: i64,
    graph_id: i64,
    node: &canvas_node::Model,
    document: &CanvasGraphDocument,
    run_handle: Option<CanvasRunHandle>,
    mode: &str,
) -> Result<CanvasRunSummary, String> {
    let upstream_run_id = run_handle.map(|run| run.workflow_run_id);
    let (video_ids, title, source_node_ids) =
        build_canvas_assemble_input_for_run(Some(state), upstream_run_id, node, document).await?;
    let step_key = format!("canvas_node:{}", node.id);
    let input_json = json!({
        "project_id": project_id,
        "graph_id": graph_id,
        "node_id": node.id,
        "video_ids": video_ids,
        "title": title,
        "source_node_ids": source_node_ids,
    });
    let generation_spec = upsert_freeform_canvas_generation_spec(
        state,
        FreeformCanvasSpecInput {
            project_id,
            graph_id,
            node_id: node.id,
            canvas_text_asset_id: first_canvas_text_asset_id(node, document),
            target_kind: "canvas_assembly",
            target_entity_kind: "canvas_node",
            status: WORKFLOW_STATUS_RUNNING,
            source_material_json: json!({
                "video_ids": video_ids,
                "title": title,
                "source_node_ids": source_node_ids,
            }),
            params_json: json!({
                "workflow": "assemble",
                "node_config": node.config_json,
            }),
        },
    )
    .await?;
    let run = start_canvas_node_run(
        state,
        project_id,
        graph_id,
        mode,
        &step_key,
        input_json.clone(),
        run_handle,
    )
    .await?;

    match run_canvas_assembly_task(state, project_id, run.workflow_run_id, || async {
        assemble_canvas_videos(state, user_id, project_id, video_ids.clone(), title).await
    })
    .await
    {
        Ok(video) => {
            if let Some(spec) = &generation_spec {
                mark_canvas_generation_spec_status(state, spec.id, WORKFLOW_STATUS_COMPLETED)
                    .await?;
            }
            let assembly_contract = sync_canvas_assembly_contract(
                state,
                project_id,
                graph_id,
                node.id,
                &video,
                &video_ids,
                generation_spec.as_ref().map(|spec| spec.id),
            )
            .await?;
            record_canvas_node_checkpoint(
                state,
                run.workflow_run_id,
                step_key,
                WORKFLOW_STATUS_COMPLETED,
                input_json,
                Some(json!({
                    "data_type": "video",
                    "generated_video_id": video.id,
                    "file_path": video.file_path,
                    "thumbnail_path": video.thumbnail_path,
                    "generation_spec_id": generation_spec.as_ref().map(|spec| spec.id),
                    "media_asset_id": assembly_contract.preview_media_asset_id,
                })),
                None,
                true,
            )
            .await?;
            finish_canvas_node_success(state, run, project_id, graph_id, mode).await
        }
        Err(error) => {
            if let Some(spec) = &generation_spec {
                mark_canvas_generation_spec_status(state, spec.id, WORKFLOW_STATUS_FAILED).await?;
            }
            finish_canvas_run_failed(
                state,
                run.workflow_run_id,
                project_id,
                graph_id,
                mode,
                step_key,
                input_json,
                error,
            )
            .await
        }
    }
}

async fn run_canvas_probe_node(
    state: &AppState,
    project_id: i64,
    graph_id: i64,
    node: &canvas_node::Model,
    document: &CanvasGraphDocument,
    run_handle: Option<CanvasRunHandle>,
    mode: &str,
) -> Result<CanvasRunSummary, String> {
    let data_type = probe_node_data_type(&node.node_kind)?;
    let upstream_run_id = run_handle.map(|run| run.workflow_run_id);
    let output = build_canvas_output_value(
        Some(state),
        upstream_run_id,
        node,
        document,
        data_type,
        &node.node_kind,
    )
    .await?;
    let step_key = format!("canvas_node:{}", node.id);
    let input_json = json!({
        "project_id": project_id,
        "graph_id": graph_id,
        "node_id": node.id,
        "data_type": data_type,
        "source_node_ids": output.source_node_ids,
    });
    let run = start_canvas_node_run(
        state,
        project_id,
        graph_id,
        mode,
        &step_key,
        input_json.clone(),
        run_handle,
    )
    .await?;
    let artifact_json = canvas_probe_artifact_json(state, data_type, &output).await?;
    update_canvas_node_observation(
        state,
        node.id,
        canvas_probe_observation_patch(&artifact_json, &output.source_node_ids),
    )
    .await?;
    record_canvas_node_checkpoint(
        state,
        run.workflow_run_id,
        step_key,
        WORKFLOW_STATUS_COMPLETED,
        input_json,
        Some(artifact_json),
        None,
        true,
    )
    .await?;
    finish_canvas_node_success(state, run, project_id, graph_id, mode).await
}

fn probe_node_data_type(node_kind: &str) -> Result<&'static str, String> {
    match node_kind {
        "TextProbeNode" => Ok("text"),
        "ImageProbeNode" => Ok("image"),
        "VideoProbeNode" => Ok("video"),
        "AudioProbeNode" => Ok("audio"),
        other => Err(format!("探针节点类型无效: {other}")),
    }
}

async fn canvas_probe_artifact_json(
    state: &AppState,
    data_type: &str,
    output: &CanvasOutputValue,
) -> Result<Value, String> {
    match data_type {
        "text" => {
            let text = output
                .text
                .as_deref()
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .ok_or_else(|| "TextProbeNode 缺少文本输入".to_string())?;
            Ok(json!({
                "data_type": "text",
                "text": text,
                "source_node_ids": output.source_node_ids,
            }))
        }
        "image" => {
            if let Some(image_id) = output.generated_image_id {
                let image = generated_image::Entity::find_by_id(image_id)
                    .one(state.db.as_ref())
                    .await
                    .map_err(|err| err.to_string())?
                    .ok_or_else(|| format!("图片探针引用的图片不存在: {image_id}"))?;
                return Ok(json!({
                    "data_type": "image",
                    "generated_image_id": image_id,
                    "media_asset_id": output.media_asset_id,
                    "file_path": image.file_path,
                    "source_node_ids": output.source_node_ids,
                }));
            }
            let media_asset_id = output
                .media_asset_id
                .ok_or_else(|| "ImageProbeNode 缺少图片输入".to_string())?;
            let asset = media_asset::Entity::find_by_id(media_asset_id)
                .one(state.db.as_ref())
                .await
                .map_err(|err| err.to_string())?;
            Ok(json!({
                "data_type": "image",
                "media_asset_id": media_asset_id,
                "file_path": asset.as_ref().and_then(|asset| asset.file_path.clone()),
                "thumbnail_path": asset.as_ref().and_then(|asset| asset.thumbnail_path.clone()),
                "source_node_ids": output.source_node_ids,
            }))
        }
        "video" => {
            if let Some(video_id) = output.generated_video_id {
                let video = generated_video::Entity::find_by_id(video_id)
                    .one(state.db.as_ref())
                    .await
                    .map_err(|err| err.to_string())?
                    .ok_or_else(|| format!("视频探针引用的视频不存在: {video_id}"))?;
                return Ok(json!({
                    "data_type": "video",
                    "generated_video_id": video_id,
                    "media_asset_id": output.media_asset_id,
                    "file_path": video.file_path,
                    "thumbnail_path": video.thumbnail_path,
                    "source_node_ids": output.source_node_ids,
                }));
            }
            let media_asset_id = output
                .media_asset_id
                .ok_or_else(|| "VideoProbeNode 缺少视频输入".to_string())?;
            let asset = media_asset::Entity::find_by_id(media_asset_id)
                .one(state.db.as_ref())
                .await
                .map_err(|err| err.to_string())?;
            Ok(json!({
                "data_type": "video",
                "media_asset_id": media_asset_id,
                "file_path": asset.as_ref().and_then(|asset| asset.file_path.clone()),
                "thumbnail_path": asset.as_ref().and_then(|asset| asset.thumbnail_path.clone()),
                "duration_secs": asset.as_ref().and_then(|asset| asset.duration_secs),
                "source_node_ids": output.source_node_ids,
            }))
        }
        "audio" => {
            let media_asset_id = output
                .media_asset_id
                .ok_or_else(|| "AudioProbeNode 缺少音频输入".to_string())?;
            let asset = media_asset::Entity::find_by_id(media_asset_id)
                .one(state.db.as_ref())
                .await
                .map_err(|err| err.to_string())?;
            Ok(json!({
                "data_type": "audio",
                "media_asset_id": media_asset_id,
                "file_path": asset.as_ref().and_then(|asset| asset.file_path.clone()),
                "duration_secs": asset.as_ref().and_then(|asset| asset.duration_secs),
                "source_node_ids": output.source_node_ids,
            }))
        }
        other => Err(format!("探针数据类型无效: {other}")),
    }
}

fn canvas_probe_observation_patch(artifact_json: &Value, source_node_ids: &[i64]) -> Value {
    json!({
        "observed_data_type": artifact_json.get("data_type").and_then(Value::as_str),
        "observed_text": artifact_json.get("text").and_then(Value::as_str),
        "generated_image_id": artifact_json.get("generated_image_id").and_then(Value::as_i64),
        "generated_video_id": artifact_json.get("generated_video_id").and_then(Value::as_i64),
        "media_asset_id": artifact_json.get("media_asset_id").and_then(Value::as_i64),
        "file_path": artifact_json.get("file_path").and_then(Value::as_str),
        "thumbnail_path": artifact_json.get("thumbnail_path").and_then(Value::as_str),
        "duration_secs": artifact_json.get("duration_secs").and_then(Value::as_f64),
        "source_node_ids": source_node_ids,
    })
}

async fn run_canvas_text_output_node(
    state: &AppState,
    project_id: i64,
    graph_id: i64,
    node: &canvas_node::Model,
    document: &CanvasGraphDocument,
    run_handle: Option<CanvasRunHandle>,
    mode: &str,
) -> Result<CanvasRunSummary, String> {
    let upstream_run_id = run_handle.map(|run| run.workflow_run_id);
    let output = build_canvas_output_value(
        Some(state),
        upstream_run_id,
        node,
        document,
        "text",
        "TextOutputNode",
    )
    .await?;
    let text = output
        .text
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| "TextOutputNode 缺少文本输入".to_string())?;
    let title = canvas_output_title(node, "文本输出");
    let step_key = format!("canvas_node:{}", node.id);
    let input_json = json!({
        "project_id": project_id,
        "graph_id": graph_id,
        "node_id": node.id,
        "source_node_ids": output.source_node_ids,
    });
    let run = start_canvas_node_run(
        state,
        project_id,
        graph_id,
        mode,
        &step_key,
        input_json.clone(),
        run_handle,
    )
    .await?;
    let asset = state
        .canvas
        .upsert_text_asset(
            project_id,
            UpsertCanvasTextAssetInput {
                id: node
                    .ref_type
                    .as_deref()
                    .is_some_and(|value| value == "canvas_text_asset")
                    .then_some(node.ref_id)
                    .flatten(),
                title: title.clone(),
                text_kind: "canvas_output".to_string(),
                content: text.to_string(),
            },
        )
        .await
        .map_err(|err| err.to_string())?;
    update_canvas_node_artifact(
        state,
        node.id,
        "canvas_text_asset",
        asset.id,
        json!({
            "data_type": "text",
            "canvas_text_asset_id": asset.id,
            "title": asset.title,
            "text": asset.content,
        }),
    )
    .await?;
    record_canvas_node_checkpoint(
        state,
        run.workflow_run_id,
        step_key,
        WORKFLOW_STATUS_COMPLETED,
        input_json,
        Some(json!({
            "data_type": "text",
            "canvas_text_asset_id": asset.id,
            "title": asset.title,
            "text": asset.content,
        })),
        None,
        true,
    )
    .await?;
    finish_canvas_node_success(state, run, project_id, graph_id, mode).await
}

async fn run_canvas_image_output_node(
    state: &AppState,
    project_id: i64,
    graph_id: i64,
    node: &canvas_node::Model,
    document: &CanvasGraphDocument,
    run_handle: Option<CanvasRunHandle>,
    mode: &str,
) -> Result<CanvasRunSummary, String> {
    let upstream_run_id = run_handle.map(|run| run.workflow_run_id);
    let output = build_canvas_output_value(
        Some(state),
        upstream_run_id,
        node,
        document,
        "image",
        "ImageOutputNode",
    )
    .await?;
    let step_key = format!("canvas_node:{}", node.id);
    let input_json = json!({
        "project_id": project_id,
        "graph_id": graph_id,
        "node_id": node.id,
        "generated_image_id": output.generated_image_id,
        "media_asset_id": output.media_asset_id,
        "source_node_ids": output.source_node_ids,
    });
    let run = start_canvas_node_run(
        state,
        project_id,
        graph_id,
        mode,
        &step_key,
        input_json.clone(),
        run_handle,
    )
    .await?;
    let (ref_type, ref_id, artifact_json) = if let Some(image_id) = output.generated_image_id {
        let image = generated_image::Entity::find_by_id(image_id)
            .one(state.db.as_ref())
            .await
            .map_err(|err| err.to_string())?
            .ok_or_else(|| format!("图片输出不存在: {image_id}"))?;
        (
            "generated_image",
            image_id,
            json!({
                "data_type": "image",
                "generated_image_id": image_id,
                "file_path": image.file_path,
                "media_asset_id": output.media_asset_id,
            }),
        )
    } else {
        let media_asset_id = output
            .media_asset_id
            .ok_or_else(|| "ImageOutputNode 缺少图片输入".to_string())?;
        let asset = media_asset::Entity::find_by_id(media_asset_id)
            .one(state.db.as_ref())
            .await
            .map_err(|err| err.to_string())?;
        (
            "media_asset",
            media_asset_id,
            json!({
                "data_type": "image",
                "media_asset_id": media_asset_id,
                "file_path": asset.as_ref().and_then(|asset| asset.file_path.clone()),
                "thumbnail_path": asset.as_ref().and_then(|asset| asset.thumbnail_path.clone()),
            }),
        )
    };
    update_canvas_node_artifact(state, node.id, ref_type, ref_id, artifact_json.clone()).await?;
    record_canvas_node_checkpoint(
        state,
        run.workflow_run_id,
        step_key,
        WORKFLOW_STATUS_COMPLETED,
        input_json,
        Some(artifact_json),
        None,
        true,
    )
    .await?;
    finish_canvas_node_success(state, run, project_id, graph_id, mode).await
}

async fn run_canvas_audio_output_node(
    state: &AppState,
    project_id: i64,
    graph_id: i64,
    node: &canvas_node::Model,
    document: &CanvasGraphDocument,
    run_handle: Option<CanvasRunHandle>,
    mode: &str,
) -> Result<CanvasRunSummary, String> {
    let upstream_run_id = run_handle.map(|run| run.workflow_run_id);
    let output = build_canvas_output_value(
        Some(state),
        upstream_run_id,
        node,
        document,
        "audio",
        "AudioOutputNode",
    )
    .await?;
    let media_asset_id = output
        .media_asset_id
        .ok_or_else(|| "AudioOutputNode 缺少音频输入".to_string())?;
    let step_key = format!("canvas_node:{}", node.id);
    let input_json = json!({
        "project_id": project_id,
        "graph_id": graph_id,
        "node_id": node.id,
        "media_asset_id": media_asset_id,
        "source_node_ids": output.source_node_ids,
    });
    let run = start_canvas_node_run(
        state,
        project_id,
        graph_id,
        mode,
        &step_key,
        input_json.clone(),
        run_handle,
    )
    .await?;
    let asset = media_asset::Entity::find_by_id(media_asset_id)
        .one(state.db.as_ref())
        .await
        .map_err(|err| err.to_string())?;
    let artifact_json = json!({
        "data_type": "audio",
        "media_asset_id": media_asset_id,
        "file_path": asset.as_ref().and_then(|asset| asset.file_path.clone()),
        "duration_secs": asset.as_ref().and_then(|asset| asset.duration_secs),
    });
    update_canvas_node_artifact(
        state,
        node.id,
        "media_asset",
        media_asset_id,
        artifact_json.clone(),
    )
    .await?;
    record_canvas_node_checkpoint(
        state,
        run.workflow_run_id,
        step_key,
        WORKFLOW_STATUS_COMPLETED,
        input_json,
        Some(artifact_json),
        None,
        true,
    )
    .await?;
    finish_canvas_node_success(state, run, project_id, graph_id, mode).await
}

async fn run_canvas_export_video_node(
    state: &AppState,
    project_id: i64,
    graph_id: i64,
    node: &canvas_node::Model,
    document: &CanvasGraphDocument,
    run_handle: Option<CanvasRunHandle>,
    mode: &str,
) -> Result<CanvasRunSummary, String> {
    let upstream_run_id = run_handle.map(|run| run.workflow_run_id);
    let export_input =
        build_canvas_export_video_input_for_run(Some(state), upstream_run_id, node, document)
            .await?;
    let step_key = format!("canvas_node:{}", node.id);
    let input_json = json!({
        "project_id": project_id,
        "graph_id": graph_id,
        "node_id": node.id,
        "video_id": export_input.video_id,
        "source_node_ids": export_input.source_node_ids,
    });
    let run = start_canvas_node_run(
        state,
        project_id,
        graph_id,
        mode,
        &step_key,
        input_json.clone(),
        run_handle,
    )
    .await?;

    let video_id = export_input
        .video_id
        .ok_or_else(|| "VideoOutputNode 缺少可输出的视频输入".to_string())?;
    let video = generated_video::Entity::find_by_id(video_id)
        .one(state.db.as_ref())
        .await
        .map_err(|err| err.to_string())?
        .ok_or_else(|| format!("画布输出视频不存在: {video_id}"))?;
    let output_json = json!({
        "data_type": "video",
        "generated_video_id": video_id,
        "file_path": video.file_path,
        "thumbnail_path": video.thumbnail_path,
        "output": {
            "kind": "video",
            "generated_video_id": video_id,
            "file_path": video.file_path,
            "thumbnail_path": video.thumbnail_path
        }
    });
    update_canvas_node_artifact(
        state,
        node.id,
        "generated_video",
        video_id,
        output_json.clone(),
    )
    .await?;

    record_canvas_node_checkpoint(
        state,
        run.workflow_run_id,
        step_key,
        WORKFLOW_STATUS_COMPLETED,
        input_json,
        Some(output_json),
        None,
        true,
    )
    .await?;
    finish_canvas_node_success(state, run, project_id, graph_id, mode).await
}

async fn sync_completed_canvas_image_asset(
    state: &AppState,
    project_id: i64,
    image_id: i64,
    generation_spec_id: Option<i64>,
    origin: &str,
) -> Result<Option<i64>, String> {
    let Some(generation_spec_id) = generation_spec_id else {
        return Ok(None);
    };
    let image = generated_image::Entity::find_by_id(image_id)
        .one(state.db.as_ref())
        .await
        .map_err(|err| err.to_string())?
        .ok_or_else(|| format!("画布图片产物不存在: {image_id}"))?;
    let asset = ensure_canvas_image_media_asset(
        state.db.as_ref(),
        project_id,
        &image,
        Some(generation_spec_id),
        origin,
    )
    .await?;
    Ok(Some(asset.id))
}

fn canvas_output_title(node: &canvas_node::Model, fallback: &str) -> String {
    node.config_json
        .get("title")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or(fallback)
        .to_string()
}

async fn sync_completed_canvas_video_asset(
    state: &AppState,
    project_id: i64,
    video_id: i64,
    generation_spec_id: Option<i64>,
    origin: &str,
) -> Result<Option<i64>, String> {
    let Some(generation_spec_id) = generation_spec_id else {
        return Ok(None);
    };
    let video = generated_video::Entity::find_by_id(video_id)
        .one(state.db.as_ref())
        .await
        .map_err(|err| err.to_string())?
        .ok_or_else(|| format!("画布视频产物不存在: {video_id}"))?;
    let asset = ensure_canvas_video_media_asset(
        state.db.as_ref(),
        project_id,
        &video,
        Some(generation_spec_id),
        origin,
    )
    .await?;
    Ok(Some(asset.id))
}
