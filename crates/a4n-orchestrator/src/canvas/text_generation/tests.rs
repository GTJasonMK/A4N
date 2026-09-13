use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

use a4n_canvas_contract::{
    CanvasLlmTextNodeApplicationInput, CanvasRenderTextNodeApplicationInput,
    CanvasRenderTextNodeApplicationPort,
};
use a4n_db::{
    DatabaseConnection,
    canvas::{
        CanvasEdgeRecordInput, CanvasGraphRecordDocument, CanvasGraphRecordInput,
        CanvasNodeRecordInput,
    },
    generation::{GENERATION_PROMPT_KIND_CANVAS_SHOT_SCRIPT, GENERATION_PROMPT_KIND_VIDEO_PROMPT},
    project::{CreateFreeformVideoProjectParams, create_freeform_video_project_record},
};
use a4n_db_models::models::generation::{
    generation_spec, prompt_artifact, workflow_checkpoint, workflow_run,
};
use a4n_error::AppError;
use a4n_llm::provider::{ChatMessage, ChatResponse, UsageInfo};
use async_trait::async_trait;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
use serde_json::{Value, json};

use crate::{OrchestratorContext, test_support::TestOrchestratorBuilder};

use super::{
    CanvasLlmTextNodeApplicationPort, CanvasTextGenerationLlmCall, CanvasTextGenerationLlmPort,
    build_canvas_llm_text_request_for_preview,
    execution::{map_canvas_llm_error, validate_restored_checkpoint_for_test},
    input::{build_canvas_llm_text_input, build_canvas_rendered_prompt},
};

struct TestLlmPort {
    calls: AtomicUsize,
    result: Result<ChatResponse, String>,
}

impl TestLlmPort {
    fn success(content: &str, finish_reason: Option<&str>) -> Self {
        Self {
            calls: AtomicUsize::new(0),
            result: Ok(ChatResponse {
                message: ChatMessage::assistant(content),
                usage: Some(UsageInfo {
                    prompt_tokens: 10,
                    completion_tokens: 4,
                    total_tokens: 14,
                    prompt_cache_hit_tokens: None,
                    prompt_cache_miss_tokens: None,
                    prompt_cache_write_tokens: None,
                }),
                finish_reason: finish_reason.map(str::to_string),
                reasoning_content: None,
            }),
        }
    }

    fn failure(message: &str) -> Self {
        Self {
            calls: AtomicUsize::new(0),
            result: Err(message.to_string()),
        }
    }
}

#[async_trait]
impl CanvasTextGenerationLlmPort for TestLlmPort {
    async fn max_output_tokens(&self, _config_id: Option<i64>) -> Result<usize, AppError> {
        Ok(4096)
    }

    async fn stream_complete(
        &self,
        _input: CanvasTextGenerationLlmCall,
    ) -> Result<ChatResponse, AppError> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        self.result.clone().map_err(AppError::LlmError)
    }
}

async fn setup_context() -> (Arc<DatabaseConnection>, OrchestratorContext) {
    let database = Arc::new(a4n_migration::test_support::connect_test_database().await);
    crate::test_support::create_test_user(database.as_ref()).await;
    let context = TestOrchestratorBuilder::new(Arc::clone(&database))
        .build()
        .await;
    (database, context)
}

async fn insert_canvas(
    database: &DatabaseConnection,
    context: &OrchestratorContext,
    target_kind: &str,
    target_config: Value,
) -> (i64, CanvasGraphRecordDocument) {
    let project = create_freeform_video_project_record(
        database,
        1,
        CreateFreeformVideoProjectParams {
            title: format!("Canvas {target_kind} application 测试"),
            description: String::new(),
        },
    )
    .await
    .expect("测试项目应创建成功");
    let document = context
        .save_canvas_graph_record(
            project.id,
            CanvasGraphRecordInput {
                id: None,
                expected_version: None,
                name: Some("文本生成测试画布".to_string()),
                graph_kind: Some("video_studio".to_string()),
                viewport_json: None,
                nodes: vec![
                    node_input(
                        "source",
                        "TextInputNode",
                        json!({"content": "雨夜里，角色推开仓库门。"}),
                    ),
                    node_input("target", target_kind, target_config),
                ],
                edges: vec![CanvasEdgeRecordInput {
                    source_client_id: "source".to_string(),
                    target_client_id: "target".to_string(),
                    source_port: "text".to_string(),
                    target_port: "input".to_string(),
                    role: "context".to_string(),
                    config_json: None,
                }],
            },
            chrono::Utc::now().naive_utc(),
        )
        .await
        .expect("测试画布应保存成功");
    (project.id, document)
}

