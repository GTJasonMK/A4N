use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

use a4n_core::{
    error::AppError,
    models::{
        ai::{generated_image, generated_video},
        generation::{generation_spec, workflow_checkpoint, workflow_run},
        media::media_asset,
        video_project::{assembly_clip, assembly_timeline},
    },
    video_workflow::{
        WORKFLOW_MODE_CANVAS_ASSEMBLY, WORKFLOW_MODE_CANVAS_TRIM, WORKFLOW_MODE_CANVAS_VIDEO,
    },
};
use a4n_db::{
    DatabaseConnection,
    canvas::{
        CanvasEdgeRecordInput, CanvasGraphRecordDocument, CanvasGraphRecordInput,
        CanvasNodeRecordInput,
    },
    generation::{
        GENERATION_SPEC_TARGET_KIND_CANVAS_ASSEMBLY, GENERATION_SPEC_TARGET_KIND_CANVAS_IMAGE,
        GENERATION_SPEC_TARGET_KIND_CANVAS_VIDEO,
    },
    project::{CreateFreeformVideoProjectParams, create_freeform_video_project_record},
};
use a4n_domain::image::IMAGE_TYPE_CANVAS_FRAME;
use async_trait::async_trait;
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, QueryOrder, Set};
use serde_json::{Value, json};

use crate::{
    OrchestratorContext, canvas::CanvasNodeRunHandle, test_support::TestOrchestratorBuilder,
};

use super::{
    CanvasAssembleVideosHostInput, CanvasEditingNodeExecutionInput, CanvasEditingNodeHostPort,
    CanvasEditingNodeInput, CanvasExtractFrameHostInput, CanvasGeneratedImageArtifact,
    CanvasGeneratedVideoArtifact, CanvasTrimVideoHostInput,
    build_canvas_editing_node_input_for_preview,
};

struct TestEditingHost {
    database: Arc<DatabaseConnection>,
    calls: AtomicUsize,
    fail_operation: Option<&'static str>,
    cancel_trim: bool,
}

impl TestEditingHost {
    fn success(database: Arc<DatabaseConnection>) -> Self {
        Self {
            database,
            calls: AtomicUsize::new(0),
            fail_operation: None,
            cancel_trim: false,
        }
    }

    fn failure(database: Arc<DatabaseConnection>, operation: &'static str) -> Self {
        Self {
            database,
            calls: AtomicUsize::new(0),
            fail_operation: Some(operation),
            cancel_trim: false,
        }
    }

    fn cancelling_trim(database: Arc<DatabaseConnection>) -> Self {
        Self {
            database,
            calls: AtomicUsize::new(0),
            fail_operation: None,
            cancel_trim: true,
        }
    }

    fn operation_error(&self, operation: &'static str) -> Result<(), AppError> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        if self.fail_operation == Some(operation) {
            return Err(AppError::InvalidState {
                current: "editing_host".to_string(),
                operation: format!("{operation} unavailable"),
            });
        }
        Ok(())
    }
}

#[async_trait]
impl CanvasEditingNodeHostPort for TestEditingHost {
    async fn extract_frame(
        &self,
        input: CanvasExtractFrameHostInput,
    ) -> Result<CanvasGeneratedImageArtifact, AppError> {
        self.operation_error("extract_frame")?;
        let image = insert_generated_image(
            self.database.as_ref(),
            input.project_id,
            "projects/output/extracted-frame.png",
        )
        .await;
        Ok(CanvasGeneratedImageArtifact {
            generated_image_id: image.id,
        })
    }

