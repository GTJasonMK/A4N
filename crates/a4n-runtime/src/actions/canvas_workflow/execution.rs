use super::*;
use a4n_services::workflow::{UpsertWorkflowCheckpointInput, WorkflowProgressEventInput};
use chrono::Utc;

#[derive(Debug, Clone)]
struct PreparedCanvasGraphRun {
    workflow_run_id: i64,
    project_id: i64,
    graph_id: i64,
    mode: String,
    runnable_node_ids: Vec<i64>,
}

pub async fn run_canvas_graph(
    state: &AppState,
    user_id: i64,
    input: RunCanvasGraphInput,
) -> Result<CanvasRunSummary, String> {
    let prepared = prepare_canvas_graph_run(state, user_id, input).await?;
    submit_prepared_canvas_graph_run(state, user_id, prepared)
}

pub async fn run_canvas_node(
    state: &AppState,
    user_id: i64,
    input: RunCanvasNodeInput,
) -> Result<CanvasRunSummary, String> {
    let prepared = prepare_canvas_single_node_run(state, user_id, input).await?;
    submit_prepared_canvas_graph_run(state, user_id, prepared)
}

pub async fn resume_canvas_graph_run(
    state: &AppState,
    user_id: i64,
    workflow_run_id: i64,
) -> Result<CanvasRunSummary, String> {
    let prepared = prepare_canvas_graph_resume(state, user_id, workflow_run_id).await?;
    if prepared.runnable_node_ids.is_empty() {
        return Ok(CanvasRunSummary {
            workflow_run_id: prepared.workflow_run_id,
            project_id: prepared.project_id,
            graph_id: prepared.graph_id,
            mode: prepared.mode,
            status: WORKFLOW_STATUS_COMPLETED.to_string(),
            error_message: None,
        });
    }
    submit_prepared_canvas_graph_run(state, user_id, prepared)
}

fn submit_prepared_canvas_graph_run(
    state: &AppState,
    user_id: i64,
    prepared: PreparedCanvasGraphRun,
) -> Result<CanvasRunSummary, String> {
    let summary = CanvasRunSummary {
        workflow_run_id: prepared.workflow_run_id,
        project_id: prepared.project_id,
        graph_id: prepared.graph_id,
        mode: prepared.mode.clone(),
        status: WORKFLOW_STATUS_RUNNING.to_string(),
        error_message: None,
    };
    let runtime_snapshot = crate::runtime_config::current_snapshot();
    let background_state = state.clone();
    tokio::spawn(async move {
        let workflow_run_id = prepared.workflow_run_id;
        let result = execute_prepared_canvas_graph_run_with_runtime_config(
            &background_state,
            user_id,
            prepared,
            runtime_snapshot,
        )
        .await;
        if let Err(error) = result {
            if canvas_workflow_run_is_cancelled(&background_state, workflow_run_id)
                .await
                .unwrap_or(false)
            {
                tracing::info!(
                    workflow_run_id,
                    error = %error,
                    "画布后台运行在取消后停止"
                );
                return;
            }
            if let Err(mark_error) = background_state
                .workflow_run
                .mark_status(workflow_run_id, WORKFLOW_STATUS_FAILED, true, Some(&error))
                .await
            {
                tracing::error!(
                    workflow_run_id,
                    error = %error,
                    mark_error = %mark_error,
                    "画布后台运行失败，且标记失败状态也失败"
                );
            } else {
                tracing::error!(
                    workflow_run_id,
                    error = %error,
                    "画布后台运行失败"
                );
            }
        }
    });
    Ok(summary)
}

