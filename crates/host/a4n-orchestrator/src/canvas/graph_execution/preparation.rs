use std::collections::BTreeSet;

use a4n_canvas_contract::{
    CanvasGraphPreparationHostPort, CanvasGraphResumePreparationInput,
    CanvasGraphRunPreparationInput, CanvasSingleNodeRunPreparationInput, PreparedCanvasGraphRun,
};
use a4n_db::canvas::{CanvasGraphRecordDocument, validate_canvas_record_document_edge_contract};
use chrono::Utc;
use serde_json::{Value, json};

use crate::{
    OrchestratorContext,
    canvas::{WORKFLOW_KIND_VIDEO_CANVAS_GRAPH, canvas_node_step_key},
    workflow::WORKFLOW_STATUS_COMPLETED,
};

use super::planning::{
    is_runnable_canvas_node, runnable_canvas_node_ids, unsupported_canvas_node_execution_error,
    validate_canvas_output_target,
};

impl OrchestratorContext {
    pub async fn prepare_canvas_graph_run_validated(
        &self,
        input: CanvasGraphRunPreparationInput,
        host_port: &dyn CanvasGraphPreparationHostPort,
    ) -> Result<PreparedCanvasGraphRun, String> {
        let document = self
            .load_canvas_execution_document(
                input.user_id,
                input.project_id,
                input.graph_id,
                host_port,
            )
            .await?;
        validate_canvas_output_target(&document, input.target_node_id)?;
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
            "runnable_node_ids": runnable_node_ids,
        });
        let first_step_key = runnable_node_ids
            .first()
            .map(|node_id| canvas_node_step_key(*node_id))
            .transpose()?;
        let graph_run = self
            .begin_canvas_graph_workflow_validated(
                input.project_id,
                input.graph_id,
                mode,
                Utc::now().timestamp_micros(),
                graph_input_json,
                first_step_key,
            )
            .await
            .map_err(|error| error.to_string())?;
        Ok(PreparedCanvasGraphRun {
            workflow_run_id: graph_run.id,
            user_id: input.user_id,
            project_id: input.project_id,
            graph_id: input.graph_id,
            mode: mode.to_string(),
            runnable_node_ids,
        })
    }

    pub async fn prepare_canvas_single_node_run_validated(
        &self,
        input: CanvasSingleNodeRunPreparationInput,
        host_port: &dyn CanvasGraphPreparationHostPort,
    ) -> Result<PreparedCanvasGraphRun, String> {
        let document = self
            .load_canvas_execution_document(
                input.user_id,
                input.project_id,
                input.graph_id,
                host_port,
            )
            .await?;
        let node = document
            .nodes
            .iter()
            .find(|node| node.id == input.node_id)
            .ok_or_else(|| format!("画布节点不存在: {}", input.node_id))?;
        if !is_runnable_canvas_node(node) {
            return Err(unsupported_canvas_node_execution_error(&node.node_kind));
        }
        let step_key = canvas_node_step_key(input.node_id)?;
        let graph_input_json = json!({
            "project_id": input.project_id,
            "graph_id": input.graph_id,
            "graph_kind": document.graph.graph_kind,
            "graph_version": document.graph.version,
            "node_id": input.node_id,
            "mode": "single_node",
            "runnable_node_ids": [input.node_id],
        });
        let graph_run = self
            .begin_canvas_single_node_workflow_validated(
                input.project_id,
                input.graph_id,
                input.node_id,
                Utc::now().timestamp_micros(),
                graph_input_json,
                step_key,
            )
            .await
            .map_err(|error| error.to_string())?;
        Ok(PreparedCanvasGraphRun {
            workflow_run_id: graph_run.id,
            user_id: input.user_id,
            project_id: input.project_id,
            graph_id: input.graph_id,
            mode: "single_node".to_string(),
            runnable_node_ids: vec![input.node_id],
        })
    }

    pub async fn prepare_canvas_graph_resume_validated(
        &self,
        input: CanvasGraphResumePreparationInput,
        host_port: &dyn CanvasGraphPreparationHostPort,
    ) -> Result<PreparedCanvasGraphRun, String> {
        let run = self
            .workflow_run_summary(input.workflow_run_id)
            .await
            .map_err(|error| error.to_string())?;
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
        let target_node_id = run
            .input_fingerprint_json
            .as_ref()
            .and_then(|value| value.get("target_node_id"))
            .and_then(Value::as_i64);
        let mode = run.mode.ok_or_else(|| "画布运行缺少 mode".to_string())?;
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
        let document = self
            .load_canvas_execution_document(input.user_id, project_id, graph_id, host_port)
            .await?;
        if let Some(graph_version) = run
            .input_fingerprint_json
            .as_ref()
            .and_then(|value| value.get("graph_version"))
            .and_then(Value::as_i64)
            && graph_version != i64::from(document.graph.version)
        {
            return Err(format!(
                "画布已在 workflow 运行后变更: workflow_graph_version={graph_version}, current={}",
                document.graph.version
            ));
        }
        validate_canvas_output_target(&document, target_node_id)?;
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
        let completed_step_keys = self
            .workflow_checkpoint_step_statuses(input.workflow_run_id)
            .await
            .map_err(|error| error.to_string())?
            .into_iter()
            .filter(|(_, status)| status == WORKFLOW_STATUS_COMPLETED)
            .map(|(step_key, _)| step_key)
            .collect::<BTreeSet<_>>();
        let mut pending_node_ids = Vec::new();
        for node_id in runnable_node_ids {
            let step_key = canvas_node_step_key(node_id)?;
            if !completed_step_keys.contains(&step_key) {
                pending_node_ids.push(node_id);
            }
        }
        if pending_node_ids.is_empty() {
            let completed = self
                .complete_canvas_graph_workflow_validated(input.workflow_run_id)
                .await
                .map_err(|error| error.to_string())?;
            return Ok(PreparedCanvasGraphRun {
                workflow_run_id: completed.id,
                user_id: input.user_id,
                project_id,
                graph_id,
                mode,
                runnable_node_ids: Vec::new(),
            });
        }
        self.mark_workflow_running_validated(input.workflow_run_id)
            .await
            .map_err(|error| error.to_string())?;
        if let Some(first_node_id) = pending_node_ids.first() {
            let step_key = canvas_node_step_key(*first_node_id)?;
            self.mark_workflow_current_step(input.workflow_run_id, &step_key)
                .await
                .map_err(|error| error.to_string())?;
        }
        Ok(PreparedCanvasGraphRun {
            workflow_run_id: input.workflow_run_id,
            user_id: input.user_id,
            project_id,
            graph_id,
            mode,
            runnable_node_ids: pending_node_ids,
        })
    }

    async fn load_canvas_execution_document(
        &self,
        user_id: i64,
        project_id: i64,
        graph_id: i64,
        host_port: &dyn CanvasGraphPreparationHostPort,
    ) -> Result<CanvasGraphRecordDocument, String> {
        host_port.verify_project_editor(user_id, project_id).await?;
        let document = self
            .get_or_create_canvas_graph_record(project_id, Some(graph_id), Utc::now().naive_utc())
            .await
            .map_err(|error| error.to_string())?;
        validate_canvas_record_document_edge_contract(&document)
            .map_err(|error| error.to_string())?;
        Ok(document)
    }
}