    async fn trim_video(
        &self,
        input: CanvasTrimVideoHostInput,
    ) -> Result<CanvasGeneratedVideoArtifact, AppError> {
        self.operation_error("trim_video")?;
        if self.cancel_trim {
            a4n_db::workflow::cancel_workflow_run_persistence(
                self.database.as_ref(),
                input.workflow_run_id,
                "test cancellation",
            )
            .await
            .expect("测试 workflow 应可取消");
            return Err(AppError::Cancelled);
        }
        let video = insert_generated_video(
            self.database.as_ref(),
            input.project_id,
            generated_video::VIDEO_TYPE_TRIMMED_CLIP,
            WORKFLOW_MODE_CANVAS_TRIM,
            "projects/output/trimmed-video.mp4",
        )
        .await;
        Ok(CanvasGeneratedVideoArtifact {
            generated_video_id: video.id,
        })
    }

    async fn assemble_videos(
        &self,
        input: CanvasAssembleVideosHostInput,
    ) -> Result<CanvasGeneratedVideoArtifact, AppError> {
        self.operation_error("assemble_videos")?;
        let video = insert_generated_video(
            self.database.as_ref(),
            input.project_id,
            generated_video::VIDEO_TYPE_CANVAS_ASSEMBLY,
            WORKFLOW_MODE_CANVAS_ASSEMBLY,
            "projects/output/assembled-video.mp4",
        )
        .await;
        Ok(CanvasGeneratedVideoArtifact {
            generated_video_id: video.id,
        })
    }
}

#[test]
fn canvas_editing_input_rules_are_owned_by_orchestrator() {
    let document = test_document_for_input_rules();
    let assemble = document
        .nodes
        .iter()
        .find(|node| node.node_kind == "AssembleVideoNode")
        .expect("assemble node 应存在");

    let input = build_canvas_editing_node_input_for_preview(assemble, &document)
        .expect("assemble 输入应构造成功");

    assert_eq!(
        input,
        CanvasEditingNodeInput::AssembleVideo(super::CanvasAssembleVideoNodeInput {
            video_ids: vec![92, 91],
            title: Some("组装预览".to_string()),
            canvas_text_asset_id: None,
            source_node_ids: vec![3, 1],
        })
    );

    let mut hidden = assemble.clone();
    hidden.config_json = json!({"video_ids": [91]});
    let error = build_canvas_editing_node_input_for_preview(&hidden, &document)
        .expect_err("隐藏 video_ids 必须被拒绝");
    assert!(error.contains("不支持 config_json.video_ids"));
}

#[tokio::test]
async fn extract_frame_application_persists_spec_asset_checkpoint_and_workflow() {
    let setup = setup_canvas("ExtractFrameNode", json!({"frame_kind": "tail"}), 1).await;
    let host = TestEditingHost::success(Arc::clone(&setup.database));

    let summary = run_node(&setup, &host, None, "single_node")
        .await
        .expect("抽帧 application 应成功");

    let (spec, run, checkpoint) = persisted_lifecycle(&setup, summary.workflow_run_id).await;
    let image = generated_image::Entity::find()
        .filter(generated_image::Column::ProjectId.eq(setup.project_id))
        .one(setup.database.as_ref())
        .await
        .expect("应可查询抽帧图片")
        .expect("抽帧图片应存在");
    let asset = media_asset::Entity::find()
        .filter(media_asset::Column::ProjectId.eq(setup.project_id))
        .one(setup.database.as_ref())
        .await
        .expect("应可查询抽帧媒体资产")
        .expect("抽帧媒体资产应存在");

    assert_eq!(summary.status, "completed");
    assert_eq!(host.calls.load(Ordering::SeqCst), 1);
    assert_eq!(spec.target_kind, GENERATION_SPEC_TARGET_KIND_CANVAS_IMAGE);
    assert_eq!(spec.status, "completed");
    assert_eq!(run.status, "completed");
    assert_eq!(checkpoint.status, "completed");
    assert_eq!(
        checkpoint.artifacts_json.as_ref().unwrap()["generated_image_id"],
        image.id
    );
    assert_eq!(
        checkpoint.artifacts_json.as_ref().unwrap()["media_asset_id"],
        asset.id
    );
    assert_eq!(asset.origin, "canvas_extracted_frame");
}