async fn prepare_canvas_graph_resume(
    state: &AppState,
    user_id: i64,
    workflow_run_id: i64,
) -> Result<PreparedCanvasGraphRun, String> {
    let run = state
        .workflow_run
        .get_run(workflow_run_id)
        .await
        .map_err(|err| err.to_string())?;
    if run.workflow_kind != WORKFLOW_KIND_VIDEO_CANVAS_GRAPH {
        return Err(format!(
            "工作流类型 {} 不能按画布运行恢复",
            run.workflow_kind
        ));
    }
    let project_id = run
        .project_id
        .ok_or_else(|| "画布运行缺少 project_id".to_string())?;
    let graph_id = run
        .scope_id
        .filter(|_| run.scope_table.as_deref() == Some("canvas_graph"))
        .ok_or_else(|| "画布运行缺少 canvas_graph scope".to_string())?;
    access::verify_project_editor_string(state, user_id, project_id).await?;

    let target_node_id = run
        .input_fingerprint_json
        .as_ref()
        .and_then(|value| value.get("target_node_id"))
        .and_then(Value::as_i64);
    let mode = run.mode.unwrap_or_else(|| {
        if target_node_id.is_some() {
            "run_to_node".to_string()
        } else {
            "full_graph".to_string()
        }
    });
    let single_node_id = if mode == "single_node" {
        Some(
            run.input_fingerprint_json
                .as_ref()
                .and_then(|value| value.get("node_id"))
                .and_then(Value::as_i64)
                .ok_or_else(|| "单节点运行缺少 node_id".to_string())?,
        )
    } else {
        None
    };
    let document = state
        .canvas
        .get_or_create_graph(project_id, Some(graph_id))
        .await
        .map_err(|e| e.to_string())?;
    validate_canvas_document_edge_contract(&document).map_err(|e| e.to_string())?;
    if let Some(target_node_id) = target_node_id {
        let target = document
            .nodes
            .iter()
            .find(|node| node.id == target_node_id)
            .ok_or_else(|| format!("画布节点不存在: {target_node_id}"))?;
        if !canvas_node_kind_is_output(&target.node_kind) {
            return Err("运行到此节点只能锚定输出节点".to_string());
        }
    }
    if let Some(single_node_id) = single_node_id {
        let node = document
            .nodes
            .iter()
            .find(|node| node.id == single_node_id)
            .ok_or_else(|| format!("画布节点不存在: {single_node_id}"))?;
        if !is_runnable_canvas_node(node) {
            return Err(unsupported_canvas_node_execution_error(&node.node_kind));
        }
    }
    let runnable_node_ids = match single_node_id {
        Some(node_id) => vec![node_id],
        None => runnable_canvas_node_ids(&document, target_node_id)?,
    };
    let completed_step_keys = state
        .workflow_run
        .checkpoints(workflow_run_id)
        .await
        .map_err(|err| err.to_string())?
        .into_iter()
        .filter(|checkpoint| checkpoint.status == WORKFLOW_STATUS_COMPLETED)
        .map(|checkpoint| checkpoint.step_key)
        .collect::<BTreeSet<_>>();
    let pending_node_ids = runnable_node_ids
        .into_iter()
        .filter(|node_id| !completed_step_keys.contains(&format!("canvas_node:{node_id}")))
        .collect::<Vec<_>>();
    if pending_node_ids.is_empty() {
        let completed = state
            .workflow_run
            .mark_status(workflow_run_id, WORKFLOW_STATUS_COMPLETED, false, None)
            .await
            .map_err(|err| err.to_string())?;
        return Ok(PreparedCanvasGraphRun {
            workflow_run_id: completed.id,
            project_id,
            graph_id,
            mode,
            runnable_node_ids: Vec::new(),
        });
    }
    let first_step_key = pending_node_ids
        .first()
        .map(|node_id| format!("canvas_node:{node_id}"));
    state
        .workflow_run
        .mark_status(workflow_run_id, WORKFLOW_STATUS_RUNNING, true, None)
        .await
        .map_err(|err| err.to_string())?;
    if let Some(first_step_key) = first_step_key {
        state
            .workflow_run
            .mark_current_step(workflow_run_id, &first_step_key)
            .await
            .map_err(|err| err.to_string())?;
    }
    Ok(PreparedCanvasGraphRun {
        workflow_run_id,
        project_id,
        graph_id,
        mode,
        runnable_node_ids: pending_node_ids,
    })
}

