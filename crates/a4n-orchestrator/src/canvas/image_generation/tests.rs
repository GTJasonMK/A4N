use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

use a4n_canvas_contract::CanvasImageNodeApplicationInput;
use a4n_db::{
    DatabaseConnection,
    canvas::{
        CanvasEdgeRecordInput, CanvasGraphRecordDocument, CanvasGraphRecordInput,
        CanvasNodeRecordInput,
    },
    generation::GENERATION_SPEC_TARGET_KIND_CANVAS_IMAGE,
    project::{CreateFreeformVideoProjectParams, create_freeform_video_project_record},
};
use a4n_db_models::models::{
    ai::generated_image,
    generation::{generation_run, generation_spec, workflow_checkpoint, workflow_run},
    media::media_asset,
};
use a4n_domain::{image::IMAGE_TYPE_CANVAS, media::GeneratedMediaArtifactFile};
use a4n_error::AppError;
use a4n_provider_contract::ImageProviderConfig;
use async_trait::async_trait;
use base64::Engine as _;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
use serde_json::{Value, json};
use uuid::Uuid;

use crate::{
    OrchestratorContext, canvas::CanvasNodeRunHandle, test_support::TestOrchestratorBuilder,
};

use super::{
    CanvasImageNodeApplicationPort, CanvasImageNodeExecutionInput,
    build_canvas_image_node_input_for_preview,
};
use crate::media::{
    ImageGenerationHostPort, ImageGenerationProgress, ImageGenerationProviderInput,
    ImageGenerationProviderOutput, PersistGeneratedImageFileInput, PersistedGeneratedImageFile,
    PrepareGeneratedImageFileInput, PreparedGeneratedImageFile,
};

const PNG_1X1_B64: &str =
    "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mNkYAAAAAYAAjCB0C8AAAAASUVORK5CYII=";

fn png_1x1_bytes() -> Vec<u8> {
    base64::engine::general_purpose::STANDARD
        .decode(PNG_1X1_B64)
        .expect("测试 PNG 应有效")
}

struct TestImageHost {
    calls: AtomicUsize,
    fail_provider: bool,
}

impl TestImageHost {
    fn success() -> Self {
        Self {
            calls: AtomicUsize::new(0),
            fail_provider: false,
        }
    }

    fn failure() -> Self {
        Self {
            calls: AtomicUsize::new(0),
            fail_provider: true,
        }
    }
}

#[async_trait]
impl ImageGenerationHostPort for TestImageHost {
    async fn resolve_image_config(
        &self,
        _config_id: Option<i64>,
    ) -> Result<ImageProviderConfig, AppError> {
        Ok(image_config())
    }

    fn runtime_config_snapshot(&self) -> Option<Value> {
        Some(json!({"profile": "canvas-image-test"}))
    }

    fn read_reference_image(
        &self,
        label: &str,
        _relative_path: &str,
    ) -> Result<a4n_image::ImageReference, AppError> {
        Ok(a4n_image::ImageReference {
            label: label.to_string(),
            data_uri: "data:image/png;base64,AQ==".to_string(),
        })
    }