fn node_input(client_id: &str, node_kind: &str, config_json: Value) -> CanvasNodeRecordInput {
    CanvasNodeRecordInput {
        client_id: client_id.to_string(),
        node_kind: node_kind.to_string(),
        ref_type: None,
        ref_id: None,
        config_json: Some(config_json),
        position_json: json!({"x": 0, "y": 0}),
        state: None,
    }
}

fn target_node(
    document: &CanvasGraphRecordDocument,
) -> &a4n_db_models::models::canvas::canvas_node::Model {
    document
        .nodes
        .iter()
        .find(|node| node.node_kind != "TextInputNode")
        .expect("目标节点应存在")
}

#[test]
fn canvas_text_input_and_render_rules_are_owned_by_orchestrator() {
    let now = chrono::Utc::now().naive_utc();
    let graph = a4n_db_models::models::canvas::canvas_graph::Model {
        id: 1,
        project_id: 2,
        name: "测试".to_string(),
        graph_kind: "video_studio".to_string(),
        version: 1,
        viewport_json: None,
        created_at: now,
        updated_at: now,
    };
    let source = a4n_db_models::models::canvas::canvas_node::Model {
        id: 10,
        graph_id: 1,
        node_kind: "TextInputNode".to_string(),
        ref_type: None,
        ref_id: None,
        config_json: json!({"content": "雨夜追逐"}),
        position_json: json!({"x": 0, "y": 0}),
        state: "idle".to_string(),
        created_at: now,
        updated_at: now,
    };
    let llm = a4n_db_models::models::canvas::canvas_node::Model {
        id: 11,
        graph_id: 1,
        node_kind: "LlmTextNode".to_string(),
        ref_type: None,
        ref_id: None,
        config_json: json!({
            "instruction": "改写为三条镜头脚本",
            "output_kind": GENERATION_PROMPT_KIND_CANVAS_SHOT_SCRIPT,
            "llm_config_id": 12,
            "temperature": 0.3,
            "max_tokens": 800,
        }),
        position_json: json!({"x": 0, "y": 0}),
        state: "idle".to_string(),
        created_at: now,
        updated_at: now,
    };
    let render = a4n_db_models::models::canvas::canvas_node::Model {
        id: 12,
        node_kind: "RenderTextNode".to_string(),
        config_json: json!({
            "template": "模板 {{context}}",
            "output_kind": GENERATION_PROMPT_KIND_VIDEO_PROMPT,
        }),
        ..llm.clone()
    };
    let source_id = source.id;
    let edge = |target_node_id| a4n_db_models::models::canvas::canvas_edge::Model {
        id: target_node_id,
        graph_id: 1,
        source_node_id: source_id,
        target_node_id,
        source_port: "text".to_string(),
        target_port: "input".to_string(),
        role: "context".to_string(),
        config_json: json!({}),
    };
    let document = CanvasGraphRecordDocument {
        graph,
        nodes: vec![source, llm.clone(), render.clone()],
        edges: vec![edge(llm.id), edge(render.id)],
        text_assets: Vec::new(),
    };

    let llm_input = build_canvas_llm_text_input(&llm, &document, None).expect("LLM 输入应构造成功");
    let request = build_canvas_llm_text_request_for_preview(2, 1, &llm, &document)
        .expect("LLM 请求预览应复用执行请求构造器");
    let rendered =
        build_canvas_rendered_prompt(&render, &document, None).expect("Render 输入应构造成功");

    assert_eq!(llm_input.config_id, Some(12));
    assert_eq!(
        llm_input.output_kind,
        GENERATION_PROMPT_KIND_CANVAS_SHOT_SCRIPT
    );
    assert_eq!(llm_input.max_tokens, Some(800));
    assert!(llm_input.user_prompt.contains("雨夜追逐"));
    assert_eq!(request.max_tokens, Some(800));
    let trace = request.trace_context.expect("预览请求应携带 trace 上下文");
    assert_eq!(trace.workflow_run_id, None);
    assert_eq!(trace.step_key, "canvas_node:11");
    assert_eq!(rendered.prompt_kind, GENERATION_PROMPT_KIND_VIDEO_PROMPT);
    assert_eq!(rendered.prompt_text, "模板 雨夜追逐");
}