async fn prepare_canvas_graph_run(
    state: &AppState,
    user_id: i64,
    input: RunCanvasGraphInput,
) -> Result<PreparedCanvasGraphRun, String> {
    access::verify_project_editor_string(state, user_id, input.project_id).await?;
    let document = state
        .canvas
        .get_or_create_graph(input.project_id, Some(input.graph_id))
        .await
        .map_err(|e| e.to_string())?;
    validate_canvas_document_edge_contract(&document).map_err(|e| e.to_string())?;
    if let Some(target_node_id) = input.target_node_id
        && !document.nodes.iter().any(|node| node.id == target_node_id)
    {
        return Err(format!("画布节点不存在: {target_node_id}"));
    }
    if let Some(target_node_id) = input.target_node_id {
        let target = document
            .nodes
            .iter()
            .find(|node| node.id == target_node_id)
            .ok_or_else(|| format!("画布节点不存在: {target_node_id}"))?;
        if !canvas_node_kind_is_output(&target.node_kind) {
            return Err("运行到此节点只能锚定输出节点".to_string());
        }
    }
    let mode = if input.target_node_id.is_some() {
        "run_to_node"
    } else {
        "full_graph"
    };
    let runnable_node_ids = runnable_canvas_node_ids(&document, input.target_node_id)?;
    if runnable_node_ids.is_empty() {
        return Err("画布中没有可执行的操作节点".to_string());
    }
    let graph_input_json = json!({
        "project_id": input.project_id,
        "graph_id": input.graph_id,
        "graph_kind": document.graph.graph_kind,
        "graph_version": document.graph.version,
        "target_node_id": input.target_node_id,
        "runnable_node_ids": runnable_node_ids.clone(),
    });
    let first_step_key = runnable_node_ids
        .first()
        .map(|node_id| format!("canvas_node:{node_id}"));
    let graph_run = state
        .workflow_run
        .upsert_run(UpsertWorkflowRunInput {
            business_key: format!(
                "video_canvas_graph:{}:{mode}:graph:{}",
                input.graph_id,
                Utc::now().timestamp_micros()
            ),
            workflow_kind: WORKFLOW_KIND_VIDEO_CANVAS_GRAPH.to_string(),
            project_id: Some(input.project_id),
            root_project_id: Some(input.project_id),
            source_snapshot_id: None,
            scope_table: Some("canvas_graph".to_string()),
            scope_id: Some(input.graph_id),
            chapter_id: None,
            mode: Some(mode.to_string()),
            input_fingerprint_json: Some(graph_input_json),
            status: WORKFLOW_STATUS_RUNNING.to_string(),
            current_step_key: first_step_key,
            active_task_id: None,
            can_resume: true,
            resume_block_reason: None,
            error_message: None,
        })
        .await
        .map_err(|e| e.to_string())?;
    let shared_run = CanvasRunHandle {
        workflow_run_id: graph_run.id,
        finalize_on_success: false,
    };

    Ok(PreparedCanvasGraphRun {
        workflow_run_id: shared_run.workflow_run_id,
        project_id: input.project_id,
        graph_id: input.graph_id,
        mode: mode.to_string(),
        runnable_node_ids,
    })
}

async fn prepare_canvas_single_node_run(
    state: &AppState,
    user_id: i64,
    input: RunCanvasNodeInput,
) -> Result<PreparedCanvasGraphRun, String> {
    access::verify_project_editor_string(state, user_id, input.project_id).await?;
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
    if !is_runnable_canvas_node(node) {
        return Err(unsupported_canvas_node_execution_error(&node.node_kind));
    }
    let step_key = format!("canvas_node:{}", input.node_id);
    let graph_input_json = json!({
        "project_id": input.project_id,
        "graph_id": input.graph_id,
        "graph_kind": document.graph.graph_kind,
        "graph_version": document.graph.version,
        "node_id": input.node_id,
        "mode": "single_node",
        "runnable_node_ids": [input.node_id],
    });
    let graph_run = state
        .workflow_run
        .upsert_run(UpsertWorkflowRunInput {
            business_key: format!(
                "video_canvas_graph:{}:single_node:{}:{}",
                input.graph_id,
                input.node_id,
                Utc::now().timestamp_micros()
            ),
            workflow_kind: WORKFLOW_KIND_VIDEO_CANVAS_GRAPH.to_string(),
            project_id: Some(input.project_id),
            root_project_id: Some(input.project_id),
            source_snapshot_id: None,
            scope_table: Some("canvas_graph".to_string()),
            scope_id: Some(input.graph_id),
            chapter_id: None,
            mode: Some("single_node".to_string()),
            input_fingerprint_json: Some(graph_input_json),
            status: WORKFLOW_STATUS_RUNNING.to_string(),
            current_step_key: Some(step_key),
            active_task_id: None,
            can_resume: true,
            resume_block_reason: None,
            error_message: None,
        })
        .await
        .map_err(|e| e.to_string())?;
    Ok(PreparedCanvasGraphRun {
        workflow_run_id: graph_run.id,
        project_id: input.project_id,
        graph_id: input.graph_id,
        mode: "single_node".to_string(),
        runnable_node_ids: vec![input.node_id],
    })
}