    async fn generate_image(
        &self,
        _input: ImageGenerationProviderInput,
        _progress: &ImageGenerationProgress<'_>,
    ) -> Result<ImageGenerationProviderOutput, AppError> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        if self.fail_provider {
            return Err(AppError::InvalidState {
                current: "image_provider".to_string(),
                operation: "canvas image provider unavailable".to_string(),
            });
        }
        Ok(ImageGenerationProviderOutput {
            bytes: png_1x1_bytes(),
            format: "png".to_string(),
            model: "canvas-image-test-model".to_string(),
        })
    }

    fn prepare_generated_image_file(
        &self,
        input: PrepareGeneratedImageFileInput<'_>,
    ) -> Result<PreparedGeneratedImageFile, AppError> {
        let final_path = format!(
            "projects/{}/images/{}_{}_{}.{}",
            input.project_id,
            input.image_type,
            input.file_extra.unwrap_or("main"),
            input.publication_id,
            input.format
        );
        Ok(PreparedGeneratedImageFile {
            staging_path: format!("{final_path}.part"),
            final_path,
        })
    }

    fn persist_generated_image_file(
        &self,
        input: PersistGeneratedImageFileInput<'_>,
        prepared: &PreparedGeneratedImageFile,
    ) -> Result<PersistedGeneratedImageFile, AppError> {
        Ok(PersistedGeneratedImageFile {
            relative_path: prepared.final_path.clone(),
            file_size: input.bytes.len() as i64,
        })
    }

    fn validate_generated_image_file(
        &self,
        _relative_path: &str,
        _label: &str,
    ) -> Result<(), AppError> {
        Ok(())
    }

    fn inspect_generated_image_file(
        &self,
        _relative_path: &str,
        _label: &str,
    ) -> Result<GeneratedMediaArtifactFile, AppError> {
        Ok(GeneratedMediaArtifactFile {
            content_type: "image/png".to_string(),
            sha256: "test-sha256".to_string(),
            byte_size: png_1x1_bytes().len() as u64,
        })
    }

    async fn remove_generated_image_file(
        &self,
        _relative_path: &str,
        _label: &str,
    ) -> Result<(), AppError> {
        Ok(())
    }
}

fn image_config() -> ImageProviderConfig {
    ImageProviderConfig {
        id: 7,
        name: "Canvas 图片测试配置".to_string(),
        backend: "test-image-provider".to_string(),
        base_url: Some("http://127.0.0.1:47645".to_string()),
        api_key: None,
        model: "configured-canvas-image-model".to_string(),
        max_concurrent: 1,
        supported_sizes_json: json!([{"width": 1024, "height": 576}]),
    }
}

async fn setup_context() -> (Arc<DatabaseConnection>, OrchestratorContext, i64) {
    let database = Arc::new(a4n_migration::test_support::connect_test_database().await);
    let suffix = Uuid::new_v4().simple().to_string();
    let user = a4n_db::auth::create_user(
        database.as_ref(),
        &format!("canvas-image-test-{suffix}"),
        &format!("canvas-image-test-{suffix}@example.test"),
        "test-password-123",
        "user",
    )
    .await
    .expect("Canvas 图片测试用户应创建成功");
    let context = TestOrchestratorBuilder::new(Arc::clone(&database))
        .build()
        .await;
    (database, context, user.id)
}