#[tokio::test]
async fn trim_video_application_persists_video_asset_and_completed_lifecycle() {
    let setup = setup_canvas(
        "TrimVideoNode",
        json!({"start_secs": 0.5, "end_secs": 2.5}),
        1,
    )
    .await;
    let host = TestEditingHost::success(Arc::clone(&setup.database));

    let summary = run_node(&setup, &host, None, "single_node")
        .await
        .expect("裁剪 application 应成功");

    let (spec, run, checkpoint) = persisted_lifecycle(&setup, summary.workflow_run_id).await;
    let asset = media_asset::Entity::find()
        .filter(media_asset::Column::ProjectId.eq(setup.project_id))
        .one(setup.database.as_ref())
        .await
        .expect("应可查询裁剪媒体资产")
        .expect("裁剪媒体资产应存在");

    assert_eq!(summary.status, "completed");
    assert_eq!(spec.target_kind, GENERATION_SPEC_TARGET_KIND_CANVAS_VIDEO);
    assert_eq!(spec.status, "completed");
    assert_eq!(run.status, "completed");
    assert_eq!(checkpoint.status, "completed");
    assert_eq!(asset.origin, "canvas_trim");
}

#[tokio::test]
async fn trim_video_cancellation_marks_spec_and_checkpoint_cancelled() {
    let setup = setup_canvas(
        "TrimVideoNode",
        json!({"start_secs": 0.0, "end_secs": 2.0}),
        1,
    )
    .await;
    let host = TestEditingHost::cancelling_trim(Arc::clone(&setup.database));

    let summary = run_node(&setup, &host, None, "single_node")
        .await
        .expect("取消应返回 cancelled summary");

    let (spec, run, checkpoint) = persisted_lifecycle(&setup, summary.workflow_run_id).await;
    assert_eq!(summary.status, "cancelled");
    assert_eq!(summary.error_message.as_deref(), Some("test cancellation"));
    assert_eq!(spec.status, "cancelled");
    assert_eq!(run.status, "cancelled");
    assert_eq!(checkpoint.status, "cancelled");
    assert_eq!(
        checkpoint.error_message.as_deref(),
        Some("test cancellation")
    );
}

#[tokio::test]
async fn assemble_application_persists_preview_asset_timeline_and_ordered_clips() {
    let setup = setup_canvas("AssembleVideoNode", json!({"title": "测试成片"}), 2).await;
    let host = TestEditingHost::success(Arc::clone(&setup.database));

    let summary = run_node(&setup, &host, None, "single_node")
        .await
        .expect("成片 application 应成功");

    let (spec, run, checkpoint) = persisted_lifecycle(&setup, summary.workflow_run_id).await;
    let timeline = assembly_timeline::Entity::find()
        .filter(assembly_timeline::Column::ProjectId.eq(setup.project_id))
        .one(setup.database.as_ref())
        .await
        .expect("应可查询 assembly timeline")
        .expect("assembly timeline 应存在");
    let clips = assembly_clip::Entity::find()
        .filter(assembly_clip::Column::TimelineId.eq(timeline.id))
        .order_by_asc(assembly_clip::Column::OrderIndex)
        .all(setup.database.as_ref())
        .await
        .expect("应可查询 assembly clips");
    let assets = media_asset::Entity::find()
        .filter(media_asset::Column::ProjectId.eq(setup.project_id))
        .all(setup.database.as_ref())
        .await
        .expect("应可查询 assembly assets");

    assert_eq!(summary.status, "completed");
    assert_eq!(
        spec.target_kind,
        GENERATION_SPEC_TARGET_KIND_CANVAS_ASSEMBLY
    );
    assert_eq!(spec.status, "completed");
    assert_eq!(run.status, "completed");
    assert_eq!(checkpoint.status, "completed");
    assert_eq!(clips.len(), 2);
    assert_eq!(clips[0].order_index, 1);
    assert_eq!(clips[1].order_index, 2);
    assert_eq!(assets.len(), 3);
    assert_eq!(
        timeline.preview_media_asset_id,
        checkpoint.artifacts_json.as_ref().unwrap()["media_asset_id"].as_i64()
    );
}

