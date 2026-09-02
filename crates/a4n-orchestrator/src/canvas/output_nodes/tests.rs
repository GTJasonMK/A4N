use std::sync::Arc;

use a4n_core::models::{
    ai::{generated_image, generated_video},
    canvas::{canvas_edge, canvas_graph, canvas_node, canvas_text_asset},
    generation::{workflow_checkpoint, workflow_run},
    media::media_asset,
};
use a4n_db::{
    DatabaseConnection,
    canvas::{
        CanvasEdgeRecordInput, CanvasGraphRecordDocument, CanvasGraphRecordInput,
        CanvasNodeRecordInput,
    },
    project::{CreateFreeformVideoProjectParams, create_freeform_video_project_record},
};
use a4n_domain::image::IMAGE_TYPE_CANVAS;
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, Set};
use serde_json::{Value, json};

use crate::{OrchestratorContext, test_support::TestOrchestratorBuilder};

use super::{
    contracts::{CanvasOutputDataType, CanvasOutputNodeExecutionInput},
    input::{collect_canvas_output_value, validate_canvas_output_value},
};

#[test]
fn canvas_output_input_reads_connected_refs_and_rejects_hidden_config() {
    let mut image_source = test_node(1, "ImageInputNode", json!({"generated_image_id": 999}));
    image_source.ref_type = Some("generated_image".to_string());
    image_source.ref_id = Some(91);
    let output_node = test_node(2, "ImageOutputNode", json!({}));
    let document = test_document(
        vec![image_source, output_node.clone()],
        vec![test_edge(1, 2, "image", "image")],
    );

    let output = collect_canvas_output_value(
        &output_node,
        &document,
        None,
        CanvasOutputDataType::Image,
        "ImageOutputNode",
        false,
    )
    .expect("图片输出输入应解析成功");

    assert_eq!(output.generated_image_id, Some(91));
    assert_eq!(output.source_node_ids, vec![1]);

    let hidden_source = test_node(3, "ImageInputNode", json!({"generated_image_id": 92}));
    let hidden_output = test_node(4, "ImageOutputNode", json!({}));
    let hidden_document = test_document(
        vec![hidden_source, hidden_output.clone()],
        vec![test_edge(3, 4, "image", "image")],
    );
    let hidden = collect_canvas_output_value(
        &hidden_output,
        &hidden_document,
        None,
        CanvasOutputDataType::Image,
        "ImageOutputNode",
        false,
    )
    .expect("隐藏 config 不应作为输出值");
    let error =
        validate_canvas_output_value(&hidden, CanvasOutputDataType::Image, "ImageOutputNode")
            .expect_err("隐藏 config 必须被拒绝");
    assert!(error.contains("缺少图片输入"));
}

#[test]
fn canvas_output_input_requires_current_workflow_checkpoint_for_operation_source() {
    let source = test_node(1, "ImageProbeNode", json!({"media_asset_id": 91}));
    let output_node = test_node(2, "ImageOutputNode", json!({}));
    let document = test_document(
        vec![source, output_node.clone()],
        vec![test_edge(1, 2, "image", "image")],
    );

    let error = collect_canvas_output_value(
        &output_node,
        &document,
        None,
        CanvasOutputDataType::Image,
        "ImageOutputNode",
        false,
    )
    .expect_err("未运行的操作节点不能回落到 config");
    assert!(error.contains("当前 workflow checkpoint"));

    let checkpoint = test_checkpoint(31, 1, json!({"media_asset_id": 91}));
    let output = collect_canvas_output_value(
        &output_node,
        &document,
        Some(&[checkpoint]),
        CanvasOutputDataType::Image,
        "ImageOutputNode",
        false,
    )
    .expect("当前 workflow checkpoint 应可作为输出输入");
    assert_eq!(output.media_asset_id, Some(91));
}

#[test]
fn canvas_output_hidden_video_and_audio_config_is_rejected() {
    let video_source = test_node(
        1,
        "VideoInputNode",
        json!({"generated_video_id": 91, "video_id": 92}),
    );
    let video_output = test_node(2, "VideoOutputNode", json!({}));
    let video_document = test_document(
        vec![video_source, video_output.clone()],
        vec![test_edge(1, 2, "video", "video")],
    );
    let output = collect_canvas_output_value(
        &video_output,
        &video_document,
        None,
        CanvasOutputDataType::Video,
        "VideoOutputNode",
        true,
    )
    .expect("隐藏视频 config 不应成为输出值");
    assert_eq!(output.generated_video_id, None);

    let audio_source = test_node(3, "AudioInputNode", json!({"media_asset_id": 93}));
    let audio_output = test_node(4, "AudioOutputNode", json!({}));
    let audio_document = test_document(
        vec![audio_source, audio_output.clone()],
        vec![test_edge(3, 4, "audio", "audio")],
    );
    let output = collect_canvas_output_value(
        &audio_output,
        &audio_document,
        None,
        CanvasOutputDataType::Audio,
        "AudioOutputNode",
        false,
    )
    .expect("隐藏音频 config 不应成为输出值");
    let error =
        validate_canvas_output_value(&output, CanvasOutputDataType::Audio, "AudioOutputNode")
            .expect_err("隐藏 media_asset_id 必须被拒绝");
    assert!(error.contains("缺少音频输入"));
}