async fn execute_prepared_canvas_graph_run_with_runtime_config(
    state: &AppState,
    user_id: i64,
    prepared: PreparedCanvasGraphRun,
    runtime_snapshot: Option<crate::runtime_config::RuntimeConfigSnapshot>,
) -> Result<CanvasRunSummary, String> {
    match runtime_snapshot {
        Some(snapshot) => {
            crate::runtime_config::scope(
                snapshot,
                execute_prepared_canvas_graph_run(state, user_id, prepared),
            )
            .await
        }
        None => execute_prepared_canvas_graph_run(state, user_id, prepared).await,
    }
}

async fn execute_prepared_canvas_graph_run(
    state: &AppState,
    user_id: i64,
    prepared: PreparedCanvasGraphRun,
) -> Result<CanvasRunSummary, String> {
    let shared_run = CanvasRunHandle {
        workflow_run_id: prepared.workflow_run_id,
        finalize_on_success: false,
    };
    let mut last_summary = None;
    for node_id in prepared.runnable_node_ids.iter().copied() {
        if canvas_workflow_run_is_cancelled(state, prepared.workflow_run_id).await? {
            return Ok(cancelled_canvas_graph_summary(&prepared));
        }
        let current_document = state
            .canvas
            .get_or_create_graph(prepared.project_id, Some(prepared.graph_id))
            .await
            .map_err(|e| e.to_string())?;
        validate_canvas_document_edge_contract(&current_document).map_err(|e| e.to_string())?;
        let node = current_document
            .nodes
            .iter()
            .find(|node| node.id == node_id)
            .ok_or_else(|| format!("画布节点不存在: {node_id}"))?;
        let summary = run_canvas_operation_node(
            state,
            user_id,
            prepared.project_id,
            prepared.graph_id,
            node,
            &current_document,
            Some(shared_run),
            &prepared.mode,
        )
        .await?;
        let should_stop = summary.status == WORKFLOW_STATUS_FAILED
            || summary.status == WORKFLOW_STATUS_RUNNING
            || summary.status == WORKFLOW_STATUS_CANCELLED;
        last_summary = Some(summary);
        if should_stop {
            break;
        }
    }

    let Some(summary) = last_summary else {
        return Err("画布中没有可执行的操作节点".to_string());
    };
    if canvas_workflow_run_is_cancelled(state, prepared.workflow_run_id).await? {
        return Ok(cancelled_canvas_graph_summary(&prepared));
    }
    if summary.status == WORKFLOW_STATUS_COMPLETED {
        let completed = state
            .workflow_run
            .mark_status(
                prepared.workflow_run_id,
                WORKFLOW_STATUS_COMPLETED,
                false,
                None,
            )
            .await
            .map_err(|e| e.to_string())?;
        return Ok(CanvasRunSummary {
            workflow_run_id: completed.id,
            project_id: prepared.project_id,
            graph_id: prepared.graph_id,
            mode: prepared.mode,
            status: completed.status,
            error_message: None,
        });
    }
    Ok(summary)
}

fn cancelled_canvas_graph_summary(prepared: &PreparedCanvasGraphRun) -> CanvasRunSummary {
    CanvasRunSummary {
        workflow_run_id: prepared.workflow_run_id,
        project_id: prepared.project_id,
        graph_id: prepared.graph_id,
        mode: prepared.mode.clone(),
        status: WORKFLOW_STATUS_CANCELLED.to_string(),
        error_message: Some("用户取消画布运行".to_string()),
    }
}