#[tokio::test]
async fn editing_host_failure_marks_spec_workflow_and_checkpoint_failed() {
    let setup = setup_canvas("ExtractFrameNode", json!({"frame_kind": "first"}), 1).await;
    let host = TestEditingHost::failure(Arc::clone(&setup.database), "extract_frame");

    let summary = run_node(&setup, &host, None, "single_node")
        .await
        .expect("业务失败应返回 failed summary");

    let (spec, run, checkpoint) = persisted_lifecycle(&setup, summary.workflow_run_id).await;
    assert_eq!(summary.status, "failed");
    assert!(
        summary
            .error_message
            .as_deref()
            .is_some_and(|message| message.contains("extract_frame unavailable"))
    );
    assert_eq!(spec.status, "failed");
    assert_eq!(run.status, "failed");
    assert_eq!(checkpoint.status, "failed");
}

#[tokio::test]
async fn shared_graph_run_completes_editing_node_without_finalizing_workflow() {
    let setup = setup_canvas("ExtractFrameNode", json!({"frame_kind": "first"}), 1).await;
    let host = TestEditingHost::success(Arc::clone(&setup.database));
    let step_key = format!("canvas_node:{}", setup.node.id);
    let graph_run = setup
        .context
        .begin_canvas_graph_workflow_validated(
            setup.project_id,
            setup.document.graph.id,
            "full_graph",
            chrono::Utc::now().timestamp_micros(),
            json!({
                "project_id": setup.project_id,
                "graph_id": setup.document.graph.id,
                "runnable_node_ids": [setup.node.id],
            }),
            Some(step_key.clone()),
        )
        .await
        .expect("共享 graph workflow 应创建成功");

    let summary = run_node(
        &setup,
        &host,
        Some(CanvasNodeRunHandle {
            workflow_run_id: graph_run.id,
            finalize_on_success: false,
        }),
        "full_graph",
    )
    .await
    .expect("共享 workflow 中编辑节点应成功");

    let persisted_run = workflow_run::Entity::find_by_id(graph_run.id)
        .one(setup.database.as_ref())
        .await
        .expect("应可查询共享 workflow")
        .expect("共享 workflow 应存在");
    let checkpoint = workflow_checkpoint::Entity::find()
        .filter(workflow_checkpoint::Column::WorkflowRunId.eq(graph_run.id))
        .filter(workflow_checkpoint::Column::StepKey.eq(step_key))
        .one(setup.database.as_ref())
        .await
        .expect("应可查询共享节点 checkpoint")
        .expect("共享节点 checkpoint 应存在");

    assert_eq!(summary.workflow_run_id, graph_run.id);
    assert_eq!(summary.status, "completed");
    assert_eq!(persisted_run.status, "running");
    assert_eq!(checkpoint.status, "completed");
}

struct CanvasSetup {
    database: Arc<DatabaseConnection>,
    context: OrchestratorContext,
    project_id: i64,
    document: CanvasGraphRecordDocument,
    node: a4n_core::models::canvas::canvas_node::Model,
}