#[test]
fn canvas_video_output_only_consumes_video_target_port() {
    let mut source = test_node(1, "VideoInputNode", json!({}));
    source.ref_type = Some("generated_video".to_string());
    source.ref_id = Some(91);
    let output_node = test_node(2, "VideoOutputNode", json!({}));
    let wrong_port = test_document(
        vec![source.clone(), output_node.clone()],
        vec![test_edge(1, 2, "video", "context")],
    );
    let output = collect_canvas_output_value(
        &output_node,
        &wrong_port,
        None,
        CanvasOutputDataType::Video,
        "VideoOutputNode",
        true,
    )
    .expect("非 video 端口应被忽略");
    assert_eq!(output.generated_video_id, None);

    let video_port = test_document(
        vec![source, output_node.clone()],
        vec![test_edge(1, 2, "video", "video")],
    );
    let output = collect_canvas_output_value(
        &output_node,
        &video_port,
        None,
        CanvasOutputDataType::Video,
        "VideoOutputNode",
        true,
    )
    .expect("video 端口应被读取");
    assert_eq!(output.generated_video_id, Some(91));
}

#[tokio::test]
async fn canvas_output_application_persists_text_image_audio_and_video_artifacts() {
    let (database, context) = setup_context().await;
    let project = create_freeform_video_project_record(
        database.as_ref(),
        1,
        CreateFreeformVideoProjectParams {
            title: "Canvas Output application 测试".to_string(),
            description: String::new(),
        },
    )
    .await
    .expect("测试项目应创建成功");
    let image = insert_generated_image(database.as_ref(), project.id).await;
    let video = insert_generated_video(database.as_ref(), project.id).await;
    let audio = insert_audio_asset(database.as_ref(), project.id).await;
    let document = context
        .save_canvas_graph_record(
            project.id,
            CanvasGraphRecordInput {
                id: None,
                expected_version: None,
                name: Some("输出节点测试画布".to_string()),
                graph_kind: Some("video_studio".to_string()),
                viewport_json: None,
                nodes: vec![
                    record_node(
                        "text-source",
                        "TextInputNode",
                        json!({"content": "最终字幕"}),
                    ),
                    record_node(
                        "text-output",
                        "TextOutputNode",
                        json!({"title": "字幕输出"}),
                    ),
                    referenced_record_node(
                        "image-source",
                        "ImageInputNode",
                        "generated_image",
                        image.id,
                    ),
                    record_node("image-output", "ImageOutputNode", json!({})),
                    referenced_record_node(
                        "audio-source",
                        "AudioInputNode",
                        "media_asset",
                        audio.id,
                    ),
                    record_node("audio-output", "AudioOutputNode", json!({})),
                    referenced_record_node(
                        "video-source",
                        "VideoInputNode",
                        "generated_video",
                        video.id,
                    ),
                    record_node("video-output", "VideoOutputNode", json!({})),
                ],
                edges: vec![
                    record_edge("text-source", "text-output", "text", "text"),
                    record_edge("image-source", "image-output", "image", "image"),
                    record_edge("audio-source", "audio-output", "audio", "audio"),
                    record_edge_with_role(
                        "video-source",
                        "video-output",
                        "video",
                        "video",
                        "final_video",
                    ),
                ],
            },
            chrono::Utc::now().naive_utc(),
        )
        .await
        .expect("测试画布应保存成功");

    let expected_refs = [
        ("TextOutputNode", "canvas_text_asset"),
        ("ImageOutputNode", "generated_image"),
        ("AudioOutputNode", "media_asset"),
        ("VideoOutputNode", "generated_video"),
    ];
    for (node_kind, expected_ref_type) in expected_refs {
        let node = document
            .nodes
            .iter()
            .find(|node| node.node_kind == node_kind)
            .expect("输出节点应存在");
        let summary = context
            .run_canvas_output_node_validated(CanvasOutputNodeExecutionInput {
                project_id: project.id,
                graph_id: document.graph.id,
                node,
                document: &document,
                shared_run: None,
                mode: "single_node",
            })
            .await
            .expect("输出节点 application 应成功");
        let persisted_node = canvas_node::Entity::find_by_id(node.id)
            .one(database.as_ref())
            .await
            .expect("应可查询输出节点")
            .expect("输出节点应存在");
        let run = workflow_run::Entity::find_by_id(summary.workflow_run_id)
            .one(database.as_ref())
            .await
            .expect("应可查询输出 workflow")
            .expect("输出 workflow 应存在");
        let checkpoint = workflow_checkpoint::Entity::find()
            .filter(workflow_checkpoint::Column::WorkflowRunId.eq(summary.workflow_run_id))
            .filter(workflow_checkpoint::Column::StepKey.eq(format!("canvas_node:{}", node.id)))
            .one(database.as_ref())
            .await
            .expect("应可查询输出 checkpoint")
            .expect("输出 checkpoint 应存在");

        assert_eq!(
            summary.status, "completed",
            "{node_kind} 输出失败: {:?}",
            summary.error_message
        );
        assert_eq!(run.status, "completed");
        assert_eq!(persisted_node.ref_type.as_deref(), Some(expected_ref_type));
        assert_eq!(persisted_node.state, "completed");
        assert!(persisted_node.ref_id.is_some());
        assert_eq!(checkpoint.status, "completed");
    }

    let text_assets = canvas_text_asset::Entity::find()
        .all(database.as_ref())
        .await
        .expect("应可查询文本输出资产");
    assert!(text_assets.iter().any(|asset| {
        asset.project_id == project.id && asset.title == "字幕输出" && asset.content == "最终字幕"
    }));
}

