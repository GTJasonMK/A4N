use std::sync::Arc;

use a4n_core::models::{
    ai::{generated_image, generated_video},
    canvas::canvas_node,
    generation::{workflow_checkpoint, workflow_run},
    media::media_asset,
};
use a4n_db::{
    DatabaseConnection,
    canvas::{CanvasEdgeRecordInput, CanvasGraphRecordInput, CanvasNodeRecordInput},
    project::{CreateFreeformVideoProjectParams, create_freeform_video_project_record},
};
use a4n_domain::image::IMAGE_TYPE_CANVAS;
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, Set};
use serde_json::json;

use super::{
    contracts::CanvasProbeNodeExecutionInput,
    execution::{canvas_probe_observation_patch, probe_node_data_type},
};
use crate::{
    OrchestratorContext, canvas::CanvasOutputDataType, test_support::TestOrchestratorBuilder,
};

#[test]
fn canvas_probe_node_types_are_canonical() {
    assert_eq!(
        probe_node_data_type("TextProbeNode").unwrap(),
        CanvasOutputDataType::Text
    );
    assert_eq!(
        probe_node_data_type("ImageProbeNode").unwrap(),
        CanvasOutputDataType::Image
    );
    assert_eq!(
        probe_node_data_type("VideoProbeNode").unwrap(),
        CanvasOutputDataType::Video
    );
    assert_eq!(
        probe_node_data_type("AudioProbeNode").unwrap(),
        CanvasOutputDataType::Audio
    );
    assert!(probe_node_data_type("UnknownProbeNode").is_err());
}

#[test]
fn canvas_probe_observation_patch_projects_only_canonical_fields() {
    let patch = canvas_probe_observation_patch(
        &json!({
            "data_type": "video",
            "generated_video_id": 91,
            "file_path": "video.mp4",
            "thumbnail_path": "video.jpg",
            "duration_secs": 3.5,
            "ignored": "value",
        }),
        &[1, 2],
    );

    assert_eq!(patch["observed_data_type"], json!("video"));
    assert_eq!(patch["generated_video_id"], json!(91));
    assert_eq!(patch["source_node_ids"], json!([1, 2]));
    assert!(patch.get("ignored").is_none());
}

#[tokio::test]
async fn canvas_probe_application_persists_text_image_video_and_audio_observations() {
    let (database, context) = setup_context().await;
    let project = create_freeform_video_project_record(
        database.as_ref(),
        1,
        CreateFreeformVideoProjectParams {
            title: "Canvas Probe application 测试".to_string(),
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
                name: Some("探针节点测试画布".to_string()),
                graph_kind: Some("video_studio".to_string()),
                viewport_json: None,
                nodes: vec![
                    record_node(
                        "text-source",
                        "TextInputNode",
                        json!({"content": "观察到的字幕"}),
                    ),
                    record_node("text-probe", "TextProbeNode", json!({"label": "文本探针"})),
                    referenced_record_node(
                        "image-source",
                        "ImageInputNode",
                        "generated_image",
                        image.id,
                    ),
                    record_node("image-probe", "ImageProbeNode", json!({})),
                    referenced_record_node(
                        "video-source",
                        "VideoInputNode",
                        "generated_video",
                        video.id,
                    ),
                    record_node("video-probe", "VideoProbeNode", json!({})),
                    referenced_record_node(
                        "audio-source",
                        "AudioInputNode",
                        "media_asset",
                        audio.id,
                    ),
                    record_node("audio-probe", "AudioProbeNode", json!({})),
                ],
                edges: vec![
                    record_edge("text-source", "text-probe", "text", "text"),
                    record_edge("image-source", "image-probe", "image", "image"),
                    record_edge("video-source", "video-probe", "video", "video"),
                    record_edge("audio-source", "audio-probe", "audio", "audio"),
                ],
            },
            chrono::Utc::now().naive_utc(),
        )
        .await
        .expect("测试画布应保存成功");

    for node_kind in [
        "TextProbeNode",
        "ImageProbeNode",
        "VideoProbeNode",
        "AudioProbeNode",
    ] {
        let node = document
            .nodes
            .iter()
            .find(|node| node.node_kind == node_kind)
            .expect("探针节点应存在");
        let summary = context
            .run_canvas_probe_node_validated(CanvasProbeNodeExecutionInput {
                project_id: project.id,
                graph_id: document.graph.id,
                node,
                document: &document,
                shared_run: None,
                mode: "single_node",
            })
            .await
            .expect("探针 application 应返回持久化终态");
        let persisted_node = canvas_node::Entity::find_by_id(node.id)
            .one(database.as_ref())
            .await
            .expect("应可查询探针节点")
            .expect("探针节点应存在");
        let run = workflow_run::Entity::find_by_id(summary.workflow_run_id)
            .one(database.as_ref())
            .await
            .expect("应可查询探针 workflow")
            .expect("探针 workflow 应存在");
        let checkpoint = workflow_checkpoint::Entity::find()
            .filter(workflow_checkpoint::Column::WorkflowRunId.eq(summary.workflow_run_id))
            .filter(workflow_checkpoint::Column::StepKey.eq(format!("canvas_node:{}", node.id)))
            .one(database.as_ref())
            .await
            .expect("应可查询探针 checkpoint")
            .expect("探针 checkpoint 应存在");

        assert_eq!(
            summary.status, "completed",
            "{node_kind} 探针失败: {:?}",
            summary.error_message
        );
        assert_eq!(run.status, "completed");
        assert_eq!(persisted_node.state, "completed");
        assert_eq!(persisted_node.ref_type, None);
        assert_eq!(persisted_node.ref_id, None);
        assert_eq!(checkpoint.status, "completed");
        assert_eq!(
            checkpoint
                .artifacts_json
                .as_ref()
                .and_then(|value| value.get("data_type"))
                .and_then(serde_json::Value::as_str),
            persisted_node
                .config_json
                .get("observed_data_type")
                .and_then(serde_json::Value::as_str)
        );

        match node_kind {
            "TextProbeNode" => {
                assert_eq!(
                    persisted_node.config_json["observed_text"],
                    json!("观察到的字幕")
                );
                assert_eq!(persisted_node.config_json["label"], json!("文本探针"));
            }
            "ImageProbeNode" => {
                assert_eq!(
                    persisted_node.config_json["generated_image_id"],
                    json!(image.id)
                );
                assert_eq!(
                    persisted_node.config_json["file_path"],
                    json!("projects/output/probe-image.png")
                );
            }
            "VideoProbeNode" => {
                assert_eq!(
                    persisted_node.config_json["generated_video_id"],
                    json!(video.id)
                );
                assert_eq!(
                    persisted_node.config_json["file_path"],
                    json!("projects/output/probe-video.mp4")
                );
                assert_eq!(
                    persisted_node.config_json["thumbnail_path"],
                    json!("projects/output/probe-video.jpg")
                );
            }
            "AudioProbeNode" => {
                assert_eq!(
                    persisted_node.config_json["media_asset_id"],
                    json!(audio.id)
                );
                assert_eq!(persisted_node.config_json["duration_secs"], json!(2.5));
            }
            _ => unreachable!("测试仅包含 canonical ProbeNode"),
        }
        assert_eq!(
            persisted_node.config_json["source_node_ids"]
                .as_array()
                .map(Vec::len),
            Some(1)
        );
    }
}