async fn run_canvas_operation_node(
    state: &AppState,
    user_id: i64,
    project_id: i64,
    graph_id: i64,
    node: &canvas_node::Model,
    document: &CanvasGraphDocument,
    run_handle: Option<CanvasRunHandle>,
    mode: &str,
) -> Result<CanvasRunSummary, String> {
    match node.node_kind.as_str() {
        "RenderTextNode" => {
            run_canvas_render_text_node(
                state, project_id, graph_id, node, document, run_handle, mode,
            )
            .await
        }
        "LlmTextNode" => {
            run_canvas_llm_text_node(
                state, project_id, graph_id, node, document, run_handle, mode,
            )
            .await
        }
        "GenerateImageNode" => {
            run_canvas_generate_image_node(
                state, user_id, project_id, graph_id, node, document, run_handle, mode,
            )
            .await
        }
        "GenerateVideoNode" => {
            run_canvas_generate_video_node(
                state, user_id, project_id, graph_id, node, document, run_handle, mode,
            )
            .await
        }
        "TrimVideoNode" => {
            run_canvas_trim_video_node(
                state, user_id, project_id, graph_id, node, document, run_handle, mode,
            )
            .await
        }
        "ExtractFrameNode" => {
            run_canvas_extract_frame_node(
                state, user_id, project_id, graph_id, node, document, run_handle, mode,
            )
            .await
        }
        "AssembleVideoNode" => {
            run_canvas_assemble_node(
                state, user_id, project_id, graph_id, node, document, run_handle, mode,
            )
            .await
        }
        "TextOutputNode" => {
            run_canvas_text_output_node(
                state, project_id, graph_id, node, document, run_handle, mode,
            )
            .await
        }
        "ImageOutputNode" => {
            run_canvas_image_output_node(
                state, project_id, graph_id, node, document, run_handle, mode,
            )
            .await
        }
        "AudioOutputNode" => {
            run_canvas_audio_output_node(
                state, project_id, graph_id, node, document, run_handle, mode,
            )
            .await
        }
        "VideoOutputNode" => {
            run_canvas_export_video_node(
                state, project_id, graph_id, node, document, run_handle, mode,
            )
            .await
        }
        "TextProbeNode" | "ImageProbeNode" | "VideoProbeNode" | "AudioProbeNode" => {
            run_canvas_probe_node(
                state, project_id, graph_id, node, document, run_handle, mode,
            )
            .await
        }
        other => Err(unsupported_canvas_node_execution_error(other)),
    }
}

pub(super) fn runnable_canvas_node_ids(
    document: &CanvasGraphDocument,
    target_node_id: Option<i64>,
) -> Result<Vec<i64>, String> {
    let order = topological_canvas_node_ids(document)?;
    let allowed = match target_node_id {
        Some(node_id) => upstream_canvas_node_ids(document, node_id)?,
        None => document.nodes.iter().map(|node| node.id).collect(),
    };
    Ok(order
        .into_iter()
        .filter(|node_id| allowed.contains(node_id))
        .filter(|node_id| {
            document
                .nodes
                .iter()
                .find(|node| node.id == *node_id)
                .is_some_and(is_runnable_canvas_node)
        })
        .collect())
}

fn is_runnable_canvas_node(node: &canvas_node::Model) -> bool {
    canvas_node_kind_is_executable(&node.node_kind)
}

fn canvas_node_kind_is_executable(node_kind: &str) -> bool {
    matches!(
        node_kind,
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
            | "TextOutputNode"
            | "ImageOutputNode"
            | "VideoOutputNode"
            | "AudioOutputNode"
    )
}

fn canvas_node_kind_is_output(node_kind: &str) -> bool {
    matches!(
        node_kind,
        "TextOutputNode" | "ImageOutputNode" | "VideoOutputNode" | "AudioOutputNode"
    )
}

pub(super) fn unsupported_canvas_node_execution_error(node_kind: &str) -> String {
    format!("画布节点类型不支持执行: {node_kind}")
}