async fn insert_canvas(
    database: &DatabaseConnection,
    context: &OrchestratorContext,
    user_id: i64,
    target_config: Value,
) -> (i64, CanvasGraphRecordDocument) {
    let project = create_freeform_video_project_record(
        database,
        user_id,
        CreateFreeformVideoProjectParams {
            title: "Canvas Image application 测试".to_string(),
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
                name: Some("图片生成测试画布".to_string()),
                graph_kind: Some("video_studio".to_string()),
                viewport_json: None,
                nodes: vec![
                    node_input(
                        "source",
                        "TextInputNode",
                        json!({"content": "雨夜仓库入口，电影感构图"}),
                    ),
                    node_input("target", "GenerateImageNode", target_config),
                ],
                edges: vec![CanvasEdgeRecordInput {
                    source_client_id: "source".to_string(),
                    target_client_id: "target".to_string(),
                    source_port: "text".to_string(),
                    target_port: "text".to_string(),
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
        .find(|node| node.node_kind == "GenerateImageNode")
        .expect("图片节点应存在")
}

#[test]
fn canvas_image_input_rules_are_owned_by_orchestrator() {
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
        config_json: json!({"content": "画布图片提示词"}),
        position_json: json!({"x": 0, "y": 0}),
        state: "idle".to_string(),
        created_at: now,
        updated_at: now,
    };
    let image = a4n_db_models::models::canvas::canvas_node::Model {
        id: 11,
        graph_id: 1,
        node_kind: "GenerateImageNode".to_string(),
        ref_type: None,
        ref_id: None,
        config_json: json!({"image_config_id": 7, "config_id": 99, "prompt": "隐藏提示词", "width": 1024, "height": 576}),
        position_json: json!({"x": 0, "y": 0}),
        state: "idle".to_string(),
        created_at: now,
        updated_at: now,
    };
    let document = CanvasGraphRecordDocument {
        graph,
        nodes: vec![source, image.clone()],
        edges: vec![a4n_db_models::models::canvas::canvas_edge::Model {
            id: 1,
            graph_id: 1,
            source_node_id: 10,
            target_node_id: image.id,
            source_port: "text".to_string(),
            target_port: "text".to_string(),
            role: "context".to_string(),
            config_json: json!({}),
        }],
        text_assets: Vec::new(),
    };

    let input = build_canvas_image_node_input_for_preview(&image, &document)
        .expect("图片节点输入应构造成功");

    assert_eq!(input.prompt, "画布图片提示词");
    assert_eq!(input.config_id, Some(7));
    assert_eq!(input.source_node_ids, vec![10]);
}

#[test]
fn canvas_image_input_rejects_inline_prompt_and_unresolved_operation_source() {
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
    let image = a4n_db_models::models::canvas::canvas_node::Model {
        id: 11,
        graph_id: 1,
        node_kind: "GenerateImageNode".to_string(),
        ref_type: None,
        ref_id: None,
        config_json: json!({"prompt": "隐藏提示词", "width": 1024, "height": 576}),
        position_json: json!({"x": 0, "y": 0}),
        state: "idle".to_string(),
        created_at: now,
        updated_at: now,
    };
    let empty = CanvasGraphRecordDocument {
        graph: graph.clone(),
        nodes: vec![image.clone()],
        edges: Vec::new(),
        text_assets: Vec::new(),
    };
    let inline_error = build_canvas_image_node_input_for_preview(&image, &empty)
        .expect_err("内联 prompt 不能作为操作输入");
    assert!(inline_error.contains("缺少 prompt 输入"));

    let source = a4n_db_models::models::canvas::canvas_node::Model {
        id: 10,
        node_kind: "RenderTextNode".to_string(),
        config_json: json!({"content": "旧结果"}),
        ..image.clone()
    };
    let unresolved = CanvasGraphRecordDocument {
        graph,
        nodes: vec![source, image.clone()],
        edges: vec![a4n_db_models::models::canvas::canvas_edge::Model {
            id: 1,
            graph_id: 1,
            source_node_id: 10,
            target_node_id: image.id,
            source_port: "text".to_string(),
            target_port: "text".to_string(),
            role: "context".to_string(),
            config_json: json!({}),
        }],
        text_assets: Vec::new(),
    };
    let checkpoint_error = build_canvas_image_node_input_for_preview(&image, &unresolved)
        .expect_err("未运行的上游操作节点必须失败");
    assert!(checkpoint_error.contains("当前 workflow checkpoint"));
}

#[tokio::test]
async fn canvas_image_application_persists_complete_artifact_chain() {
    let (database, context, user_id) = setup_context().await;
    let (project_id, document) = insert_canvas(
        database.as_ref(),
        &context,
        user_id,
        json!({"image_config_id": 7, "width": 1024, "height": 576}),
    )
    .await;
    let node = target_node(&document);
    let host = TestImageHost::success();

    let summary = context
        .run_image(
            CanvasImageNodeApplicationInput {
                user_id,
                project_id,
                graph_id: document.graph.id,
                node_id: node.id,
                shared_run: None,
                mode: "single_node".to_string(),
            },
            &host,
            &ImageGenerationProgress::none(),
        )
        .await
        .expect("Canvas 图片 application 应成功");

    let spec = generation_spec::Entity::find()
        .filter(generation_spec::Column::ProjectId.eq(project_id))
        .filter(generation_spec::Column::TargetEntityId.eq(node.id))
        .one(database.as_ref())
        .await
        .expect("应可查询 generation spec")
        .expect("generation spec 应存在");
    let image = generated_image::Entity::find()
        .filter(generated_image::Column::ProjectId.eq(project_id))
        .one(database.as_ref())
        .await
        .expect("应可查询 generated image")
        .expect("generated image 应存在");
    let generation_runs = generation_run::Entity::find()
        .filter(generation_run::Column::ProjectId.eq(project_id))
        .all(database.as_ref())
        .await
        .expect("应可查询 generation runs");
    let assets = media_asset::Entity::find()
        .filter(media_asset::Column::ProjectId.eq(project_id))
        .all(database.as_ref())
        .await
        .expect("应可查询 media assets");
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

    assert_eq!(summary.status, "completed");
    assert_eq!(host.calls.load(Ordering::SeqCst), 1);
    assert_eq!(spec.status, "completed");
    assert_eq!(spec.target_kind, GENERATION_SPEC_TARGET_KIND_CANVAS_IMAGE);
    assert_eq!(image.image_type, IMAGE_TYPE_CANVAS);
    assert_eq!(generation_runs.len(), 1);
    assert_eq!(assets.len(), 1);
    assert_eq!(run.status, "completed");
    assert!(checkpoints.iter().any(|row| {
        row.step_key == format!("canvas_node:{}", node.id) && row.status == "completed"
    }));
    assert!(checkpoints.iter().any(|row| {
        row.step_key.contains("generate_canvas_image") && row.status == "completed"
    }));
}

#[tokio::test]
async fn canvas_image_shared_run_completes_node_without_finalizing_graph_workflow() {
    let (database, context, user_id) = setup_context().await;
    let (project_id, document) = insert_canvas(
        database.as_ref(),
        &context,
        user_id,
        json!({"image_config_id": 7, "width": 1024, "height": 576}),
    )
    .await;
    let node = target_node(&document);
    let step_key = format!("canvas_node:{}", node.id);
    let graph_run = context
        .begin_canvas_graph_workflow_validated(
            project_id,
            document.graph.id,
            "full_graph",
            chrono::Utc::now().timestamp_micros(),
            json!({
                "project_id": project_id,
                "graph_id": document.graph.id,
                "runnable_node_ids": [node.id],
            }),
            Some(step_key.clone()),
        )
        .await
        .expect("共享画布 workflow 应创建成功");
    let host = TestImageHost::success();

    let summary = context
        .run_canvas_image_node_validated(
            CanvasImageNodeExecutionInput {
                requested_by_user_id: user_id,
                project_id,
                graph_id: document.graph.id,
                node,
                document: &document,
                shared_run: Some(CanvasNodeRunHandle {
                    workflow_run_id: graph_run.id,
                    finalize_on_success: false,
                }),
                mode: "full_graph",
            },
            &host,
            &ImageGenerationProgress::none(),
        )
        .await
        .expect("共享 workflow 中的 Canvas 图片节点应成功");

    let persisted_run = workflow_run::Entity::find_by_id(graph_run.id)
        .one(database.as_ref())
        .await
        .expect("应可查询共享 workflow")
        .expect("共享 workflow 应存在");
    let node_checkpoint = workflow_checkpoint::Entity::find()
        .filter(workflow_checkpoint::Column::WorkflowRunId.eq(graph_run.id))
        .filter(workflow_checkpoint::Column::StepKey.eq(step_key))
        .one(database.as_ref())
        .await
        .expect("应可查询节点 checkpoint")
        .expect("节点 checkpoint 应存在");

    assert_eq!(summary.workflow_run_id, graph_run.id);
    assert_eq!(summary.status, "completed");
    assert_eq!(persisted_run.status, "running");
    assert_eq!(node_checkpoint.status, "completed");
    assert_eq!(host.calls.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn canvas_image_provider_failure_marks_spec_and_workflow_failed() {
    let (database, context, user_id) = setup_context().await;
    let (project_id, document) = insert_canvas(
        database.as_ref(),
        &context,
        user_id,
        json!({"width": 1024, "height": 576}),
    )
    .await;
    let node = target_node(&document);
    let host = TestImageHost::failure();

    let summary = context
        .run_canvas_image_node_validated(
            CanvasImageNodeExecutionInput {
                requested_by_user_id: 1,
                project_id,
                graph_id: document.graph.id,
                node,
                document: &document,
                shared_run: None,
                mode: "single_node",
            },
            &host,
            &ImageGenerationProgress::none(),
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

    assert_eq!(host.calls.load(Ordering::SeqCst), 1);
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