#[tokio::test]
async fn canvas_probe_application_marks_workflow_and_checkpoint_failed_when_artifact_is_missing() {
    let (database, context) = setup_context().await;
    let project = create_freeform_video_project_record(
        database.as_ref(),
        1,
        CreateFreeformVideoProjectParams {
            title: "Canvas Probe failure 测试".to_string(),
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
                name: Some("探针失败测试画布".to_string()),
                graph_kind: Some("video_studio".to_string()),
                viewport_json: None,
                nodes: vec![
                    referenced_record_node(
                        "image-source",
                        "ImageInputNode",
                        "generated_image",
                        image.id,
                    ),
                    record_node("image-probe", "ImageProbeNode", json!({})),
                ],
                edges: vec![record_edge("image-source", "image-probe", "image", "image")],
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
        .find(|node| node.node_kind == "ImageProbeNode")
        .expect("图片探针节点应存在");

    let summary = context
        .run_canvas_probe_node_validated(CanvasProbeNodeExecutionInput {
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
    let checkpoint = workflow_checkpoint::Entity::find()
        .filter(workflow_checkpoint::Column::WorkflowRunId.eq(summary.workflow_run_id))
        .filter(workflow_checkpoint::Column::StepKey.eq(format!("canvas_node:{}", node.id)))
        .one(database.as_ref())
        .await
        .expect("应可查询失败 checkpoint")
        .expect("失败 checkpoint 应存在");

    assert_eq!(summary.status, "failed");
    assert!(
        summary
            .error_message
            .as_deref()
            .is_some_and(|message| message.contains("图片探针引用的图片不存在"))
    );
    assert_eq!(run.status, "failed");
    assert_eq!(checkpoint.status, "failed");
    assert!(
        checkpoint
            .error_message
            .as_deref()
            .is_some_and(|message| message.contains("图片探针引用的图片不存在"))
    );
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
        file_path: Set("projects/output/probe-image.png".to_string()),
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
        file_path: Set(Some("projects/output/probe-video.mp4".to_string())),
        thumbnail_path: Set(Some("projects/output/probe-video.jpg".to_string())),
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
        file_path: Set(Some("projects/output/probe-audio.wav".to_string())),
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

fn record_node(
    client_id: &str,
    node_kind: &str,
    config_json: serde_json::Value,
) -> CanvasNodeRecordInput {
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
    CanvasEdgeRecordInput {
        source_client_id: source_client_id.to_string(),
        target_client_id: target_client_id.to_string(),
        source_port: source_port.to_string(),
        target_port: target_port.to_string(),
        role: "context".to_string(),
        config_json: None,
    }
}
