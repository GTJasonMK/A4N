use super::*;
use a4n_core::models::{
    ai::{generated_image, generated_video},
    canvas::{canvas_graph, canvas_node},
};
use a4n_domain::CanvasNodeRefKind;
use chrono::Utc;
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, QueryOrder, Set};
use serde_json::json;

#[test]
fn missing_image_cleanup_removes_only_deleted_generated_image_refs() {
    let mut config = json!({
        "generated_image_id": 10,
        "title": "still valid"
    });
    let object = config.as_object_mut().expect("object config");

    remove_config_any_i64_ref(object, IMAGE_REF_CONFIG_KEY, |id| *id == 10);

    assert!(config.get("generated_image_id").is_none());
    assert_eq!(config["title"], json!("still valid"));
}

#[tokio::test]
async fn canvas_reference_cleanup_batches_missing_image_and_video_nodes() {
    let db = a4n_migration::test_support::connect_test_database().await;
    let project = crate::project::create_freeform_video_project_record(
        &db,
        1,
        crate::project::CreateFreeformVideoProjectParams {
            title: "画布缺失引用批量清理".to_string(),
            description: String::new(),
        },
    )
    .await
    .unwrap();
    let now = Utc::now().naive_utc();
    let graph = canvas_graph::ActiveModel {
        project_id: Set(project.id),
        name: Set("清理测试画布".to_string()),
        graph_kind: Set("video_studio".to_string()),
        version: Set(1),
        viewport_json: Set(None),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(&db)
    .await
    .unwrap();
    let images =
        generated_image::Entity::insert_many((0..20).map(|index| generated_image::ActiveModel {
            project_id: Set(project.id),
            image_type: Set("canvas".to_string()),
            file_path: Set(format!("images/{index}.png")),
            prompt: Set(format!("image-{index}")),
            model: Set("test".to_string()),
            width: Set(512),
            height: Set(512),
            file_size: Set(None),
            ai_score: Set(None),
            is_favorite: Set(false),
            chapter_id: Set(None),
            panel_id: Set(None),
            metadata_json: Set(None),
            created_at: Set(now),
            ..Default::default()
        }))
        .exec_with_returning_many(&db)
        .await
        .unwrap();
    let videos =
        generated_video::Entity::insert_many((0..20).map(|index| generated_video::ActiveModel {
            project_id: Set(project.id),
            chapter_id: Set(None),
            video_type: Set(generated_video::VIDEO_TYPE_CANVAS.to_string()),
            workflow_mode: Set(Some("canvas_video".to_string())),
            file_path: Set(None),
            thumbnail_path: Set(None),
            prompt: Set(format!("video-{index}")),
            negative_prompt: Set(None),
            model: Set("test".to_string()),
            width: Set(1280),
            height: Set(720),
            duration_secs: Set(1.0),
            fps: Set(24),
            file_size: Set(None),
            source_video_id: Set(None),
            source_image_id: Set(None),
            status: Set(generated_video::STATUS_COMPLETED.to_string()),
            provider_job_id: Set(None),
            error_message: Set(None),
            timeline_item_id: Set(None),
            metadata_json: Set(None),
            is_favorite: Set(false),
            created_at: Set(now),
            updated_at: Set(now),
            ..Default::default()
        }))
        .exec_with_returning_many(&db)
        .await
        .unwrap();
    let node_rows = images
        .iter()
        .map(|image| canvas_node::ActiveModel {
            graph_id: Set(graph.id),
            node_kind: Set("GenerateImageNode".to_string()),
            ref_type: Set(Some(CanvasNodeRefKind::GeneratedImage.to_string())),
            ref_id: Set(Some(image.id)),
            config_json: Set(json!({"generated_image_id": image.id})),
            position_json: Set(json!({"x": image.id, "y": 0})),
            state: Set("ready".to_string()),
            created_at: Set(now),
            updated_at: Set(now),
            ..Default::default()
        })
        .chain(videos.iter().map(|video| canvas_node::ActiveModel {
            graph_id: Set(graph.id),
            node_kind: Set("GenerateVideoNode".to_string()),
            ref_type: Set(Some(CanvasNodeRefKind::GeneratedVideo.to_string())),
            ref_id: Set(Some(video.id)),
            config_json: Set(if video.id == videos[0].id {
                json!({"generated_video_id": videos[1].id})
            } else {
                json!({"generated_video_id": video.id})
            }),
            position_json: Set(json!({"x": video.id, "y": 1})),
            state: Set("ready".to_string()),
            created_at: Set(now),
            updated_at: Set(now),
            ..Default::default()
        }))
        .collect::<Vec<_>>();
    canvas_node::Entity::insert_many(node_rows)
        .exec_without_returning(&db)
        .await
        .unwrap();

    mark_canvas_generated_image_refs_missing(
        &db,
        &images.iter().map(|row| row.id).collect::<Vec<_>>(),
    )
    .await
    .unwrap();
    mark_canvas_generated_video_refs_missing_for_projects_and_videos(
        &db,
        &[project.id],
        &videos.iter().map(|row| row.id).collect::<Vec<_>>(),
    )
    .await
    .unwrap();

    let nodes = canvas_node::Entity::find()
        .filter(canvas_node::Column::GraphId.eq(graph.id))
        .order_by_asc(canvas_node::Column::Id)
        .all(&db)
        .await
        .unwrap();
    assert_eq!(nodes.len(), 40);
    assert!(nodes.iter().all(|node| node.state == "missing"));
    assert!(nodes.iter().all(|node| node.ref_type.is_none()));
    assert!(nodes.iter().all(|node| node.ref_id.is_none()));
    assert!(nodes.iter().all(|node| {
        node.config_json.get("generated_image_id").is_none()
            && node.config_json.get("generated_video_id").is_none()
    }));
    let canonical_missing_id = json!(videos[0].id);
    assert!(
        nodes
            .iter()
            .any(|node| { node.config_json.get("missing_ref_id") == Some(&canonical_missing_id) })
    );
}