fn topological_canvas_node_ids(document: &CanvasGraphDocument) -> Result<Vec<i64>, String> {
    let node_ids = document
        .nodes
        .iter()
        .map(|node| node.id)
        .collect::<BTreeSet<_>>();
    let mut indegree = document
        .nodes
        .iter()
        .map(|node| (node.id, 0usize))
        .collect::<BTreeMap<_, _>>();
    let mut outgoing = document
        .nodes
        .iter()
        .map(|node| (node.id, Vec::new()))
        .collect::<BTreeMap<_, _>>();

    for edge in &document.edges {
        if !node_ids.contains(&edge.source_node_id) {
            return Err(format!(
                "画布连线引用不存在的源节点: {}",
                edge.source_node_id
            ));
        }
        if !node_ids.contains(&edge.target_node_id) {
            return Err(format!(
                "画布连线引用不存在的目标节点: {}",
                edge.target_node_id
            ));
        }
        *indegree.entry(edge.target_node_id).or_default() += 1;
        outgoing
            .entry(edge.source_node_id)
            .or_default()
            .push(edge.target_node_id);
    }

    let mut queue = indegree
        .iter()
        .filter_map(|(node_id, degree)| (*degree == 0).then_some(*node_id))
        .collect::<VecDeque<_>>();
    let mut order = Vec::with_capacity(document.nodes.len());
    while let Some(node_id) = queue.pop_front() {
        order.push(node_id);
        for target_id in outgoing.get(&node_id).into_iter().flatten() {
            if let Some(degree) = indegree.get_mut(target_id) {
                *degree -= 1;
                if *degree == 0 {
                    queue.push_back(*target_id);
                }
            }
        }
    }
    if order.len() != document.nodes.len() {
        return Err("画布暂只支持有向无环图，当前连线存在环路".to_string());
    }
    Ok(order)
}

fn upstream_canvas_node_ids(
    document: &CanvasGraphDocument,
    target_node_id: i64,
) -> Result<BTreeSet<i64>, String> {
    if !document.nodes.iter().any(|node| node.id == target_node_id) {
        return Err(format!("画布节点不存在: {target_node_id}"));
    }
    let mut incoming = BTreeMap::<i64, Vec<i64>>::new();
    for edge in &document.edges {
        incoming
            .entry(edge.target_node_id)
            .or_default()
            .push(edge.source_node_id);
    }
    let mut visited = BTreeSet::new();
    let mut queue = VecDeque::from([target_node_id]);
    while let Some(node_id) = queue.pop_front() {
        if !visited.insert(node_id) {
            continue;
        }
        for source_id in incoming.get(&node_id).into_iter().flatten() {
            queue.push_back(*source_id);
        }
    }
    Ok(visited)
}

