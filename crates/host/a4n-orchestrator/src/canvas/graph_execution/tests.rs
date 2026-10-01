use std::sync::{Arc, Mutex};

use a4n_api_contract::canvas::CanvasRunSummary;
use a4n_canvas_contract::{
    CanvasGraphExecutionHostPort, CanvasGraphNodeExecutionInput, CanvasGraphPreparationHostPort,
    CanvasGraphResumePreparationInput, CanvasGraphRunPreparationInput,
    CanvasSingleNodeRunPreparationInput,
};
use a4n_db::{
    DatabaseConnection,
    canvas::{CanvasGraphRecordDocument, CanvasGraphRecordInput, CanvasNodeRecordInput},
    project::{CreateFreeformVideoProjectParams, create_freeform_video_project_record},
};
use a4n_db_models::models::generation::workflow_run;
use async_trait::async_trait;
use sea_orm::EntityTrait;
use serde_json::json;

use super::planning::{
    canvas_node_kind_is_executable, canvas_node_kind_is_output, runnable_canvas_node_ids,
    unsupported_canvas_node_execution_error,
};
use crate::{
    OrchestratorContext,
    test_support::TestOrchestratorBuilder,
    workflow::{UpsertWorkflowCheckpointInput, WORKFLOW_STATUS_COMPLETED, WORKFLOW_STATUS_FAILED},
};

#[test]
fn canvas_graph_execution_uses_canonical_operation_and_output_kinds() {
    for node_kind in [
        "RenderTextNode",
        "LlmTextNode",
        "GenerateImageNode",
        "GenerateVideoNode",
        "ExtractFrameNode",
        "TrimVideoNode",
        "AssembleVideoNode",
        "TextProbeNode",
        "ImageProbeNode",
        "VideoProbeNode",
        "AudioProbeNode",
        "TextOutputNode",
        "ImageOutputNode",
        "VideoOutputNode",
        "AudioOutputNode",
    ] {
        assert!(
            canvas_node_kind_is_executable(node_kind),
            "{node_kind} 应是 canonical 可执行节点"
        );
    }
    for node_kind in [
        "TextOutputNode",
        "ImageOutputNode",
        "VideoOutputNode",
        "AudioOutputNode",
    ] {
        assert!(canvas_node_kind_is_output(node_kind));
    }
    for node_kind in [
        "TextInputNode",
        "RenderPromptNode",
        "LlmTransformNode",
        "AssembleNode",
        "ExportVideoNode",
        "TextNode",
        "ConfigNode",
        "MediaAssetNode",
    ] {
        assert!(!canvas_node_kind_is_executable(node_kind));
    }
    assert!(!canvas_node_kind_is_output("GenerateVideoNode"));
    assert!(!canvas_node_kind_is_output("TextProbeNode"));
    assert_eq!(
        unsupported_canvas_node_execution_error("TextNode"),
        "画布节点类型不支持执行: TextNode"
    );
}