async fn setup_canvas(node_kind: &str, config: Value, source_count: usize) -> CanvasSetup {
    let database = Arc::new(a4n_migration::test_support::connect_test_database().await);
    let context = TestOrchestratorBuilder::new(Arc::clone(&database))
        .build()
        .await;
    let project = create_freeform_video_project_record(
        database.as_ref(),
        1,
        CreateFreeformVideoProjectParams {
            title: format!("Canvas {node_kind} application 测试"),
            description: String::new(),
        },
    )
    .await
    .expect("测试项目应创建成功");
    let mut source_videos = Vec::new();
    for index in 0..source_count {
        source_videos.push(
            insert_generated_video(
                database.as_ref(),
                project.id,
                "canvas_source",
                WORKFLOW_MODE_CANVAS_VIDEO,
                &format!("projects/output/source-{index}.mp4"),
            )
            .await,
        );
    }
    let mut nodes = source_videos
        .iter()
        .enumerate()
        .map(|(index, video)| CanvasNodeRecordInput {
            client_id: format!("source-{index}"),
            node_kind: "VideoInputNode".to_string(),
            ref_type: Some("generated_video".to_string()),
            ref_id: Some(video.id),
            config_json: Some(json!({})),
            position_json: json!({"x": 0, "y": index}),
            state: None,
        })
        .collect::<Vec<_>>();
    nodes.push(CanvasNodeRecordInput {
        client_id: "target".to_string(),
        node_kind: node_kind.to_string(),
        ref_type: None,
        ref_id: None,
        config_json: Some(config),
        position_json: json!({"x": 1, "y": 0}),
        state: None,
    });
    let edges = source_videos
        .iter()
        .enumerate()
        .map(|(index, _)| CanvasEdgeRecordInput {
            source_client_id: format!("source-{index}"),
            target_client_id: "target".to_string(),
            source_port: "video".to_string(),
            target_port: if node_kind == "AssembleVideoNode" {
                "clip".to_string()
            } else {
                "video".to_string()
            },
            role: if node_kind == "AssembleVideoNode" {
                "assembly_clip".to_string()
            } else {
                "source_video".to_string()
            },
            config_json: (node_kind == "AssembleVideoNode")
                .then(|| json!({"order_index": index + 1})),
        })
        .collect();
    let document = context
        .save_canvas_graph_record(
            project.id,
            CanvasGraphRecordInput {
                id: None,
                expected_version: None,
                name: Some(format!("{node_kind} 测试画布")),
                graph_kind: Some("video_studio".to_string()),
                viewport_json: None,
                nodes,
                edges,
            },
            chrono::Utc::now().naive_utc(),
        )
        .await
        .expect("测试画布应保存成功");
    let node = document
        .nodes
        .iter()
        .find(|node| node.node_kind == node_kind)
        .expect("目标节点应存在")
        .clone();
    CanvasSetup {
        database,
        context,
        project_id: project.id,
        document,
        node,
    }
}

async fn run_node(
    setup: &CanvasSetup,
    host: &dyn CanvasEditingNodeHostPort,
    shared_run: Option<CanvasNodeRunHandle>,
    mode: &str,
) -> Result<a4n_api_contract::canvas::CanvasRunSummary, AppError> {
    setup
        .context
        .run_canvas_editing_node_validated(
            CanvasEditingNodeExecutionInput {
                user_id: 1,
                project_id: setup.project_id,
                graph_id: setup.document.graph.id,
                node: &setup.node,
                document: &setup.document,
                shared_run,
                mode,
            },
            host,
        )
        .await
}

async fn persisted_lifecycle(
    setup: &CanvasSetup,
    workflow_run_id: i64,
) -> (
    generation_spec::Model,
    workflow_run::Model,
    workflow_checkpoint::Model,
) {
    let spec = generation_spec::Entity::find()
        .filter(generation_spec::Column::ProjectId.eq(setup.project_id))
        .filter(generation_spec::Column::TargetEntityId.eq(setup.node.id))
        .one(setup.database.as_ref())
        .await
        .expect("应可查询 generation spec")
        .expect("generation spec 应存在");
    let run = workflow_run::Entity::find_by_id(workflow_run_id)
        .one(setup.database.as_ref())
        .await
        .expect("应可查询 workflow")
        .expect("workflow 应存在");
    let checkpoint = workflow_checkpoint::Entity::find()
        .filter(workflow_checkpoint::Column::WorkflowRunId.eq(workflow_run_id))
        .filter(workflow_checkpoint::Column::StepKey.eq(format!("canvas_node:{}", setup.node.id)))
        .one(setup.database.as_ref())
        .await
        .expect("应可查询节点 checkpoint")
        .expect("节点 checkpoint 应存在");
    (spec, run, checkpoint)
}