#[test]
fn canvas_llm_error_mapping_preserves_cancellation() {
    assert!(matches!(
        map_canvas_llm_error(AppError::Cancelled),
        AppError::Cancelled
    ));
    assert!(matches!(
        map_canvas_llm_error(AppError::LlmError("provider unavailable".to_string())),
        AppError::LlmError(message) if message.contains("provider unavailable")
    ));
}

#[test]
fn restored_checkpoint_rejects_mismatch() {
    let mismatch = validate_restored_checkpoint_for_test(
        json!({"text": "旧文本"}),
        "新文本",
        "canvas_node:1:llm_transform",
    )
    .expect_err("parsed_output 不一致必须失败");
    assert!(
        mismatch
            .to_string()
            .contains("checkpoint_parsed_output_mismatch")
    );
}

#[tokio::test]
async fn llm_text_application_persists_complete_trace() {
    let (database, context) = setup_context().await;
    let (project_id, document) = insert_canvas(
        database.as_ref(),
        &context,
        "LlmTextNode",
        json!({
            "instruction": "改写为镜头脚本",
            "output_kind": GENERATION_PROMPT_KIND_CANVAS_SHOT_SCRIPT,
            "llm_config_id": 7,
        }),
    )
    .await;
    let node = target_node(&document);
    let port = TestLlmPort::success("第一镜：仓库门被推开。", Some("stop"));

    let summary = context
        .run_llm_text(
            CanvasLlmTextNodeApplicationInput {
                user_id: 1,
                project_id,
                graph_id: document.graph.id,
                node_id: node.id,
                shared_run: None,
                mode: "single_node".to_string(),
            },
            &port,
        )
        .await
        .expect("LLM 文本 application 应成功");

    assert_eq!(summary.status, "completed");
    assert_eq!(port.calls.load(Ordering::SeqCst), 1);
    let spec = generation_spec::Entity::find()
        .filter(generation_spec::Column::ProjectId.eq(project_id))
        .filter(generation_spec::Column::TargetEntityId.eq(node.id))
        .one(database.as_ref())
        .await
        .expect("应可查询 generation spec")
        .expect("generation spec 应存在");
    let artifact = prompt_artifact::Entity::find()
        .filter(prompt_artifact::Column::GenerationSpecId.eq(spec.id))
        .one(database.as_ref())
        .await
        .expect("应可查询 prompt artifact")
        .expect("prompt artifact 应存在");
    let run = workflow_run::Entity::find_by_id(summary.workflow_run_id)
        .one(database.as_ref())
        .await
        .expect("应可查询 workflow")
        .expect("workflow 应存在");
    let checkpoints = workflow_checkpoint::Entity::find()
        .filter(workflow_checkpoint::Column::WorkflowRunId.eq(run.id))
        .all(database.as_ref())
        .await
        .expect("应可查询 checkpoints");

    assert_eq!(spec.status, "completed");
    assert_eq!(artifact.prompt_text, "第一镜：仓库门被推开。");
    assert_eq!(
        artifact.prompt_kind,
        GENERATION_PROMPT_KIND_CANVAS_SHOT_SCRIPT
    );
    assert_eq!(run.status, "completed");
    assert_eq!(checkpoints.len(), 2);
    assert!(
        checkpoints
            .iter()
            .any(|row| row.step_key == format!("canvas_node:{}", node.id))
    );
    assert!(
        checkpoints
            .iter()
            .any(|row| row.step_key == format!("canvas_node:{}:llm_transform", node.id))
    );
}