#[test]
fn canvas_graph_execution_scope_supports_full_graph_and_output_target() {
    let document = test_graph_document(
        vec![
            test_node(1, "TextInputNode"),
            test_node(2, "LlmTextNode"),
            test_node(3, "GenerateImageNode"),
            test_node(4, "GenerateVideoNode"),
            test_node(5, "VideoOutputNode"),
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
            .expect_err("不存在的目标节点必须失败")
            .contains("画布节点不存在")
    );
}

#[test]
fn canvas_graph_execution_to_output_runs_upstream_with_existing_results() {
    let mut source = test_node(1, "TextInputNode");
    source.config_json = json!({"content": "用户修改后的输入"});
    let mut video = test_node(2, "GenerateVideoNode");
    video.config_json = json!({
        "canvas_runtime_result": true,
        "generated_video_id": 91,
    });
    let document = test_graph_document(
        vec![source, video, test_node(3, "VideoOutputNode")],
        vec![test_edge(1, 2), test_edge(2, 3)],
    );

    assert_eq!(
        runnable_canvas_node_ids(&document, Some(3)).unwrap(),
        vec![2, 3]
    );
    assert_eq!(
        runnable_canvas_node_ids(&document, None).unwrap(),
        vec![2, 3]
    );
}

#[tokio::test]
async fn canvas_graph_application_executes_nodes_in_order_and_completes_workflow() {
    let (database, context) = setup_context().await;
    let (project_id, document) = insert_test_graph(database.as_ref(), &context, 3).await;
    let host_port = TestGraphHostPort::default();

    let prepared = context
        .prepare_canvas_graph_run_validated(
            CanvasGraphRunPreparationInput {
                user_id: 1,
                project_id,
                graph_id: document.graph.id,
                target_node_id: None,
            },
            &host_port,
        )
        .await
        .expect("整图运行应准备成功");
    let expected_node_ids = prepared.runnable_node_ids.clone();
    let summary = context
        .run_prepared_canvas_graph_validated(prepared, &host_port)
        .await
        .expect("整图运行应返回持久化终态");
    let run = workflow_run::Entity::find_by_id(summary.workflow_run_id)
        .one(database.as_ref())
        .await
        .expect("应可查询 workflow")
        .expect("workflow 应存在");

    assert_eq!(summary.status, WORKFLOW_STATUS_COMPLETED);
    assert_eq!(run.status, WORKFLOW_STATUS_COMPLETED);
    assert_eq!(host_port.executed_node_ids(), expected_node_ids);
    assert_eq!(host_port.verification_calls(), vec![(1, project_id)]);
}

#[tokio::test]
async fn canvas_graph_application_marks_workflow_failed_and_stops_after_host_error() {
    let (database, context) = setup_context().await;
    let (project_id, document) = insert_test_graph(database.as_ref(), &context, 3).await;
    let preparing_port = TestGraphHostPort::default();
    let prepared = context
        .prepare_canvas_graph_run_validated(
            CanvasGraphRunPreparationInput {
                user_id: 1,
                project_id,
                graph_id: document.graph.id,
                target_node_id: None,
            },
            &preparing_port,
        )
        .await
        .expect("整图运行应准备成功");
    let failed_node_id = prepared.runnable_node_ids[1];
    let host_port = TestGraphHostPort::failing_at(failed_node_id);

    let summary = context
        .run_prepared_canvas_graph_validated(prepared, &host_port)
        .await
        .expect("节点失败应返回持久化后的 failed summary");
    let run = workflow_run::Entity::find_by_id(summary.workflow_run_id)
        .one(database.as_ref())
        .await
        .expect("应可查询失败 workflow")
        .expect("失败 workflow 应存在");

    assert_eq!(summary.status, WORKFLOW_STATUS_FAILED);
    assert_eq!(run.status, WORKFLOW_STATUS_FAILED);
    assert!(
        summary
            .error_message
            .as_deref()
            .is_some_and(|message| message.contains("模拟节点失败"))
    );
    assert_eq!(host_port.executed_node_ids().len(), 2);
    assert_eq!(host_port.executed_node_ids()[1], failed_node_id);
}

#[tokio::test]
async fn canvas_graph_resume_skips_completed_checkpoints_and_finishes_pending_nodes() {
    let (database, context) = setup_context().await;
    let (project_id, document) = insert_test_graph(database.as_ref(), &context, 2).await;
    let preparing_port = TestGraphHostPort::default();
    let prepared = context
        .prepare_canvas_graph_run_validated(
            CanvasGraphRunPreparationInput {
                user_id: 1,
                project_id,
                graph_id: document.graph.id,
                target_node_id: None,
            },
            &preparing_port,
        )
        .await
        .expect("整图运行应准备成功");
    let completed_node_id = prepared.runnable_node_ids[0];
    let pending_node_id = prepared.runnable_node_ids[1];
    context
        .upsert_workflow_checkpoint(UpsertWorkflowCheckpointInput {
            workflow_run_id: prepared.workflow_run_id,
            step_key: format!("canvas_node:{completed_node_id}"),
            step_order: 0,
            status: WORKFLOW_STATUS_COMPLETED.to_string(),
            input_fingerprint_sha256: None,
            input_fingerprint_json: None,
            checkpoint_json: None,
            artifacts_json: None,
            generation_run_ids_json: None,
            error_message: None,
            completed: true,
        })
        .await
        .expect("completed checkpoint 应写入成功");
    context
        .fail_workflow_run_validated(prepared.workflow_run_id, "模拟中断")
        .await
        .expect("workflow 应可进入可恢复失败态");

    let host_port = TestGraphHostPort::default();
    let resumed = context
        .prepare_canvas_graph_resume_validated(
            CanvasGraphResumePreparationInput {
                user_id: 1,
                workflow_run_id: prepared.workflow_run_id,
            },
            &host_port,
        )
        .await
        .expect("workflow 应恢复成功");
    assert_eq!(resumed.runnable_node_ids, vec![pending_node_id]);

    let summary = context
        .run_prepared_canvas_graph_validated(resumed, &host_port)
        .await
        .expect("恢复执行应成功");
    let run = workflow_run::Entity::find_by_id(summary.workflow_run_id)
        .one(database.as_ref())
        .await
        .expect("应可查询恢复后的 workflow")
        .expect("恢复后的 workflow 应存在");

    assert_eq!(summary.status, WORKFLOW_STATUS_COMPLETED);
    assert_eq!(run.status, WORKFLOW_STATUS_COMPLETED);
    assert_eq!(host_port.executed_node_ids(), vec![pending_node_id]);
}

#[tokio::test]
async fn canvas_single_node_preparation_limits_execution_to_requested_node() {
    let (database, context) = setup_context().await;
    let (project_id, document) = insert_test_graph(database.as_ref(), &context, 3).await;
    let requested_node_id = document.nodes[1].id;
    let host_port = TestGraphHostPort::default();

    let prepared = context
        .prepare_canvas_single_node_run_validated(
            CanvasSingleNodeRunPreparationInput {
                user_id: 1,
                project_id,
                graph_id: document.graph.id,
                node_id: requested_node_id,
            },
            &host_port,
        )
        .await
        .expect("单节点运行应准备成功");

    assert_eq!(prepared.mode, "single_node");
    assert_eq!(prepared.runnable_node_ids, vec![requested_node_id]);
}

#[derive(Default)]
struct TestGraphHostPort {
    fail_node_id: Option<i64>,
    verification_calls: Mutex<Vec<(i64, i64)>>,
    executed_node_ids: Mutex<Vec<i64>>,
}

impl TestGraphHostPort {
    fn failing_at(node_id: i64) -> Self {
        Self {
            fail_node_id: Some(node_id),
            ..Self::default()
        }
    }

    fn verification_calls(&self) -> Vec<(i64, i64)> {
        self.verification_calls
            .lock()
            .expect("verification mutex")
            .clone()
    }

    fn executed_node_ids(&self) -> Vec<i64> {
        self.executed_node_ids
            .lock()
            .expect("execution mutex")
            .clone()
    }
}

#[async_trait]
impl CanvasGraphPreparationHostPort for TestGraphHostPort {
    async fn verify_project_editor(&self, user_id: i64, project_id: i64) -> Result<(), String> {
        self.verification_calls
            .lock()
            .expect("verification mutex")
            .push((user_id, project_id));
        Ok(())
    }
}

#[async_trait]
impl CanvasGraphExecutionHostPort for TestGraphHostPort {
    async fn execute_node(
        &self,
        input: CanvasGraphNodeExecutionInput,
    ) -> Result<CanvasRunSummary, String> {
        self.executed_node_ids
            .lock()
            .expect("execution mutex")
            .push(input.node_id);
        if self.fail_node_id == Some(input.node_id) {
            return Err(format!("模拟节点失败: {}", input.node_id));
        }
        Ok(CanvasRunSummary {
            workflow_run_id: input.shared_run.workflow_run_id,
            project_id: input.project_id,
            graph_id: input.graph_id,
            mode: input.mode,
            status: WORKFLOW_STATUS_COMPLETED.to_string(),
            error_message: None,
        })
    }
}

async fn setup_context() -> (Arc<DatabaseConnection>, OrchestratorContext) {
    let database = Arc::new(a4n_migration::test_support::connect_test_database().await);
    let context = TestOrchestratorBuilder::new(Arc::clone(&database))
        .build()
        .await;
    (database, context)
}

async fn insert_test_graph(
    database: &DatabaseConnection,
    context: &OrchestratorContext,
    operation_count: usize,
) -> (i64, CanvasGraphRecordDocument) {
    let project = create_freeform_video_project_record(
        database,
        1,
        CreateFreeformVideoProjectParams {
            title: format!("Canvas Graph Runner application 测试-{operation_count}"),
            description: String::new(),
        },
    )
    .await
    .expect("测试项目应创建成功");
    let operation_kinds = ["RenderTextNode", "LlmTextNode", "TextOutputNode"];
    let nodes = operation_kinds
        .into_iter()
        .take(operation_count)
        .enumerate()
        .map(|(index, node_kind)| record_node(&format!("operation-{index}"), node_kind))
        .collect();
    let document = context
        .save_canvas_graph_record(
            project.id,
            CanvasGraphRecordInput {
                id: None,
                expected_version: None,
                name: Some("Graph Runner 测试画布".to_string()),
                graph_kind: Some("video_studio".to_string()),
                viewport_json: None,
                nodes,
                edges: Vec::new(),
            },
            chrono::Utc::now().naive_utc(),
        )
        .await
        .expect("测试画布应保存成功");
    (project.id, document)
}

fn record_node(client_id: &str, node_kind: &str) -> CanvasNodeRecordInput {
    CanvasNodeRecordInput {
        client_id: client_id.to_string(),
        node_kind: node_kind.to_string(),
        ref_type: None,
        ref_id: None,
        config_json: Some(json!({})),
        position_json: json!({"x": 0, "y": 0}),
        state: None,
    }
}

fn test_graph_document(
    nodes: Vec<a4n_domain::canvas::CanvasNodeRecord>,
    edges: Vec<a4n_domain::canvas::CanvasEdgeRecord>,
) -> CanvasGraphRecordDocument {
    let now = chrono::Utc::now().naive_utc();
    CanvasGraphRecordDocument {
        graph: a4n_domain::canvas::CanvasGraphRecord {
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

fn test_node(id: i64, node_kind: &str) -> a4n_domain::canvas::CanvasNodeRecord {
    let now = chrono::Utc::now().naive_utc();
    a4n_domain::canvas::CanvasNodeRecord {
        id,
        graph_id: 10,
        node_kind: node_kind.to_string(),
        ref_type: None,
        ref_id: None,
        config_json: json!({}),
        position_json: json!({"x": 0, "y": 0}),
        state: "ready".to_string(),
        created_at: now,
        updated_at: now,
    }
}

fn test_edge(source_node_id: i64, target_node_id: i64) -> a4n_domain::canvas::CanvasEdgeRecord {
    a4n_domain::canvas::CanvasEdgeRecord {
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