pub(super) async fn finish_canvas_run_failed(
    state: &AppState,
    workflow_run_id: i64,
    project_id: i64,
    graph_id: i64,
    mode: &str,
    step_key: String,
    input_json: Value,
    error: String,
) -> Result<CanvasRunSummary, String> {
    record_canvas_node_checkpoint(
        state,
        workflow_run_id,
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
            workflow_run_id,
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

pub(super) async fn record_canvas_node_checkpoint(
    state: &AppState,
    workflow_run_id: i64,
    step_key: String,
    status: &str,
    input_json: Value,
    artifacts_json: Option<Value>,
    error_message: Option<String>,
    completed: bool,
) -> Result<(), String> {
    if status != WORKFLOW_STATUS_COMPLETED {
        state
            .workflow_run
            .mark_current_step(workflow_run_id, &step_key)
            .await
            .map_err(|e| e.to_string())?;
    }
    state
        .workflow_run
        .upsert_checkpoint(UpsertWorkflowCheckpointInput {
            workflow_run_id,
            step_key: step_key.clone(),
            step_order: 0,
            status: status.to_string(),
            input_fingerprint_sha256: None,
            input_fingerprint_json: Some(input_json),
            checkpoint_json: artifacts_json.clone(),
            artifacts_json,
            generation_run_ids_json: None,
            error_message,
            completed,
        })
        .await
        .map_err(|e| e.to_string())?;
    let progress_message = canvas_progress_message(status, &step_key);
    state
        .workflow_run
        .record_progress_event(WorkflowProgressEventInput {
            workflow_run_id,
            task_id: None,
            current: if status == WORKFLOW_STATUS_RUNNING {
                0
            } else {
                1
            },
            total: 1,
            message: &progress_message,
            meta: None,
        })
        .await
        .map_err(|e| e.to_string())?;
    Ok(())
}

fn canvas_progress_message(status: &str, step_key: &str) -> String {
    match status {
        WORKFLOW_STATUS_RUNNING => format!("画布节点运行中: {step_key}"),
        WORKFLOW_STATUS_COMPLETED => format!("画布节点已完成: {step_key}"),
        WORKFLOW_STATUS_FAILED => format!("画布节点失败: {step_key}"),
        other => format!("画布节点状态 {other}: {step_key}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use a4n_core::models::canvas::{canvas_edge, canvas_graph};

    fn test_graph_document(
        nodes: Vec<canvas_node::Model>,
        edges: Vec<canvas_edge::Model>,
    ) -> CanvasGraphDocument {
        let now = chrono::Utc::now().naive_utc();
        CanvasGraphDocument {
            graph: canvas_graph::Model {
                id: 10,
                project_id: 20,
                name: "测试画布".to_string(),
                graph_kind: "video_studio".to_string(),
                version: 1,
                viewport_json: None,
                created_at: now,
                updated_at: now,
            },
            nodes,
            edges,
            text_assets: Vec::new(),
        }
    }

    fn test_node_with_id(id: i64, kind: &str) -> canvas_node::Model {
        let now = chrono::Utc::now().naive_utc();
        canvas_node::Model {
            id,
            graph_id: 10,
            node_kind: kind.to_string(),
            ref_type: None,
            ref_id: None,
            config_json: json!({}),
            position_json: json!({"x": 0, "y": 0}),
            state: "ready".to_string(),
            created_at: now,
            updated_at: now,
        }
    }

    fn test_edge(source_node_id: i64, target_node_id: i64) -> canvas_edge::Model {
        canvas_edge::Model {
            id: source_node_id * 100 + target_node_id,
            graph_id: 10,
            source_node_id,
            source_port: "output".to_string(),
            target_node_id,
            target_port: "input".to_string(),
            role: "context".to_string(),
            config_json: json!({}),
        }
    }

    #[test]
    fn canvas_node_execution_kind_rejects_non_operation_nodes() {
        assert!(canvas_node_kind_is_executable("RenderTextNode"));
        assert!(canvas_node_kind_is_executable("LlmTextNode"));
        assert!(canvas_node_kind_is_executable("GenerateVideoNode"));
        assert!(canvas_node_kind_is_executable("AssembleVideoNode"));
        assert!(canvas_node_kind_is_executable("TextProbeNode"));
        assert!(canvas_node_kind_is_executable("ImageProbeNode"));
        assert!(canvas_node_kind_is_executable("VideoProbeNode"));
        assert!(canvas_node_kind_is_executable("AudioProbeNode"));
        assert!(canvas_node_kind_is_executable("VideoOutputNode"));
        assert!(canvas_node_kind_is_output("VideoOutputNode"));

        assert!(!canvas_node_kind_is_executable("TextInputNode"));
        assert!(!canvas_node_kind_is_output("GenerateVideoNode"));
        assert!(!canvas_node_kind_is_output("TextProbeNode"));
        assert!(!canvas_node_kind_is_output("TextInputNode"));
        assert!(!canvas_node_kind_is_executable("RenderPromptNode"));
        assert!(!canvas_node_kind_is_executable("LlmTransformNode"));
        assert!(!canvas_node_kind_is_executable("AssembleNode"));
        assert!(!canvas_node_kind_is_executable("ExportVideoNode"));
        assert!(!canvas_node_kind_is_executable("TextNode"));
        assert!(!canvas_node_kind_is_executable("ConfigNode"));
        assert!(!canvas_node_kind_is_executable("MediaAssetNode"));

        assert_eq!(
            unsupported_canvas_node_execution_error("TextNode"),
            "画布节点类型不支持执行: TextNode"
        );
    }

    #[test]
    fn canvas_graph_execution_scope_supports_full_and_to_node() {
        let document = test_graph_document(
            vec![
                test_node_with_id(1, "TextInputNode"),
                test_node_with_id(2, "LlmTextNode"),
                test_node_with_id(3, "GenerateImageNode"),
                test_node_with_id(4, "GenerateVideoNode"),
                test_node_with_id(5, "VideoOutputNode"),
            ],
            vec![
                test_edge(1, 2),
                test_edge(2, 3),
                test_edge(3, 4),
                test_edge(4, 5),
            ],
        );

        assert_eq!(
            runnable_canvas_node_ids(&document, None).unwrap(),
            vec![2, 3, 4, 5]
        );
        assert_eq!(
            runnable_canvas_node_ids(&document, Some(5)).unwrap(),
            vec![2, 3, 4, 5]
        );
        assert!(
            runnable_canvas_node_ids(&document, Some(999))
                .expect_err("missing target node should fail")
                .contains("画布节点不存在")
        );
    }
}
