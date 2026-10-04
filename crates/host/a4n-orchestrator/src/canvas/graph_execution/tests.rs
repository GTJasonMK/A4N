use std::sync::{Arc, Mutex};

use a4n_api_contract::canvas::CanvasRunSummary;
use a4n_canvas_contract::{
    CanvasGraphExecutionHostPort, CanvasGraphNodeExecutionInput, CanvasGraphPreparationHostPort,
    CanvasGraphResumePreparationInput, CanvasGraphRunPreparationInput,
    CanvasSingleNodeRunPreparationInput,
};
use a4n_db::{
    DatabaseConnection,
    canvas::{
        CanvasEdgeRecordInput, CanvasGraphRecordDocument, CanvasGraphRecordInput,
        CanvasNodeRecordInput,
    },
    project::{CreateCanvasProjectParams, create_canvas_project_record},
};
use a4n_db_models::models::{canvas::canvas_graph, generation::workflow_run};
use async_trait::async_trait;
use sea_orm::{ActiveModelTrait, EntityTrait, Set};
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
async fn canvas_graph_application_stops_when_graph_changes_during_a_node() {
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
    let first_node_id = prepared.runnable_node_ids[0];
    let host_port = TestGraphHostPort::changing_graph_after(
        Arc::clone(&database),
        document.graph.id,
        first_node_id,
    );

    let summary = context
        .run_prepared_canvas_graph_validated(prepared, &host_port)
        .await
        .expect("图变更应使工作流失败并持久化终态");
    let run = workflow_run::Entity::find_by_id(summary.workflow_run_id)
        .one(database.as_ref())
        .await
        .expect("应可查询 workflow")
        .expect("workflow 应存在");

    assert_eq!(summary.status, WORKFLOW_STATUS_FAILED);
    assert_eq!(run.status, WORKFLOW_STATUS_FAILED);
    assert!(
        summary
            .error_message
            .as_deref()
            .is_some_and(|message| message.contains("画布在工作流运行期间已更改"))
    );
    assert_eq!(host_port.executed_node_ids(), vec![first_node_id]);
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
async fn canvas_graph_run_keeps_the_source_text_version_when_resumed() {
    let (database, context) = setup_context().await;
    let project = create_canvas_project_record(
        database.as_ref(),
        1,
        CreateCanvasProjectParams {
            title: "Canvas text snapshot test".to_string(),
            description: String::new(),
        },
    )
    .await
    .expect("测试项目应创建成功");
    let asset = context
        .upsert_canvas_text_asset_record(
            project.id,
            a4n_domain::canvas::CanvasTextAssetRecordInput {
                id: None,
                expected_version: None,
                title: "剧本".to_string(),
                text_kind: "script".to_string(),
                content: "第一版正文".to_string(),
            },
            chrono::Utc::now().naive_utc(),
        )
        .await
        .expect("文本素材应创建成功");
    let mut source = record_node("source", "TextInputNode");
    source.ref_type = Some("canvas_text_asset".to_string());
    source.ref_id = Some(asset.id);
    let document = context
        .save_canvas_graph_record(
            project.id,
            CanvasGraphRecordInput {
                id: None,
                expected_version: None,
                name: Some("文本来源工作流".to_string()),
                graph_kind: Some("video_studio".to_string()),
                viewport_json: None,
                nodes: vec![source, record_node("rewrite", "LlmTextNode")],
                edges: vec![CanvasEdgeRecordInput {
                    source_client_id: "source".to_string(),
                    target_client_id: "rewrite".to_string(),
                    source_port: "text".to_string(),
                    target_port: "input".to_string(),
                    role: "context".to_string(),
                    config_json: Some(json!({})),
                }],
            },
            chrono::Utc::now().naive_utc(),
        )
        .await
        .expect("测试画布应保存成功");
    let host_port = TestGraphHostPort::default();
    let prepared = context
        .prepare_canvas_graph_run_validated(
            CanvasGraphRunPreparationInput {
                user_id: 1,
                project_id: project.id,
                graph_id: document.graph.id,
                target_node_id: None,
            },
            &host_port,
        )
        .await
        .expect("工作流应使用文本素材版本准备成功");
    let run = workflow_run::Entity::find_by_id(prepared.workflow_run_id)
        .one(database.as_ref())
        .await
        .expect("应可读取工作流")
        .expect("工作流应存在");
    let snapshot = run
        .input_fingerprint_json
        .as_ref()
        .and_then(|input| input.get("canvas_text_asset_snapshot"))
        .and_then(serde_json::Value::as_array)
        .expect("工作流应保存文本素材快照");
    assert_eq!(snapshot.len(), 1);
    assert_eq!(snapshot[0]["id"], asset.id);
    assert_eq!(snapshot[0]["current_version"], 1);
    assert_eq!(snapshot[0]["content"], "第一版正文");

    context
        .upsert_canvas_text_asset_record(
            project.id,
            a4n_domain::canvas::CanvasTextAssetRecordInput {
                id: Some(asset.id),
                expected_version: Some(1),
                title: asset.title,
                text_kind: "script".to_string(),
                content: "第二版正文".to_string(),
            },
            chrono::Utc::now().naive_utc(),
        )
        .await
        .expect("新文本版本应保存成功");
    let current_document = context
        .get_or_create_canvas_graph_record(
            project.id,
            Some(document.graph.id),
            chrono::Utc::now().naive_utc(),
        )
        .await
        .expect("应可读取更新后的画布");
    let rewrite_node_id = document
        .nodes
        .iter()
        .find(|node| node.node_kind == "LlmTextNode")
        .expect("改写节点应存在")
        .id;
    let pinned_document = context
        .canvas_document_with_workflow_text_asset_snapshot(
            &current_document,
            Some(prepared.workflow_run_id),
            rewrite_node_id,
        )
        .await
        .expect("节点应能读取运行开始时的文本快照");
    assert_eq!(pinned_document.text_assets[0].current_version, 1);
    assert_eq!(pinned_document.text_assets[0].content, "第一版正文");

    context
        .fail_workflow_run_validated(prepared.workflow_run_id, "模拟恢复")
        .await
        .expect("工作流应进入可恢复状态");
    let resumed = context
        .prepare_canvas_graph_resume_validated(
            CanvasGraphResumePreparationInput {
                user_id: 1,
                workflow_run_id: prepared.workflow_run_id,
            },
            &host_port,
        )
        .await
        .expect("续跑应保留原文本素材快照");
    let resumed_document = context
        .canvas_document_with_workflow_text_asset_snapshot(
            &current_document,
            Some(resumed.workflow_run_id),
            rewrite_node_id,
        )
        .await
        .expect("续跑节点应仍读取原文本快照");
    assert_eq!(resumed_document.text_assets[0].current_version, 1);
    assert_eq!(resumed_document.text_assets[0].content, "第一版正文");
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
    graph_version_change_after: Option<(Arc<DatabaseConnection>, i64, i64)>,
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

    fn changing_graph_after(
        database: Arc<DatabaseConnection>,
        graph_id: i64,
        node_id: i64,
    ) -> Self {
        Self {
            graph_version_change_after: Some((database, graph_id, node_id)),
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
        if let Some((database, graph_id, node_id)) = &self.graph_version_change_after
            && *graph_id == input.graph_id
            && *node_id == input.node_id
        {
            let graph = canvas_graph::Entity::find_by_id(*graph_id)
                .one(database.as_ref())
                .await
                .map_err(|error| error.to_string())?
                .expect("测试画布应存在");
            canvas_graph::ActiveModel {
                id: Set(graph.id),
                version: Set(graph.version + 1),
                updated_at: Set(chrono::Utc::now().naive_utc()),
                ..Default::default()
            }
            .update(database.as_ref())
            .await
            .map_err(|error| error.to_string())?;
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
    let project = create_canvas_project_record(
        database,
        1,
        CreateCanvasProjectParams {
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