#[tokio::test]
async fn canvas_output_application_marks_workflow_failed_when_artifact_is_missing() {
    let (database, context) = setup_context().await;
    let project = create_freeform_video_project_record(
        database.as_ref(),
        1,
        CreateFreeformVideoProjectParams {
            title: "Canvas Output failure 测试".to_string(),
            description: String::new(),
        },
    )
    .await
    .expect("测试项目应创建成功");
    let image = insert_generated_image(database.as_ref(), project.id).await;
    let mut document = context
        .save_canvas_graph_record(
            project.id,
            CanvasGraphRecordInput {
                id: None,
                expected_version: None,
                name: Some("输出失败测试画布".to_string()),
                graph_kind: Some("video_studio".to_string()),
                viewport_json: None,
                nodes: vec![
                    referenced_record_node(
                        "image-source",
                        "ImageInputNode",
                        "generated_image",
                        image.id,
                    ),
                    record_node("image-output", "ImageOutputNode", json!({})),
                ],
                edges: vec![record_edge(
                    "image-source",
                    "image-output",
                    "image",
                    "image",
                )],
            },
            chrono::Utc::now().naive_utc(),
        )
        .await
        .expect("测试画布应保存成功");
    document
        .nodes
        .iter_mut()
        .find(|node| node.node_kind == "ImageInputNode")
        .expect("图片源节点应存在")
        .ref_id = Some(i64::MAX);
    let node = document
        .nodes
        .iter()
        .find(|node| node.node_kind == "ImageOutputNode")
        .expect("图片输出节点应存在");

    let summary = context
        .run_canvas_output_node_validated(CanvasOutputNodeExecutionInput {
            project_id: project.id,
            graph_id: document.graph.id,
            node,
            document: &document,
            shared_run: None,
            mode: "single_node",
        })
        .await
        .expect("产物缺失应返回持久化后的 failed summary");
    let run = workflow_run::Entity::find_by_id(summary.workflow_run_id)
        .one(database.as_ref())
        .await
        .expect("应可查询失败 workflow")
        .expect("失败 workflow 应存在");

    assert_eq!(summary.status, "failed");
    assert!(
        summary
            .error_message
            .as_deref()
            .is_some_and(|message| message.contains("图片输出不存在"))
    );
    assert_eq!(run.status, "failed");
}

async fn setup_context() -> (Arc<DatabaseConnection>, OrchestratorContext) {
    let database = Arc::new(a4n_migration::test_support::connect_test_database().await);
    let context = TestOrchestratorBuilder::new(Arc::clone(&database))
        .build()
        .await;
    (database, context)
}