async fn insert_generated_image(
    database: &DatabaseConnection,
    project_id: i64,
    file_path: &str,
) -> generated_image::Model {
    generated_image::ActiveModel {
        project_id: Set(project_id),
        image_type: Set(IMAGE_TYPE_CANVAS_FRAME.to_string()),
        file_path: Set(file_path.to_string()),
        prompt: Set("测试抽帧".to_string()),
        model: Set("test-host".to_string()),
        width: Set(1024),
        height: Set(576),
        file_size: Set(Some(4)),
        ai_score: Set(None),
        is_favorite: Set(false),
        chapter_id: Set(None),
        panel_id: Set(None),
        metadata_json: Set(None),
        created_at: Set(chrono::Utc::now().naive_utc()),
        ..Default::default()
    }
    .insert(database)
    .await
    .expect("生成图片应创建成功")
}

async fn insert_generated_video(
    database: &DatabaseConnection,
    project_id: i64,
    video_type: &str,
    workflow_mode: &str,
    file_path: &str,
) -> generated_video::Model {
    let now = chrono::Utc::now().naive_utc();
    generated_video::ActiveModel {
        project_id: Set(project_id),
        chapter_id: Set(None),
        video_type: Set(video_type.to_string()),
        workflow_mode: Set(Some(workflow_mode.to_string())),
        file_path: Set(Some(file_path.to_string())),
        thumbnail_path: Set(Some(format!("{file_path}.jpg"))),
        prompt: Set("测试视频".to_string()),
        negative_prompt: Set(None),
        model: Set("test-host".to_string()),
        width: Set(1024),
        height: Set(576),
        duration_secs: Set(3.0),
        fps: Set(24),
        file_size: Set(Some(8)),
        source_video_id: Set(None),
        source_image_id: Set(None),
        status: Set("completed".to_string()),
        provider_job_id: Set(None),
        error_message: Set(None),
        metadata_json: Set(None),
        is_favorite: Set(false),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(database)
    .await
    .expect("生成视频应创建成功")
}

fn test_document_for_input_rules() -> CanvasGraphRecordDocument {
    let now = chrono::Utc::now().naive_utc();
    let graph = a4n_core::models::canvas::canvas_graph::Model {
        id: 10,
        project_id: 20,
        name: "测试画布".to_string(),
        graph_kind: "video_studio".to_string(),
        version: 1,
        viewport_json: None,
        created_at: now,
        updated_at: now,
    };
    let graph_id = graph.id;
    let source = |id, video_id| a4n_core::models::canvas::canvas_node::Model {
        id,
        graph_id,
        node_kind: "VideoInputNode".to_string(),
        ref_type: Some("generated_video".to_string()),
        ref_id: Some(video_id),
        config_json: json!({}),
        position_json: json!({"x": 0, "y": 0}),
        state: "ready".to_string(),
        created_at: now,
        updated_at: now,
    };
    let assemble = a4n_core::models::canvas::canvas_node::Model {
        id: 2,
        graph_id,
        node_kind: "AssembleVideoNode".to_string(),
        ref_type: None,
        ref_id: None,
        config_json: json!({"title": "组装预览"}),
        position_json: json!({"x": 1, "y": 0}),
        state: "ready".to_string(),
        created_at: now,
        updated_at: now,
    };
    let assemble_id = assemble.id;
    let edge = |id, source_node_id, order_index| a4n_core::models::canvas::canvas_edge::Model {
        id,
        graph_id,
        source_node_id,
        target_node_id: assemble_id,
        source_port: "video".to_string(),
        target_port: "clip".to_string(),
        role: "assembly_clip".to_string(),
        config_json: json!({"order_index": order_index}),
    };
    CanvasGraphRecordDocument {
        graph,
        nodes: vec![source(1, 91), assemble, source(3, 92)],
        edges: vec![edge(1, 1, 2), edge(2, 3, 1)],
        text_assets: Vec::new(),
    }
}