#[tokio::test]
async fn llm_port_failure_marks_spec_and_workflow_failed() {
    let (database, context) = setup_context().await;
    let (project_id, document) = insert_canvas(
        database.as_ref(),
        &context,
        "LlmTextNode",
        json!({
            "instruction": "改写",
            "output_kind": GENERATION_PROMPT_KIND_CANVAS_SHOT_SCRIPT
        }),
    )
    .await;
    let node = target_node(&document);
    let port = TestLlmPort::failure("provider unavailable");

    let summary = context
        .run_llm_text(
            CanvasLlmTextNodeApplicationInput {
                user_id: 1,
                project_id,
                graph_id: document.graph.id,
                node_id: node.id,
                shared_run: None,
                mode: "single_node".to_string(),
            },
            &port,
        )
        .await
        .expect("业务失败应返回持久化后的 failed summary");

    let spec = generation_spec::Entity::find()
        .filter(generation_spec::Column::ProjectId.eq(project_id))
        .filter(generation_spec::Column::TargetEntityId.eq(node.id))
        .one(database.as_ref())
        .await
        .expect("应可查询 generation spec")
        .expect("generation spec 应存在");
    let run = workflow_run::Entity::find_by_id(summary.workflow_run_id)
        .one(database.as_ref())
        .await
        .expect("应可查询 workflow")
        .expect("workflow 应存在");

    assert_eq!(port.calls.load(Ordering::SeqCst), 1);
    assert_eq!(summary.status, "failed");
    assert!(
        summary
            .error_message
            .as_deref()
            .is_some_and(|message| message.contains("provider unavailable"))
    );
    assert_eq!(spec.status, "failed");
    assert_eq!(run.status, "failed");
}

#[tokio::test]
async fn fresh_truncated_llm_response_marks_failed_without_artifact() {
    let (database, context) = setup_context().await;
    let (project_id, document) = insert_canvas(
        database.as_ref(),
        &context,
        "LlmTextNode",
        json!({
            "instruction": "改写",
            "output_kind": GENERATION_PROMPT_KIND_CANVAS_SHOT_SCRIPT
        }),
    )
    .await;
    let node = target_node(&document);
    let port = TestLlmPort::success("尚未完成的输出", Some("length"));

    let summary = context
        .run_llm_text(
            CanvasLlmTextNodeApplicationInput {
                user_id: 1,
                project_id,
                graph_id: document.graph.id,
                node_id: node.id,
                shared_run: None,
                mode: "single_node".to_string(),
            },
            &port,
        )
        .await
        .expect("截断属于已持久化的业务失败");

    let spec = generation_spec::Entity::find()
        .filter(generation_spec::Column::ProjectId.eq(project_id))
        .filter(generation_spec::Column::TargetEntityId.eq(node.id))
        .one(database.as_ref())
        .await
        .expect("应可查询 generation spec")
        .expect("generation spec 应存在");
    let artifact_count = prompt_artifact::Entity::find()
        .filter(prompt_artifact::Column::GenerationSpecId.eq(spec.id))
        .all(database.as_ref())
        .await
        .expect("应可查询 prompt artifact")
        .len();

    assert_eq!(summary.status, "failed");
    assert!(
        summary
            .error_message
            .as_deref()
            .is_some_and(|message| message.contains("可能被截断"))
    );
    assert_eq!(spec.status, "failed");
    assert_eq!(artifact_count, 0);
}

#[tokio::test]
async fn render_text_application_persists_prompt_artifact() {
    let (database, context) = setup_context().await;
    let (project_id, document) = insert_canvas(
        database.as_ref(),
        &context,
        "RenderTextNode",
        json!({
            "template": "视频提示词：{{context}}",
            "output_kind": GENERATION_PROMPT_KIND_VIDEO_PROMPT,
        }),
    )
    .await;
    let node = target_node(&document);

    let summary = context
        .run_render_text(CanvasRenderTextNodeApplicationInput {
            user_id: 1,
            project_id,
            graph_id: document.graph.id,
            node_id: node.id,
            shared_run: None,
            mode: "single_node".to_string(),
        })
        .await
        .expect("Render 文本 application 应成功");

    let spec = generation_spec::Entity::find()
        .filter(generation_spec::Column::ProjectId.eq(project_id))
        .filter(generation_spec::Column::TargetEntityId.eq(node.id))
        .one(database.as_ref())
        .await
        .expect("应可查询 generation spec")
        .expect("generation spec 应存在");
    let artifact = prompt_artifact::Entity::find()
        .filter(prompt_artifact::Column::GenerationSpecId.eq(spec.id))
        .one(database.as_ref())
        .await
        .expect("应可查询 prompt artifact")
        .expect("prompt artifact 应存在");

    assert_eq!(summary.status, "completed");
    assert_eq!(spec.status, "completed");
    assert_eq!(artifact.prompt_kind, GENERATION_PROMPT_KIND_VIDEO_PROMPT);
    assert_eq!(artifact.prompt_text, "视频提示词：雨夜里，角色推开仓库门。");
}