async fn insert_generated_image(
    database: &DatabaseConnection,
    project_id: i64,
) -> generated_image::Model {
    generated_image::ActiveModel {
        project_id: Set(project_id),
        image_type: Set(IMAGE_TYPE_CANVAS.to_string()),
        file_path: Set("projects/output/image.png".to_string()),
        prompt: Set("测试图片".to_string()),
        model: Set("test-model".to_string()),
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
) -> generated_video::Model {
    let now = chrono::Utc::now().naive_utc();
    generated_video::ActiveModel {
        project_id: Set(project_id),
        chapter_id: Set(None),
        video_type: Set(generated_video::VIDEO_TYPE_CANVAS.to_string()),
        workflow_mode: Set(None),
        file_path: Set(Some("projects/output/video.mp4".to_string())),
        thumbnail_path: Set(Some("projects/output/video.jpg".to_string())),
        prompt: Set("测试视频".to_string()),
        negative_prompt: Set(None),
        model: Set("test-model".to_string()),
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

async fn insert_audio_asset(database: &DatabaseConnection, project_id: i64) -> media_asset::Model {
    let now = chrono::Utc::now().naive_utc();
    media_asset::ActiveModel {
        project_id: Set(project_id),
        root_project_id: Set(project_id),
        source_snapshot_id: Set(None),
        source_chapter_id: Set(None),
        source_snapshot_chapter_id: Set(None),
        media_kind: Set("audio".to_string()),
        origin: Set("canvas_input".to_string()),
        file_path: Set(Some("projects/output/audio.wav".to_string())),
        thumbnail_path: Set(None),
        width: Set(None),
        height: Set(None),
        duration_secs: Set(Some(2.5)),
        fps: Set(None),
        file_size: Set(Some(16)),
        checksum: Set(None),
        status: Set("completed".to_string()),
        error_message: Set(None),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(database)
    .await
    .expect("音频媒体资产应创建成功")
}

fn record_node(client_id: &str, node_kind: &str, config_json: Value) -> CanvasNodeRecordInput {
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

fn referenced_record_node(
    client_id: &str,
    node_kind: &str,
    ref_type: &str,
    ref_id: i64,
) -> CanvasNodeRecordInput {
    CanvasNodeRecordInput {
        ref_type: Some(ref_type.to_string()),
        ref_id: Some(ref_id),
        ..record_node(client_id, node_kind, json!({}))
    }
}

fn record_edge(
    source_client_id: &str,
    target_client_id: &str,
    source_port: &str,
    target_port: &str,
) -> CanvasEdgeRecordInput {
    record_edge_with_role(
        source_client_id,
        target_client_id,
        source_port,
        target_port,
        "context",
    )
}

fn record_edge_with_role(
    source_client_id: &str,
    target_client_id: &str,
    source_port: &str,
    target_port: &str,
    role: &str,
) -> CanvasEdgeRecordInput {
    CanvasEdgeRecordInput {
        source_client_id: source_client_id.to_string(),
        target_client_id: target_client_id.to_string(),
        source_port: source_port.to_string(),
        target_port: target_port.to_string(),
        role: role.to_string(),
        config_json: None,
    }
}

fn test_document(
    nodes: Vec<canvas_node::Model>,
    edges: Vec<canvas_edge::Model>,
) -> CanvasGraphRecordDocument {
    let now = chrono::Utc::now().naive_utc();
    CanvasGraphRecordDocument {
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

fn test_node(id: i64, kind: &str, config_json: serde_json::Value) -> canvas_node::Model {
    let now = chrono::Utc::now().naive_utc();
    canvas_node::Model {
        id,
        graph_id: 10,
        node_kind: kind.to_string(),
        ref_type: None,
        ref_id: None,
        config_json,
        position_json: json!({"x": 0, "y": 0}),
        state: "ready".to_string(),
        created_at: now,
        updated_at: now,
    }
}

fn test_edge(
    source_node_id: i64,
    target_node_id: i64,
    source_port: &str,
    target_port: &str,
) -> canvas_edge::Model {
    canvas_edge::Model {
        id: source_node_id * 100 + target_node_id,
        graph_id: 10,
        source_node_id,
        target_node_id,
        source_port: source_port.to_string(),
        target_port: target_port.to_string(),
        role: "context".to_string(),
        config_json: json!({}),
    }
}

fn test_checkpoint(
    workflow_run_id: i64,
    node_id: i64,
    artifacts_json: serde_json::Value,
) -> workflow_checkpoint::Model {
    let now = chrono::Utc::now().naive_utc();
    workflow_checkpoint::Model {
        id: node_id,
        workflow_run_id,
        step_key: format!("canvas_node:{node_id}"),
        step_order: 0,
        status: "completed".to_string(),
        input_fingerprint_sha256: None,
        input_fingerprint_json: None,
        checkpoint_json: None,
        artifacts_json: Some(artifacts_json),
        generation_run_ids_json: None,
        error_message: None,
        completed_at: Some(now),
        created_at: now,
        updated_at: now,
    }
}
